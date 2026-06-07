//! Shared context for PlanEvent consumer handlers (Strategy pattern).
//!
//! PlanLoop spawns one long-lived consumer task. Each `PlanEvent` variant is
//! handled by a dedicated function that receives the same context bundle so we
//! avoid threading many `Arc` clones through every signature.

use std::sync::Arc;

use macaca_runtime_host::persist::PersistBackend;
use macaca_proto::ApplicationId;
use tokio::sync::mpsc::Sender;

use crate::state::AppState;

/// Immutable per-consumer context captured when PlanLoop spawns its event task.
pub(crate) struct PlanEventConsumerCtx {
    /// Application runtime state (persist, kernel, service runtime, loops).
    pub state: Arc<AppState>,
    /// Application scope for all task/plan operations in this consumer.
    pub app_id: ApplicationId,
    /// Session store used to persist plan decisions for audit replay.
    pub session_store: Arc<dyn PersistBackend>,
    /// Manifest-resolved planning agent name (not hardcoded).
    pub plan_agent_name: String,
    /// Manifest-resolved entry agent name (excluded from worker candidate list).
    pub entry_agent_name: String,
    /// Channel back into macaca-task PlanLoop for synthetic events (e.g. GoalCompleted).
    pub event_tx: Sender<macaca_sdk::task::PlanEvent>,
}
