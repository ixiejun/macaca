//! Shell adapter for fork-join pause/resume handoff (Adapter pattern).
//!
//! Kernel `ForkManager` still emits in-process hook events that cannot yet be
//! serialized through `service.execution_control`.  This module bridges those
//! hooks into:
//! 1. Auditable `service.execution_control` resume commands (authoritative).
//! 2. Runtime-host local wake channels owned by the waiting parent loop.
//!
//! The adapter is intentionally application-neutral: it never branches on agent
//! role names or workflow labels.

use std::sync::Arc;

use macaca_host_composition::execution_control::{
    ExecutionControlForkJoinCoordinator, ExecutionControlLocalNotificationRuntime,
    ForkJoinParentResumeRequest,
};
use macaca_proto::{
    ApplicationId, ForkId, LlmRole, RuntimeResumeSignal, TaskId, TraceContext,
    EXECUTION_CONTROL_SERVICE_ID,
};
use tracing::{info, warn};

use crate::state::ForkSessionMapping;

/// Provider-neutral reason codes recorded in execution-control audit replay.
pub const REASON_FORK_JOIN_PARENT_RESUME_SUCCESS: &str =
    "execution_control.fork_join.parent_resume.success";
pub const REASON_FORK_JOIN_PARENT_RESUME_FAILURE: &str =
    "execution_control.fork_join.parent_resume.failure";

/// Deliver a fork-lifecycle resume through execution control, then wake the
/// local parent loop channel when the session is still active.
///
/// # Execution order
///
/// 1. Emit `ExecutionControlResumeCommand` via the runtime-host coordinator so
///    service-call audit evidence exists before any runtime-host local side effect.
/// 2. Ask runtime-host to clear the local `pause_signal` and push a
///    `RuntimeResumeSignal` into the local notification channel so framework
///    middleware can continue.
///
/// Failures in step 1 are logged but do not block step 2: the local wake signal
/// is non-authoritative but still required to release the waiting loop.
pub async fn deliver_fork_join_resume_and_notify_parent(
    coordinator: &ExecutionControlForkJoinCoordinator,
    mapping: &ForkSessionMapping,
    fork_id: ForkId,
    delegate_task_id: Option<TaskId>,
    reason_code: &str,
    resume_signal: RuntimeResumeSignal,
    notification_runtime: &Arc<ExecutionControlLocalNotificationRuntime>,
) {
    let trace = TraceContext::new(format!(
        "fork-join-resume:{}:{}",
        mapping.session_id, fork_id.0
    ));

    let resume_request = ForkJoinParentResumeRequest {
        application_id: mapping.app_id,
        session_id: mapping.session_id.clone(),
        parent_agent: mapping.from_agent.clone(),
        fork_id,
        delegate_task_id,
        trace,
        reason_code: reason_code.to_string(),
    };

    match coordinator.deliver_parent_fork_resume(resume_request).await {
        Ok(result) => {
            info!(
                session_id = %mapping.session_id,
                fork_id = %fork_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                reason_code = %reason_code,
                state = ?result.state,
                "Fork-join parent resume delivered via execution control"
            );
        }
        Err(error) => {
            warn!(
                session_id = %mapping.session_id,
                fork_id = %fork_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                reason_code = %reason_code,
                error = %error,
                "Fork-join execution-control resume failed; continuing runtime-host local wake"
            );
        }
    }

    // Local channels are wake handles only; execution-control remains the
    // authoritative service boundary for replayable resume evidence.
    if !notification_runtime
        .notify_resume(&mapping.session_id, resume_signal)
        .await
    {
        warn!(
            session_id = %mapping.session_id,
            fork_id = %fork_id.0,
            "Execution-control runtime-host local notification was unavailable for fork-join resume"
        );
    }
}

/// Extract assistant output from a fork snapshot for delegate completion signals.
pub fn extract_fork_assistant_output(messages: &[macaca_proto::LlmMessage]) -> String {
    messages
        .iter()
        .filter_map(|message| {
            if matches!(message.role, LlmRole::Assistant) {
                Some(message.content.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build a fork-to-session mapping used by hook consumers after delegate_task.
pub fn fork_session_mapping(
    session_id: String,
    app_id: ApplicationId,
    from_agent: String,
) -> ForkSessionMapping {
    ForkSessionMapping {
        session_id,
        app_id,
        from_agent,
    }
}
