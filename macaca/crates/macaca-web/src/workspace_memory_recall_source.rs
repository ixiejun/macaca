//! Workspace-backed [`macaca_context::MemorySourceProvider`] used by
//! [`macaca_context::active_recall::DefaultActiveRecallProvider`].
//!
//! ## Routing model
//! [`macaca_context::MemoryRecallQuery`] carries **session / application / agent** hints. The
//! service-backed memory client performs recall; this adapter enforces conservative visibility
//! rules so agent-private rows tagged with another [`AgentId`]
//! cannot surface during recall (fail-closed for cross-agent isolation).
//!
//! ## Tombstones
//! Optional [`macaca_memory::TombstoneIndex`] aligns recall with digest compilation and
//! `memory_forget` tooling so governance-deleted ids cannot reappear in fenced recall.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    memory_source, ConfidenceScore, ContextSourceProvenance, MemoryRecallItem, MemoryRecallQuery,
    MemorySourceProvider, PrivacyTier,
};
use macaca_memory::{MemoryPolicyHints, MemoryPrefetchCommand, MemoryScope, TombstoneIndex};
use macaca_proto::{MacacaResult, MemoryEntry, TraceContext};

/// Pure predicate: whether a workspace [`MemoryEntry`] may be returned for active recall.
///
/// This implements the **scope routing** contract for the in-process `TestMemoryManager` bridge:
/// recall search itself is unaware of agent/session facets, so filtering happens here (Strategy
/// pattern: pluggable policy in front of a generic vector/text search).
///
/// ## Rules
/// - **Agent-tagged rows** (`entry.agent_id.is_some()`): visible only when
///   `query.include_agent_private` is true **and** `current_agent_id == entry.agent_id`. If the
///   caller did not resolve `current_agent_id`, we **fail closed** and hide all agent-tagged rows
///   so private memories never leak across agents when routing metadata is missing.
/// - **Untagged rows**: treated as session-shared catalog content; gated only by
///   `query.include_session_shared`.
#[must_use]
pub(crate) fn workspace_memory_entry_visible_for_recall(
    entry: &MemoryEntry,
    query: &MemoryRecallQuery,
    current_agent_id: Option<macaca_proto::AgentId>,
) -> bool {
    if let Some(owner) = entry.agent_id {
        if let Some(current) = current_agent_id {
            if owner != current {
                return false;
            }
        } else {
            // Fail-closed: agent-tagged rows without a resolved routing id must not leak.
            return false;
        }
        return query.include_agent_private;
    }

    query.include_session_shared
}

/// Binds the Memory Service client to the composer active-recall contract.
pub struct WorkspaceMemoryRecallSource {
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    scope: MemoryScope,
    search_limit: usize,
    /// When known, agent-private memories tagged for another id are filtered out.
    current_agent_id: Option<macaca_proto::AgentId>,
    tombstones: Option<Arc<dyn TombstoneIndex>>,
}

impl WorkspaceMemoryRecallSource {
    /// `search_limit` is typically derived from [`macaca_proto::config::ActiveVectorMemoryContextConfig::max_hits`].
    #[must_use]
    pub fn new(
        memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
        scope: MemoryScope,
        search_limit: usize,
        current_agent_id: Option<macaca_proto::AgentId>,
        tombstones: Option<Arc<dyn TombstoneIndex>>,
    ) -> Self {
        Self {
            memory_client,
            scope,
            search_limit: search_limit.max(1),
            current_agent_id,
            tombstones,
        }
    }

    fn map_entry(&self, entry: MemoryEntry) -> MemoryRecallItem {
        let source_id = entry.id.0.to_string();
        MemoryRecallItem {
            source: memory_source(
                "workspace-memory",
                &source_id,
                format!("layer:{:?}", entry.layer),
            ),
            text: entry.content,
            provenance: ContextSourceProvenance {
                provider_id: "workspace-memory".into(),
                source_id,
                evidence: vec![format!("layer:{:?}", entry.layer)],
            },
            confidence: ConfidenceScore::new(85),
            privacy_tier: PrivacyTier::Workspace,
        }
    }

    fn row_visible(&self, entry: &MemoryEntry, query: &MemoryRecallQuery) -> bool {
        workspace_memory_entry_visible_for_recall(entry, query, self.current_agent_id)
    }

    async fn load_candidates(
        &self,
        query: &MemoryRecallQuery,
    ) -> MacacaResult<Vec<MemoryRecallItem>> {
        let tomb_set: HashSet<String> = match &self.tombstones {
            Some(idx) => match idx.tombstoned_memory_id_strings().await {
                Ok(v) => v.into_iter().collect(),
                Err(e) => {
                    tracing::warn!(
                        target: "macaca.workspace_memory",
                        error = %e,
                        "tombstone snapshot failed during recall; continuing without tombstone filter (fail-open)"
                    );
                    HashSet::new()
                }
            },
            None => HashSet::new(),
        };

        let mut trace = TraceContext::new(uuid::Uuid::new_v4().to_string());
        trace.session_id = self.scope.identity.session_id.clone();
        trace.agent = self.scope.identity.agent_name.clone();
        let result = self
            .memory_client
            .prefetch(MemoryPrefetchCommand {
                scope: self.scope.clone(),
                trace,
                query: query.query.clone(),
                limit: self.search_limit,
                policy: MemoryPolicyHints::default(),
            })
            .await?;
        let entries = result.entries;
        let mut items = Vec::new();
        for entry in entries {
            if tomb_set.contains(&entry.id.0.to_string()) {
                continue;
            }
            if self.row_visible(&entry, query) {
                items.push(self.map_entry(entry));
            }
        }
        Ok(items)
    }
}

#[async_trait]
impl MemorySourceProvider for WorkspaceMemoryRecallSource {
    fn provider_id(&self) -> &str {
        "workspace-memory"
    }

    async fn recall(&self, query: MemoryRecallQuery) -> MacacaResult<Vec<MemoryRecallItem>> {
        self.load_candidates(&query).await
    }
}

#[cfg(test)]
mod tests {
    use super::workspace_memory_entry_visible_for_recall;
    use super::WorkspaceMemoryRecallSource;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_context::{MemoryRecallQuery, MemorySourceProvider};
    use macaca_memory::{
        MemoryBackendConfig, MemoryBackendFactory, MemoryForgetCommand, MemoryPrefetchCommand,
        MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
        MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand,
        MemoryStatusReport, RememberText, SharedTombstoneRegistry,
    };
    use macaca_proto::{AgentId, MacacaResult, MemoryEntry, MemoryId, MemoryLayer};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn entry_with_agent(agent: Option<AgentId>) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Vector,
            content: "x".into(),
            metadata: serde_json::Value::Null,
            agent_id: agent,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[test]
    fn shared_row_requires_include_session_shared() {
        let entry = entry_with_agent(None);
        let mut q = MemoryRecallQuery::lite("q", 1024);
        q.include_session_shared = true;
        assert!(workspace_memory_entry_visible_for_recall(&entry, &q, None));
        q.include_session_shared = false;
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, None));
    }

    #[test]
    fn agent_row_requires_matching_route_and_flag() {
        let a = AgentId::new();
        let b = AgentId::new();
        let entry = entry_with_agent(Some(a));
        let mut q = MemoryRecallQuery::lite("q", 1024);
        assert!(workspace_memory_entry_visible_for_recall(
            &entry,
            &q,
            Some(a)
        ));
        assert!(!workspace_memory_entry_visible_for_recall(
            &entry,
            &q,
            Some(b)
        ));
        q.include_agent_private = false;
        assert!(!workspace_memory_entry_visible_for_recall(
            &entry,
            &q,
            Some(a)
        ));
    }

    #[test]
    fn agent_row_hidden_when_route_unresolved_fail_closed() {
        let a = AgentId::new();
        let entry = entry_with_agent(Some(a));
        let q = MemoryRecallQuery::lite("q", 1024);
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, None));
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
                stored_at: Utc::now(),
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
            command: macaca_memory::MemoryGetCommand,
        ) -> MacacaResult<macaca_memory::MemoryGetResult> {
            Ok(macaca_memory::MemoryGetResult::new(
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
                macaca_memory::MemoryCapabilitySet::basic_store_search(),
            ))
        }

        async fn snapshot(
            &self,
            _command: MemoryServiceSnapshotCommand,
        ) -> MacacaResult<MemoryServiceSnapshot> {
            Ok(MemoryServiceSnapshot::new(
                "test-memory-client",
                true,
                macaca_memory::MemoryCapabilitySet::basic_store_search(),
                None,
            ))
        }
    }

    async fn client_from_manager(
        mgr: &Arc<macaca_memory::TestMemoryManager>,
        query: &str,
    ) -> Arc<dyn macaca_sdk::SystemMemoryClient> {
        let entries = mgr
            .recall(macaca_memory::RecallQuery::new(query, 8))
            .await
            .unwrap()
            .entries;
        Arc::new(StaticMemoryClient { entries })
    }

    #[tokio::test]
    async fn tombstone_registry_hides_matching_row_before_mapping() {
        let dir = tempdir().unwrap();
        let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(dir.path().to_path_buf()));
        let mgr = Arc::new(factory.test_manager());
        let id = mgr
            .remember_text(RememberText::new("uniq-tombstone-recall-marker-xyz"))
            .await
            .unwrap();

        let reg = Arc::new(SharedTombstoneRegistry::new());
        reg.record(id).await;

        let memory_client = client_from_manager(&mgr, "uniq-tombstone-recall-marker-xyz").await;
        let scope = macaca_memory::MemoryScope::project_shared(
            macaca_proto::ApplicationId::new(),
            "workspace",
        );
        let with_ts = WorkspaceMemoryRecallSource::new(
            Arc::clone(&memory_client),
            scope.clone(),
            8,
            None,
            Some(reg),
        );
        let q = MemoryRecallQuery::lite("uniq-tombstone-recall-marker-xyz", 1024);
        let empty = MemorySourceProvider::recall(&with_ts, q.clone())
            .await
            .unwrap();
        assert!(empty.is_empty());

        let no_ts =
            WorkspaceMemoryRecallSource::new(Arc::clone(&memory_client), scope, 8, None, None);
        let hits = MemorySourceProvider::recall(&no_ts, q).await.unwrap();
        assert!(!hits.is_empty());
    }

    #[tokio::test]
    async fn active_recall_source_uses_runtime_and_preserves_scope_filtering() {
        let current_agent = AgentId::new();
        let other_agent = AgentId::new();
        let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
            entries: vec![
                entry_with_agent(Some(current_agent)),
                entry_with_agent(Some(other_agent)),
                entry_with_agent(None),
            ],
        });
        let source = WorkspaceMemoryRecallSource::new(
            memory_client,
            macaca_memory::MemoryScope::agent_private(
                macaca_proto::ApplicationId::new(),
                current_agent,
            ),
            8,
            Some(current_agent),
            None,
        );

        let mut query = MemoryRecallQuery::lite("x", 1024);
        query.include_agent_private = true;
        query.include_session_shared = false;
        let hits = source.recall(query).await.unwrap();

        assert_eq!(hits.len(), 1);
    }
}
