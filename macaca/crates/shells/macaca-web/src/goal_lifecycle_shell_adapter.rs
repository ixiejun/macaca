//! Shell adapter for goal-lifecycle pause/resume handoff (Adapter pattern).
//!
//! PlanLoop `GoalCompleted` events and `create_goal` tool callbacks cannot yet be
//! fully serialized as execution-control subscriptions inside the web shell.  This
//! module bridges task-service goal lifecycle signals into:
//! 1. Auditable `service.execution_control` resume commands (authoritative).
//! 2. Legacy in-memory resume channels owned by the waiting parent loop (compat).
//! 3. Session-visible SSE + event-log evidence for operators.
//!
//! The adapter is intentionally application-neutral: it never branches on agent
//! role names or workflow labels.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::response::sse::Event;
use macaca_sdk::framework::execution::ExecutionContext;
use macaca_sdk::framework::session::{load_module_state, save_module_state};
use macaca_runtime_host::persist::AppendEventCommand;
use macaca_proto::{
    ApplicationId, EXECUTION_CONTROL_SERVICE_ID, TaskId, TraceContext,
};
use macaca_runtime_host::{
    ExecutionControlGoalLifecycleCoordinator, GoalLifecycleParentResumeRequest,
    GoalLifecycleParentWaitRequest,
};
use tracing::{info, warn};

use crate::runtime_resume::RuntimeResumeSignal;
use crate::state::{ActiveSession, AppState};

/// Provider-neutral reason codes recorded in execution-control audit replay.
pub const REASON_GOAL_LIFECYCLE_PARENT_RESUME_SUCCESS: &str =
    "execution_control.goal_lifecycle.parent_resume.success";

/// Register a goal-bound parent pause through execution control when `create_goal`
/// records a session mapping.
///
/// Failures are logged but do not block local ExecutionContext pause bookkeeping:
/// the shell must keep functioning while execution-control migration is in progress.
pub async fn register_goal_wait_via_execution_control(
    coordinator: &ExecutionControlGoalLifecycleCoordinator,
    application_id: ApplicationId,
    session_id: String,
    parent_agent: String,
    goal_id: TaskId,
) {
    let trace = TraceContext::new(format!("goal-lifecycle-wait:{session_id}:{}", goal_id.0));
    let wait_request = GoalLifecycleParentWaitRequest {
        application_id,
        session_id: session_id.clone(),
        parent_agent,
        goal_id,
        trace,
    };

    match coordinator.register_parent_goal_wait(wait_request).await {
        Ok(result) => {
            info!(
                session_id = %session_id,
                goal_id = %goal_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                state = ?result.state,
                "Goal-lifecycle parent wait registered via execution control"
            );
        }
        Err(error) => {
            warn!(
                session_id = %session_id,
                goal_id = %goal_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                error = %error,
                "Goal-lifecycle execution-control registration failed; continuing local pause adapter"
            );
        }
    }
}

/// Deliver a goal-lifecycle resume through execution control, then wake the local
/// parent loop channel when the session is still active.
pub async fn deliver_goal_resume_and_notify_parent(
    state: &Arc<AppState>,
    coordinator: &ExecutionControlGoalLifecycleCoordinator,
    application_id: &ApplicationId,
    session_id: &str,
    parent_agent: &str,
    goal_id: TaskId,
    description: &str,
    active_session: Option<&ActiveSession>,
) {
    let trace = TraceContext::new(format!("goal-lifecycle-resume:{session_id}:{}", goal_id.0));
    let resume_request = GoalLifecycleParentResumeRequest {
        application_id: *application_id,
        session_id: session_id.to_string(),
        parent_agent: parent_agent.to_string(),
        goal_id,
        trace,
        reason_code: REASON_GOAL_LIFECYCLE_PARENT_RESUME_SUCCESS.to_string(),
        completion_summary: Some(description.to_string()),
    };

    match coordinator.deliver_parent_goal_resume(resume_request).await {
        Ok(result) => {
            info!(
                session_id = %session_id,
                goal_id = %goal_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                state = ?result.state,
                "Goal-lifecycle parent resume delivered via execution control"
            );
        }
        Err(error) => {
            warn!(
                session_id = %session_id,
                goal_id = %goal_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                error = %error,
                "Goal-lifecycle execution-control resume failed; continuing legacy shell adapter"
            );
        }
    }

    let resume_reason = RuntimeResumeSignal::DelegateCompleted {
        task_id: goal_id.to_string(),
        success: true,
        output: format!("Goal completed: {description}"),
    };

    if let Some(session) = active_session {
        // Deprecated compatibility boundary: local channels remain until loop_manager
        // fully subscribes to execution-control event streams (task 4.2).
        session.pause_signal.store(false, Ordering::SeqCst);
        if let Err(error) = session.resume_tx.send(resume_reason).await {
            warn!(
                session_id = %session_id,
                error = %error,
                "Failed to send legacy runtime resume signal after goal-lifecycle delivery"
            );
        }

        let resumed_payload = serde_json::json!({
            "session_id": session_id,
            "task_id": goal_id.to_string(),
            "success": true,
            "goal_completed": true,
        });
        state
            .persist
            .event_log
            .append_command(AppendEventCommand::new(
                session_id,
                "loop_resumed",
                parent_agent,
                resumed_payload.clone(),
            ))
            .await;
        let _ = session
            .sse_tx
            .read()
            .await
            .send(Ok(
                Event::default()
                    .event("loop_resumed")
                    .data(resumed_payload.to_string()),
            ))
            .await;

        let mut exec_ctx = ExecutionContext::new(
            session_id.to_string(),
            application_id.0.to_string(),
            parent_agent.to_string(),
        );
        let _ = load_module_state(
            state.sessions.framework_session_store.as_ref(),
            session_id,
            &mut exec_ctx,
        )
        .await;
        exec_ctx.mark_resumed(Some(format!("goal_completed:{goal_id}")));
        let _ = save_module_state(
            state.sessions.framework_session_store.as_ref(),
            session_id,
            &exec_ctx,
        )
        .await;

        info!(
            goal_id = %goal_id,
            session_id = %session_id,
            "Resumed parent session after goal completion via goal-lifecycle adapter"
        );
    } else {
        warn!(
            goal_id = %goal_id,
            session_id = %session_id,
            "Active session not found for legacy goal-lifecycle resume adapter"
        );
    }
}
