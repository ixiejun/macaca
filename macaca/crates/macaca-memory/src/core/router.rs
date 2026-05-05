use macaca_proto::MacacaResult;
use serde::{Deserialize, Serialize};

use super::scope::{MemoryScope, MemoryVisibility};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRoute {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
    Composite(Vec<MemoryRoute>),
}

pub trait MemoryRouter: Send + Sync {
    fn route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute>;
    fn recall_route(&self, scope: &MemoryScope) -> MacacaResult<MemoryRoute>;
}

#[derive(Debug, Clone, Default)]
pub struct DefaultMemoryRouter;

impl MemoryRouter for DefaultMemoryRouter {
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
