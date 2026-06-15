//! Execution-control adapter for PlanLoop / WorkerLoop session lifecycle.
//!
//! Bridges web `AppState` to `ExecutionControlSessionLoopCoordinator` so register,
//! wake, and shutdown operations are auditable before touching local waker seams.

use std::sync::Arc;

use crate::state::AppState;

/// Build the session-loop execution-control coordinator bound to web `AppState`.
pub(crate) fn session_loop_coordinator(
    state: &Arc<AppState>,
) -> macaca_host_composition::execution_control::ExecutionControlSessionLoopCoordinator {
    macaca_host_composition::execution_control::ExecutionControlSessionLoopCoordinator::new(
        Arc::clone(&state.service_runtime),
    )
}
