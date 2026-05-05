use macaca_proto::MacacaResult;
use serde::{Deserialize, Serialize};

use super::scope::{MemoryScope, MemoryVisibility};

/// Logical routing targets understood by the memory fabric.
///
/// These are not storage engines themselves. They represent visibility classes
/// that a router can map onto concrete providers or builtin adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRoute {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
    Composite(Vec<MemoryRoute>),
}

/// Routing policy for memory operations.
///
/// `route()` is used for write/get/delete style operations where the request
/// must land on exactly one authority.
///
/// `recall_route()` may widen the search space in a controlled way, because
/// recall often wants "private first, then shared" semantics without exposing
/// that policy to every caller.
pub trait MemoryRouter: Send + Sync {
    fn route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute>;
    fn recall_route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute>;
}

/// Default visibility-based router shipped with the core fabric.
///
/// The default policy is intentionally conservative:
/// - writes follow the exact visibility requested by the caller
/// - recalls may widen from `AgentPrivate` to `AgentPrivate + SessionShared`
///   when the scope also carries session/project context, because that is a
///   common agent workflow for "my notes plus shared project memory"
#[derive(Debug, Clone, Default)]
pub struct DefaultMemoryRouter;

impl MemoryRouter for DefaultMemoryRouter {
    /// Resolve the authoritative route for mutation and direct lookups.
    fn route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute> {
        scope.validate()?;
        Ok(match scope.visibility {
            MemoryVisibility::AgentPrivate => MemoryRoute::AgentPrivate,
            MemoryVisibility::SessionShared => MemoryRoute::SessionShared,
            MemoryVisibility::ApplicationShared => MemoryRoute::ApplicationShared,
            MemoryVisibility::UserScoped => MemoryRoute::UserScoped,
            MemoryVisibility::GlobalSystem => MemoryRoute::GlobalSystem,
        })
    }

    /// Resolve the route for recall.
    ///
    /// Agent-private recall can widen into a composite route when the caller
    /// also provides session/project context. This preserves private-first
    /// isolation while still allowing a single fabric search call to merge
    /// private and shared context in a deterministic order.
    fn recall_route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute> {
        scope.validate()?;
        if scope.visibility == MemoryVisibility::AgentPrivate
            && (scope.identity.session_id.is_some() || scope.identity.project_id.is_some())
        {
            return Ok(MemoryRoute::Composite(vec![
                MemoryRoute::AgentPrivate,
                MemoryRoute::SessionShared,
            ]));
        }
        self.route(scope)
    }
}
