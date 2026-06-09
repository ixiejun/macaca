//! [`WorkspaceKnowledgeDigestCapability`] — Adapter implementing
//! [`macaca_sdk::context::KnowledgeDigestCapability`] on top of the Memory Service client.
//!
//! ## Flow
//! 1. Derive the same **retrieval query** string the active-recall provider would use
//!    (latest user turn text via [`macaca_sdk::context::ContextAssembleInput::last_user_message_text`]).
//! 2. Pull a bounded set of [`macaca_proto::MemoryEntry`] rows through the Memory Service facade.
//! 3. Map rows into redacted [`macaca_sdk::context::KnowledgeDigestItem`] entries, copying
//!    only **opaque** evidence ids (`ClaimEvidence::source_id`) so digest-vs-raw suppression can align with
//!    fenced recall rows without leaking full memory payloads into structured reports.
//! 4. Apply [`macaca_sdk::context::filter_digest_items_by_tombstones`] when an optional [`macaca_sdk::memory::TombstoneIndex`] is wired
//!    (shared registry updated by [`crate::context_memory_tools::WorkspaceMemoryForgetTool`] or a governance facade snapshot).
//!
//! ## Failure semantics
//! Any fatal error returns `Ok(vec![])` — the composer treats “no digest” as a benign outcome (fail-open).
//! Tombstone snapshots that fail **only** skip filtering and log — digest rows still compile (explicit fail-open boundary).

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_sdk::context::{
    filter_digest_items_by_tombstones, ContextAssembleInput, ContextSourceProvenance,
    KnowledgeDigestCapability, KnowledgeDigestItem, PrivacyTier,
};
use macaca_sdk::memory::{MemoryPolicyHints, MemoryPrefetchCommand, MemoryScope, TombstoneIndex};
use macaca_proto::{MacacaResult, TraceContext};

/// Workspace memory → knowledge digest port adapter.
pub struct WorkspaceKnowledgeDigestCapability {
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
    search_limit: usize,
    /// Optional tombstone ledger — when present, evidence pointers referencing deleted ids are stripped.
    tombstones: Option<Arc<dyn TombstoneIndex>>,
}

impl WorkspaceKnowledgeDigestCapability {
    /// `search_limit` should track [`macaca_proto::config::KnowledgeDigestContextConfig::max_compiler_rows`].
    ///
    /// `tombstones` integrates [`SharedTombstoneRegistry`] / [`GovernanceFacadeTombstones`] implementations from
    /// [`macaca_sdk::memory`] without coupling this crate to a single storage backend.
    #[must_use]
    pub fn new(
        memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
        scope: MemoryScope,
        search_limit: usize,
        tombstones: Option<Arc<dyn TombstoneIndex>>,
    ) -> Self {
        Self {
            memory_client,
            scope,
            search_limit: search_limit.max(1),
            tombstones,
        }
    }

    fn compile_scope(&self, _input: &ContextAssembleInput) -> MemoryScope {
        self.scope.clone()
    }
}

#[async_trait]
impl KnowledgeDigestCapability for WorkspaceKnowledgeDigestCapability {
    fn capability_id(&self) -> &str {
        "workspace-knowledge-digest"
    }

    async fn digest_for_request(
        &self,
        input: &ContextAssembleInput,
    ) -> MacacaResult<Vec<KnowledgeDigestItem>> {
        let Some(query) = input.last_user_message_text() else {
            return Ok(Vec::new());
        };

        let scope = self.compile_scope(input);
        let mut trace = TraceContext::new(uuid::Uuid::new_v4().to_string());
        trace.session_id = scope.identity.session_id.clone();
        trace.agent = scope.identity.agent_name.clone();
        let entries = self
            .memory_client
            .prefetch(MemoryPrefetchCommand {
                scope: scope.clone(),
                trace,
                query: query.to_owned(),
                limit: self.search_limit,
                policy: MemoryPolicyHints::default(),
            })
            .await?
            .entries;

        let mut out = Vec::new();
        for entry in entries {
            let memory_id = entry.id.0.to_string();
            out.push(KnowledgeDigestItem {
                claim_id: format!("memory-digest-{memory_id}"),
                label: "memory-digest".into(),
                statement: entry.content,
                provenance: ContextSourceProvenance {
                    provider_id: self.capability_id().into(),
                    source_id: memory_id.clone(),
                    evidence: vec![memory_id.clone()],
                },
                evidence_memory_ids: vec![memory_id],
                confidence: 0.8,
                freshness: 0.8,
                privacy_tier: PrivacyTier::Workspace,
                request_only: true,
                redacted: true,
            });
        }

        if let Some(ref index) = self.tombstones {
            match index.tombstoned_memory_id_strings().await {
                Ok(ids) => {
                    let set: HashSet<String> = ids.into_iter().collect();
                    out = filter_digest_items_by_tombstones(out, &set);
                }
                Err(err) => tracing::warn!(
                    error = %err,
                    capability = %self.capability_id(),
                    "tombstone snapshot failed — compiled digest left unfiltered"
                ),
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    use macaca_sdk::memory::{
        FileMemory, InMemoryVectorStore, MemoryForgetCommand, MemoryPrefetchCommand,
        MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
        MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand,
        MemoryStatusReport, MockEmbedding, RememberText, SessionMemory, SharedTombstoneRegistry,
        TestMemoryManager,
    };
    use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, MemoryEntry, MemoryId};

    fn temp_memory_dir() -> PathBuf {
        let unique = format!(
            "macaca-web-knowledge-digest-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_manager(dir: &PathBuf) -> Arc<TestMemoryManager> {
        Arc::new(TestMemoryManager::new(
            SessionMemory::new(Duration::from_secs(60)),
            FileMemory::new(dir.clone()),
            Some(InMemoryVectorStore::new()),
            Some(MockEmbedding::default()),
        ))
    }

    fn assemble_input(query: &str) -> ContextAssembleInput {
        ContextAssembleInput::legacy(
            "coordinator",
            "test-model",
            vec![LlmMessage::user(query)],
            LlmOptions::default(),
        )
    }

    struct StaticMemoryClient {
        entries: Vec<MemoryEntry>,
    }

    #[async_trait]
    impl macaca_sdk::SystemMemoryClient for StaticMemoryClient {
        async fn remember(
            &self,
            _command: MemoryRememberCommand,
        ) -> MacacaResult<MemoryRememberResult> {
            Ok(MemoryRememberResult {
                id: MemoryId::new(),
                stored_at: chrono::Utc::now(),
            })
        }

        async fn recall(&self, _command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
            Ok(MemoryRecallResult::new(self.entries.clone()))
        }

        async fn prefetch(
            &self,
            _command: MemoryPrefetchCommand,
        ) -> MacacaResult<MemoryRecallResult> {
            Ok(MemoryRecallResult::new(self.entries.clone()))
        }

        async fn get(
            &self,
            command: macaca_sdk::memory::MemoryGetCommand,
        ) -> MacacaResult<macaca_sdk::memory::MemoryGetResult> {
            Ok(macaca_sdk::memory::MemoryGetResult::new(
                self.entries
                    .iter()
                    .find(|entry| entry.id == command.id)
                    .cloned(),
            ))
        }

        async fn forget(&self, _command: MemoryForgetCommand) -> MacacaResult<()> {
            Ok(())
        }

        async fn status(&self, _command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
            Ok(MemoryStatusReport::healthy(
                "test-memory-client",
                macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
            ))
        }

        async fn snapshot(
            &self,
            _command: MemoryServiceSnapshotCommand,
        ) -> MacacaResult<MemoryServiceSnapshot> {
            Ok(MemoryServiceSnapshot::new(
                "test-memory-client",
                true,
                macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
                None,
            ))
        }
    }

    async fn client_from_manager(
        manager: &Arc<TestMemoryManager>,
        query: &str,
    ) -> Arc<dyn macaca_sdk::SystemMemoryClient> {
        let entries = manager
            .recall(macaca_sdk::memory::RecallQuery::new(query, 8))
            .await
            .unwrap()
            .entries;
        Arc::new(StaticMemoryClient { entries })
    }

    #[tokio::test]
    async fn digest_for_request_compiles_workspace_memory_into_digest_rows() {
        let dir = temp_memory_dir();
        let manager = make_manager(&dir);
        manager
            .remember_text(RememberText::new(
                "Rust service owners use workspace memory to retain deployment knowledge.",
            ))
            .await
            .unwrap();

        let memory_client = client_from_manager(&manager, "deployment knowledge").await;
        let scope = macaca_sdk::memory::MemoryScope::project_shared(ApplicationId::new(), "workspace");
        let capability = WorkspaceKnowledgeDigestCapability::new(memory_client, scope, 8, None);

        let rows = capability
            .digest_for_request(&assemble_input("deployment knowledge"))
            .await
            .unwrap();

        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.redacted));
        assert!(rows.iter().all(|row| !row.evidence_memory_ids.is_empty()));
    }

    #[tokio::test]
    async fn digest_for_request_filters_tombstoned_evidence_ids() {
        let dir = temp_memory_dir();
        let manager = make_manager(&dir);
        let kept_id = manager
            .remember_text(RememberText::new(
                "Workspace runbooks describe how to rotate production credentials safely.",
            ))
            .await
            .unwrap();
        let deleted_id = manager
            .remember_text(RememberText::new(
                "An older credential note was deleted and must never reappear in digest evidence.",
            ))
            .await
            .unwrap();

        let tombstones = Arc::new(SharedTombstoneRegistry::new());
        tombstones.record(deleted_id).await;

        let memory_client = client_from_manager(&manager, "credential note").await;
        let scope = macaca_sdk::memory::MemoryScope::project_shared(ApplicationId::new(), "workspace");
        let capability =
            WorkspaceKnowledgeDigestCapability::new(memory_client, scope, 8, Some(tombstones));

        let rows = capability
            .digest_for_request(&assemble_input("credential note"))
            .await
            .unwrap();

        assert!(rows
            .iter()
            .all(|row| !row.evidence_memory_ids.contains(&deleted_id.0.to_string())));
        assert!(
            rows.iter()
                .any(|row| { row.evidence_memory_ids.contains(&kept_id.0.to_string()) })
                || rows.is_empty()
        );
    }
}
