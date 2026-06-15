//! Memory-related helper strategies for context-reporting model calls.
//!
//! The reporting chat model owns LLM orchestration, while this module owns the
//! memory Strategy construction used by context providers. Splitting the code
//! keeps the hot-path model wrapper small and makes recall/digest scope rules
//! auditable in isolation.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::context::{
    ActiveRecallBudget, ActiveRecallCapability, ActiveRecallPolicy, DefaultActiveRecallProvider,
    KnowledgeDigestCapability, MemoryPrefetchResult, MemoryRecallQuery,
};
use macaca_host_composition::memory::{MemoryScope, SharedTombstoneRegistry, TombstoneIndex};
use macaca_proto::{config::ContextConfig, AgentId, ApplicationId, MacacaError, MacacaResult};

use crate::workspace_knowledge_digest_capability::WorkspaceKnowledgeDigestCapability;
use crate::workspace_memory_recall_source::WorkspaceMemoryRecallSource;

/// Build the active-recall scope for the current framework model call.
///
/// Agent-private scope is preferred when the kernel resolved a concrete agent
/// id. When an agent id is unavailable, the runtime falls back to
/// session/project shared scope rather than guessing a private owner.
pub(crate) fn recall_scope(
    application_id: ApplicationId,
    session_id: Option<&str>,
    routing_agent_id: Option<AgentId>,
) -> MemoryScope {
    if let Some(agent_id) = routing_agent_id {
        return MemoryScope::agent_private(application_id, agent_id)
            .session_id(session_id.unwrap_or("workspace"));
    }
    digest_scope(application_id, session_id)
}

/// Build the shared digest scope used by runtime-backed knowledge compilation.
///
/// The session id is used when available. A stable project-shared fallback keeps
/// non-session tool and digest paths scoped to the application workspace without
/// introducing application-specific names or unscoped global memory reads.
pub(crate) fn digest_scope(application_id: ApplicationId, session_id: Option<&str>) -> MemoryScope {
    match session_id {
        Some(id) if !id.trim().is_empty() => MemoryScope::session_shared(application_id, id),
        _ => MemoryScope::project_shared(application_id, "workspace"),
    }
}

/// Builds the [`ActiveRecallCapability`] stack used by the context composer.
///
/// Returns `None` when the feature is disabled. The source talks through
/// `SystemMemoryClient`, so recall remains service-backed and independent from
/// a concrete memory runtime implementation.
pub(crate) fn build_workspace_recall_capability(
    cfg: &ContextConfig,
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    tombstones: Option<&Arc<SharedTombstoneRegistry>>,
    application_id: ApplicationId,
    session_id: Option<&str>,
    routing_agent_id: Option<AgentId>,
) -> Option<Arc<dyn ActiveRecallCapability>> {
    if !cfg.active_vector_memory.enabled {
        return None;
    }
    let tomb_index: Option<Arc<dyn macaca_host_composition::memory::TombstoneIndex>> = tombstones
        .map(|reg| Arc::clone(reg) as Arc<dyn macaca_host_composition::memory::TombstoneIndex>);
    let scope = recall_scope(application_id, session_id, routing_agent_id);
    let source = Arc::new(WorkspaceMemoryRecallSource::new(
        memory_client,
        scope,
        cfg.active_vector_memory.max_hits,
        routing_agent_id,
        tomb_index,
    ));
    let policy = ActiveRecallPolicy {
        budget: ActiveRecallBudget {
            max_hits: cfg.active_vector_memory.max_hits,
            max_chars: cfg.active_vector_memory.max_chars,
            max_tokens: cfg.active_vector_memory.max_tokens,
            timeout_ms: cfg.active_vector_memory.timeout_ms,
        },
        ..ActiveRecallPolicy::default()
    };
    Some(Arc::new(DefaultActiveRecallProvider::new(
        "workspace-active-recall",
        source,
        policy,
    )))
}

/// Service-owned active recall capability that derives memory scope from each request.
///
/// `service.context` is process-wide, so it cannot safely hold the per-call app/session/agent
/// capability created by [`build_workspace_recall_capability`].  This adapter keeps the same
/// service-backed retrieval and budget policy while rebuilding a conservative session/shared
/// scope from [`MemoryRecallQuery`] on every compose call.
struct ContextServiceWorkspaceRecallCapability {
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    tombstones: Option<Arc<dyn TombstoneIndex>>,
    config: macaca_proto::config::ActiveVectorMemoryContextConfig,
}

impl ContextServiceWorkspaceRecallCapability {
    fn query_scope(query: &MemoryRecallQuery) -> MacacaResult<MemoryScope> {
        let raw_app = query
            .application_id
            .as_deref()
            .ok_or_else(|| MacacaError::Config("active recall requires application_id".into()))?;
        let app_uuid = uuid::Uuid::parse_str(raw_app).map_err(|err| {
            MacacaError::Config(format!("active recall application_id is invalid: {err}"))
        })?;
        Ok(digest_scope(
            ApplicationId(app_uuid),
            query.session_id.as_deref(),
        ))
    }
}

#[async_trait]
impl ActiveRecallCapability for ContextServiceWorkspaceRecallCapability {
    fn provider_id(&self) -> &str {
        "workspace-active-recall"
    }

    async fn prefetch(&self, query: MemoryRecallQuery) -> MacacaResult<MemoryPrefetchResult> {
        let scope = Self::query_scope(&query)?;
        let source = Arc::new(WorkspaceMemoryRecallSource::new(
            Arc::clone(&self.memory_client),
            scope,
            self.config.max_hits,
            None,
            self.tombstones.clone(),
        ));
        let policy = ActiveRecallPolicy {
            budget: ActiveRecallBudget {
                max_hits: self.config.max_hits,
                max_chars: self.config.max_chars,
                max_tokens: self.config.max_tokens,
                timeout_ms: self.config.timeout_ms,
            },
            ..ActiveRecallPolicy::default()
        };
        DefaultActiveRecallProvider::new("workspace-active-recall", source, policy)
            .prefetch(query)
            .await
    }
}

/// Builds the Context Service active-recall capability.
///
/// The returned capability is intentionally request-scoped internally: the service provider can
/// store it globally, but every `prefetch` call reconstructs the app/session memory scope from the
/// incoming context assembly request.
pub(crate) fn build_context_service_recall_capability(
    cfg: &ContextConfig,
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    tombstones: Option<&Arc<SharedTombstoneRegistry>>,
) -> Option<Arc<dyn ActiveRecallCapability>> {
    if !cfg.active_vector_memory.enabled {
        return None;
    }
    let tomb_index: Option<Arc<dyn TombstoneIndex>> =
        tombstones.map(|reg| Arc::clone(reg) as Arc<dyn TombstoneIndex>);
    Some(Arc::new(ContextServiceWorkspaceRecallCapability {
        memory_client,
        tombstones: tomb_index,
        config: cfg.active_vector_memory.clone(),
    }))
}

/// Builds the [`KnowledgeDigestCapability`] used by the knowledge digest provider.
///
/// Disabled when configuration toggles digest compilation off. The resulting
/// capability pulls memory rows through `SystemMemoryClient`, preserving the
/// service boundary while keeping digest scope decisions local and auditable.
pub(crate) fn build_workspace_knowledge_digest_capability(
    cfg: &ContextConfig,
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    tombstones: Option<&Arc<SharedTombstoneRegistry>>,
    application_id: ApplicationId,
    session_id: Option<&str>,
) -> Option<Arc<dyn KnowledgeDigestCapability>> {
    if !cfg.knowledge_digest.enabled {
        return None;
    }
    let tomb_index: Option<Arc<dyn macaca_host_composition::memory::TombstoneIndex>> = tombstones
        .map(|registry| {
            Arc::clone(registry) as Arc<dyn macaca_host_composition::memory::TombstoneIndex>
        });
    Some(Arc::new(WorkspaceKnowledgeDigestCapability::new(
        memory_client,
        digest_scope(application_id, session_id),
        cfg.knowledge_digest.max_compiler_rows,
        tomb_index,
    )))
}
