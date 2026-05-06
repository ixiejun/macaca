//! Workspace-backed [`macaca_context::MemorySourceProvider`] used by
//! [`macaca_context::active_recall::DefaultActiveRecallProvider`].
//!
//! ## Routing model
//! [`macaca_context::MemoryRecallQuery`] carries **session / application / agent** hints. The
//! legacy [`macaca_memory::TestMemoryManager`] performs a unified vector/text search; this adapter
//! enforces conservative visibility rules so agent-private rows tagged with another [`AgentId`]
//! cannot surface during recall (fail-closed for cross-agent isolation).

use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    memory_source, ConfidenceScore, ContextSourceProvenance, MemoryRecallItem, MemoryRecallQuery,
    MemorySourceProvider, PrivacyTier,
};
use macaca_memory::{RecallQuery, RecallResult, TestMemoryManager};
use macaca_proto::{MemoryEntry, MacacaResult};

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

/// Binds [`TestMemoryManager`] recall to the composer active-recall contract.
pub struct WorkspaceMemoryRecallSource {
    manager: Arc<TestMemoryManager>,
    search_limit: usize,
    /// When known, agent-private memories tagged for another id are filtered out.
    current_agent_id: Option<macaca_proto::AgentId>,
}

impl WorkspaceMemoryRecallSource {
    /// `search_limit` is typically derived from [`macaca_proto::config::ActiveVectorMemoryContextConfig::max_hits`].
    #[must_use]
    pub fn new(
        manager: Arc<TestMemoryManager>,
        search_limit: usize,
        current_agent_id: Option<macaca_proto::AgentId>,
    ) -> Self {
        Self {
            manager,
            search_limit: search_limit.max(1),
            current_agent_id,
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
        let bundle: RecallResult = self
            .manager
            .recall(RecallQuery::new(&query.query, self.search_limit))
            .await?;
        let mut items = Vec::new();
        for entry in bundle.entries {
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
    use chrono::Utc;
    use macaca_context::MemoryRecallQuery;
    use macaca_proto::{AgentId, MemoryEntry, MemoryId, MemoryLayer};

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
        assert!(workspace_memory_entry_visible_for_recall(
            &entry,
            &q,
            None
        ));
        q.include_session_shared = false;
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, None));
    }

    #[test]
    fn agent_row_requires_matching_route_and_flag() {
        let a = AgentId::new();
        let b = AgentId::new();
        let entry = entry_with_agent(Some(a));
        let mut q = MemoryRecallQuery::lite("q", 1024);
        assert!(workspace_memory_entry_visible_for_recall(&entry, &q, Some(a)));
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, Some(b)));
        q.include_agent_private = false;
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, Some(a)));
    }

    #[test]
    fn agent_row_hidden_when_route_unresolved_fail_closed() {
        let a = AgentId::new();
        let entry = entry_with_agent(Some(a));
        let q = MemoryRecallQuery::lite("q", 1024);
        assert!(!workspace_memory_entry_visible_for_recall(&entry, &q, None));
    }
}
