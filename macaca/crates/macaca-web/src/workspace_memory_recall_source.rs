//! Workspace-backed [`macaca_context::MemorySourceProvider`] used by
//! [`macaca_context::active_recall::DefaultActiveRecallProvider`].
//!
//! ## Routing model
//! [`macaca_context::MemoryRecallQuery`] carries **session / application / agent** hints. The
//! runtime-backed memory facade performs recall; this adapter enforces conservative visibility
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
use macaca_memory::{
    ActiveRecallBudget, ActiveRecallRequest, MemoryScope, MemorySearchRequest, TombstoneIndex,
};
use macaca_proto::{MacacaResult, MemoryEntry};

use crate::memory_runtime::WebMemoryRuntime;

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

/// Binds [`WebMemoryRuntime`] recall to the composer active-recall contract.
pub struct WorkspaceMemoryRecallSource {
    runtime: Arc<WebMemoryRuntime>,
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
        runtime: Arc<WebMemoryRuntime>,
        scope: MemoryScope,
        search_limit: usize,
        current_agent_id: Option<macaca_proto::AgentId>,
        tombstones: Option<Arc<dyn TombstoneIndex>>,
    ) -> Self {
        Self {
            runtime,
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

        let entries =
            if self.scope.identity.agent_id.is_some() || self.scope.identity.agent_name.is_some() {
                let result = self
                    .runtime
                    .active_recall(ActiveRecallRequest {
                        scope: self.scope.clone(),
                        query: query.query.clone(),
                        budget: ActiveRecallBudget {
                            max_hits: self.search_limit,
                            max_chars: usize::MAX,
                            ..ActiveRecallBudget::default()
                        },
                    })
                    .await?;
                result.selected
            } else {
                self.runtime
                    .search(MemorySearchRequest::new(
                        self.scope.clone(),
                        query.query.clone(),
                        self.search_limit,
                    ))
                    .await?
            };
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
        ActiveRecallCandidate, ActiveRecallDecision, ActiveRecallRequest, ActiveRecallResult,
        KnowledgeCompileCapability, KnowledgeCompileRequest, KnowledgeCompileResult,
        MemoryBackendConfig, MemoryBackendFactory, MemoryDeleteRequest, MemoryGetRequest,
        MemoryRuntimeFacade, MemoryRuntimeStatus, MemorySearchRequest, MemoryWriteRequest,
        RememberText, SharedTombstoneRegistry,
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

        let runtime = Arc::new(
            crate::memory_runtime::WebMemoryRuntime::from_workspace_memory(Arc::clone(&mgr)),
        );
        let scope = macaca_memory::MemoryScope::project_shared(
            macaca_proto::ApplicationId::new(),
            "workspace",
        );
        let with_ts = WorkspaceMemoryRecallSource::new(
            Arc::clone(&runtime),
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

        let no_ts = WorkspaceMemoryRecallSource::new(Arc::clone(&runtime), scope, 8, None, None);
        let hits = MemorySourceProvider::recall(&no_ts, q).await.unwrap();
        assert!(!hits.is_empty());
    }

    struct RecallFakeRuntime {
        entries: Vec<MemoryEntry>,
    }

    #[async_trait]
    impl MemoryRuntimeFacade for RecallFakeRuntime {
        async fn remember(&self, _request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
            Ok(MemoryId::new())
        }

        async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
            Ok(self.entries.clone())
        }

        async fn get(&self, _request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
            Ok(())
        }

        async fn active_recall(
            &self,
            _request: ActiveRecallRequest,
        ) -> MacacaResult<ActiveRecallResult> {
            let candidates = self
                .entries
                .iter()
                .cloned()
                .map(|entry| ActiveRecallCandidate {
                    entry,
                    score: 100,
                    estimated_tokens: 1,
                    decision: ActiveRecallDecision::selected("fake"),
                })
                .collect();
            Ok(ActiveRecallResult {
                provider_id: "fake-runtime".into(),
                candidates,
                selected: self.entries.clone(),
                latency_ms: 0,
                diagnostics: Vec::new(),
            })
        }

        async fn compile_knowledge(
            &self,
            request: KnowledgeCompileRequest,
        ) -> MacacaResult<KnowledgeCompileResult> {
            Ok(macaca_memory::KnowledgeCompiler::default().compile(request))
        }

        async fn status(&self) -> MemoryRuntimeStatus {
            MemoryRuntimeStatus::default()
        }
    }

    #[tokio::test]
    async fn active_recall_source_uses_runtime_and_preserves_scope_filtering() {
        let current_agent = AgentId::new();
        let other_agent = AgentId::new();
        let runtime = Arc::new(crate::memory_runtime::WebMemoryRuntime::new(Arc::new(
            RecallFakeRuntime {
                entries: vec![
                    entry_with_agent(Some(current_agent)),
                    entry_with_agent(Some(other_agent)),
                    entry_with_agent(None),
                ],
            },
        )));
        let source = WorkspaceMemoryRecallSource::new(
            runtime,
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
