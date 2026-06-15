//! Kernel agent-activity projection adapter.
//!
//! Maps generic executor lifecycle events into coarse kernel status updates so
//! the agent panel stays consistent with SSE and audit streams.

use std::sync::Arc;

use macaca_host_composition::executor::ExecutorEvent;

use crate::state::AppState;

/// Synchronize visible agent activity with a chat lifecycle edge.
///
/// The kernel status tracker remains the semantic owner of agent activity.  The
/// Web chat route only knows that it is starting or finishing a session, so this
/// adapter updates an app-declared agent by identity without inspecting any
/// application-specific payload, provider name, or business-domain data.
pub(crate) async fn update_agent_activity_by_name(
    state: &Arc<AppState>,
    agent_name: &str,
    activity: macaca_proto::AgentActivity,
) {
    if let Some(manifest) = state.kernel.get_agent_by_name(agent_name).await {
        state
            .kernel
            .update_agent_activity(&manifest.id, activity)
            .await;
    }
}

/// Project delegated executor lifecycle events into the kernel status tracker.
///
/// Delegated worker details are already persisted through EventLog and streamed
/// through SSE.  This helper keeps the coarse agent status panel consistent with
/// those same generic executor events, including WASM `macaca:agent/delegate`
/// paths that do not pass through the framework worker loop.
pub(crate) async fn sync_delegated_agent_activity_from_executor_event(
    state: &Arc<AppState>,
    event: &ExecutorEvent,
) {
    match event {
        ExecutorEvent::TaskStarted { task_id, agent } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Working {
                    context: format!("Executing delegated task {}", task_id),
                },
            )
            .await;
        }
        ExecutorEvent::TaskCompleted { agent, result, .. } if result.success => {
            update_agent_activity_by_name(state, agent, macaca_proto::AgentActivity::Idle).await;
        }
        ExecutorEvent::TaskCompleted { agent, result, .. } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Error {
                    message: result.output.clone(),
                },
            )
            .await;
        }
        ExecutorEvent::TaskFailed { agent, error, .. } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Error {
                    message: error.clone(),
                },
            )
            .await;
        }
        ExecutorEvent::AgentEvent { .. }
        | ExecutorEvent::TaskCancelled { .. }
        | ExecutorEvent::TaskProgress { .. }
        | ExecutorEvent::HookEvent { .. }
        | ExecutorEvent::Shutdown => {}
    }
}
