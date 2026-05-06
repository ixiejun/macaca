//! [`WorkspaceKnowledgeDigestCapability`] — Adapter implementing
//! [`macaca_context::KnowledgeDigestCapability`] on top of [`crate::memory_runtime::WebMemoryRuntime`].
//!
//! ## Flow
//! 1. Derive the same **retrieval query** string the active-recall provider would use
//!    (latest user turn text via [`macaca_context::ContextAssembleInput::last_user_message_text`]).
//! 2. Pull a bounded set of [`macaca_proto::MemoryEntry`] rows through the runtime facade.
//! 3. Lift rows into [`macaca_memory::MemoryCandidate`] objects inside a neutral [`macaca_memory::MemoryScope`].
//! 4. Run the runtime-backed knowledge compiler.
//! 5. Map non-revoked [`macaca_memory::KnowledgeClaim`] rows into [`macaca_context::KnowledgeDigestItem`], copying
//!    only **opaque** evidence ids (`ClaimEvidence::source_id`) so digest-vs-raw suppression can align with
//!    fenced recall rows without leaking full memory payloads into structured reports.
//! 6. Apply [`macaca_context::filter_digest_items_by_tombstones`] when an optional [`macaca_memory::TombstoneIndex`] is wired
//!    (shared registry updated by [`crate::context_memory_tools::WorkspaceMemoryForgetTool`] or a governance facade snapshot).
//!
//! ## Failure semantics
//! Any fatal error returns `Ok(vec![])` — the composer treats “no digest” as a benign outcome (fail-open).
//! Tombstone snapshots that fail **only** skip filtering and log — digest rows still compile (explicit fail-open boundary).

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    filter_digest_items_by_tombstones, ContextAssembleInput, ContextSourceProvenance,
    KnowledgeDigestCapability, KnowledgeDigestItem, PrivacyTier,
};
use macaca_memory::{
    CandidateSource, KnowledgeCompileRequest, MemoryCandidate, MemoryScope, MemorySearchRequest,
    TombstoneIndex,
};
use macaca_proto::MacacaResult;

use crate::memory_runtime::WebMemoryRuntime;

/// Workspace memory → knowledge digest port adapter.
pub struct WorkspaceKnowledgeDigestCapability {
    runtime: Arc<WebMemoryRuntime>,
    scope: MemoryScope,
    search_limit: usize,
    /// Optional tombstone ledger — when present, evidence pointers referencing deleted ids are stripped.
    tombstones: Option<Arc<dyn TombstoneIndex>>,
}

impl WorkspaceKnowledgeDigestCapability {
    /// `search_limit` should track [`macaca_proto::config::KnowledgeDigestContextConfig::max_compiler_rows`].
    ///
    /// `tombstones` integrates [`SharedTombstoneRegistry`] / [`GovernanceFacadeTombstones`] implementations from
    /// [`macaca_memory`] without coupling this crate to a single storage backend.
    #[must_use]
    pub fn new(
        runtime: Arc<WebMemoryRuntime>,
        scope: MemoryScope,
        search_limit: usize,
        tombstones: Option<Arc<dyn TombstoneIndex>>,
    ) -> Self {
        Self {
            runtime,
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
        let entries = self
            .runtime
            .search(MemorySearchRequest::new(
                scope.clone(),
                query.to_owned(),
                self.search_limit,
            ))
            .await?;

        let candidates: Vec<MemoryCandidate> = entries
            .into_iter()
            .map(|entry| {
                MemoryCandidate::new(
                    entry.id.0.to_string(),
                    scope.clone(),
                    CandidateSource::AgentSummary,
                    entry.content,
                )
                .confidence(0.9)
                .recurrence_count(2)
            })
            .collect();

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let compiled = self
            .runtime
            .compile_knowledge(KnowledgeCompileRequest {
                scope,
                candidates,
                existing_claims: Vec::new(),
            })
            .await?;

        let mut out = Vec::new();
        for claim in compiled.claims {
            if claim.revoked {
                continue;
            }
            out.push(KnowledgeDigestItem {
                claim_id: claim.id.clone(),
                label: "compiled-knowledge".into(),
                statement: claim.statement,
                provenance: ContextSourceProvenance {
                    provider_id: self.capability_id().into(),
                    source_id: claim.id,
                    evidence: claim.evidence.iter().map(|e| e.source_id.clone()).collect(),
                },
                evidence_memory_ids: claim.evidence.iter().map(|e| e.source_id.clone()).collect(),
                confidence: claim.confidence.0,
                freshness: claim.freshness.0,
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

    use macaca_memory::{
        FileMemory, InMemoryVectorStore, MockEmbedding, RememberText, SessionMemory,
        SharedTombstoneRegistry, TestMemoryManager,
    };
    use macaca_proto::{ApplicationId, LlmMessage, LlmOptions};

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

        let runtime =
            Arc::new(crate::memory_runtime::WebMemoryRuntime::from_workspace_memory(manager));
        let scope = macaca_memory::MemoryScope::project_shared(ApplicationId::new(), "workspace");
        let capability = WorkspaceKnowledgeDigestCapability::new(runtime, scope, 8, None);

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

        let runtime =
            Arc::new(crate::memory_runtime::WebMemoryRuntime::from_workspace_memory(manager));
        let scope = macaca_memory::MemoryScope::project_shared(ApplicationId::new(), "workspace");
        let capability =
            WorkspaceKnowledgeDigestCapability::new(runtime, scope, 8, Some(tombstones));

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
