//! Memory-related helper strategies for context-reporting model calls.
//!
//! The reporting chat model owns LLM orchestration, while this module owns the
//! memory Strategy construction used by context providers. Splitting the code
//! keeps the hot-path model wrapper small and makes recall/digest scope rules
//! auditable in isolation.

use std::sync::Arc;

use macaca_context::{
    ActiveRecallBudget, ActiveRecallCapability, ActiveRecallPolicy, DefaultActiveRecallProvider,
    KnowledgeDigestCapability,
};
use macaca_memory::{MemoryScope, SharedTombstoneRegistry};
use macaca_proto::{config::ContextConfig, AgentId, ApplicationId};

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
    let tomb_index: Option<Arc<dyn macaca_memory::TombstoneIndex>> =
        tombstones.map(|reg| Arc::clone(reg) as Arc<dyn macaca_memory::TombstoneIndex>);
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
    let tomb_index: Option<Arc<dyn macaca_memory::TombstoneIndex>> =
        tombstones.map(|registry| Arc::clone(registry) as Arc<dyn macaca_memory::TombstoneIndex>);
    Some(Arc::new(WorkspaceKnowledgeDigestCapability::new(
        memory_client,
        digest_scope(application_id, session_id),
        cfg.knowledge_digest.max_compiler_rows,
        tomb_index,
    )))
}
