//! Shell adapter for session-loop wake/register/shutdown handoff (Adapter pattern).
//!
//! PlanLoop and WorkerLoop still run as in-process `macaca-task` controllers with
//! local `PlanLoopWaker` / `WorkerLoopWaker` handles stored on `AppState`.  This
//! module bridges task-service lifecycle signals into:
//! 1. Auditable `service.execution_control` checkpoint/register commands (authoritative).
//! 2. Legacy in-process waker maps owned by the web shell (compat seam).
//!
//! The adapter is intentionally application-neutral: it never branches on agent
//! role names or workflow labels.

use std::sync::Arc;

use macaca_proto::{ApplicationId, EXECUTION_CONTROL_SERVICE_ID, TraceContext};
use macaca_sdk::runtime_host::{
    ExecutionControlSessionLoopCoordinator, SessionLoopKind, SessionLoopRegisterRequest,
    SessionLoopShutdownRequest, SessionLoopWakeRequest,
    SESSION_LOOP_PLAN_WAKE_EVENT, SESSION_LOOP_TASK_CAPABILITY_ID,
    SESSION_LOOP_WORKER_WAKE_EVENT,
};
use macaca_sdk::task::{PlanLoopWaker, WorkerLoopWaker};
use tracing::{info, warn};

use crate::state::AppState;

/// Provider-neutral reason codes recorded in execution-control audit replay.
pub const REASON_SESSION_LOOP_GOAL_DECOMPOSITION_READY: &str =
    "execution_control.session_loop.goal_decomposition_ready";
pub const REASON_SESSION_LOOP_WORKER_SUBMIT_REVIEW: &str =
    "execution_control.session_loop.worker_submit_review";
pub const REASON_SESSION_LOOP_REVIEW_DELEGATE_COMPLETE: &str =
    "execution_control.session_loop.review_delegate_complete";
pub const REASON_SESSION_LOOP_APPLICATION_CLEANUP: &str =
    "execution_control.session_loop.application_cleanup";

/// Register a PlanLoop controller through execution control when loops are spawned.
///
/// Failures are logged but do not block local loop startup: the shell must keep
/// functioning while execution-control migration is in progress.
pub async fn register_plan_loop_via_execution_control(
    coordinator: &ExecutionControlSessionLoopCoordinator,
    application_id: ApplicationId,
    session_id: Option<String>,
) {
    let trace = TraceContext::new(format!(
        "session-loop-register-plan:{}",
        application_id.0
    ));
    let request = SessionLoopRegisterRequest {
        application_id,
        session_id,
        loop_kind: SessionLoopKind::Plan,
        worker_count: None,
        trace,
    };

    match coordinator.register_session_loop(request).await {
        Ok(result) => {
            info!(
                application_id = %application_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                capability_id = SESSION_LOOP_TASK_CAPABILITY_ID,
                state = ?result.state,
                "Plan loop registered via execution control"
            );
        }
        Err(error) => {
            warn!(
                application_id = %application_id.0,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                error = %error,
                "Plan loop execution-control registration failed; continuing local loop startup"
            );
        }
    }
}

/// Register WorkerLoop controllers through execution control when loops are spawned.
pub async fn register_worker_loops_via_execution_control(
    coordinator: &ExecutionControlSessionLoopCoordinator,
    application_id: ApplicationId,
    session_id: Option<String>,
    worker_count: usize,
) {
    let trace = TraceContext::new(format!(
        "session-loop-register-worker:{}",
        application_id.0
    ));
    let request = SessionLoopRegisterRequest {
        application_id,
        session_id,
        loop_kind: SessionLoopKind::Worker,
        worker_count: Some(worker_count),
        trace,
    };

    match coordinator.register_session_loop(request).await {
        Ok(result) => {
            info!(
                application_id = %application_id.0,
                worker_count,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                capability_id = SESSION_LOOP_TASK_CAPABILITY_ID,
                state = ?result.state,
                "Worker loops registered via execution control"
            );
        }
        Err(error) => {
            warn!(
                application_id = %application_id.0,
                worker_count,
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                error = %error,
                "Worker loop execution-control registration failed; continuing local loop startup"
            );
        }
    }
}

/// Record a plan-loop wake through execution control, then wake the local controller.
pub async fn wake_plan_loop_and_notify_local(
    state: &Arc<AppState>,
    coordinator: &ExecutionControlSessionLoopCoordinator,
    application_id: &ApplicationId,
    session_id: Option<&str>,
    reason_code: &str,
    detail: Option<String>,
) {
    let trace = TraceContext::new(format!(
        "session-loop-wake-plan:{}",
        application_id.0
    ));
    let wake_request = SessionLoopWakeRequest {
        application_id: *application_id,
        session_id: session_id.map(str::to_string),
        loop_kind: SessionLoopKind::Plan,
        trace,
        reason_code: reason_code.to_string(),
        detail,
    };

    if let Err(error) = coordinator.record_loop_wake(wake_request).await {
        warn!(
            application_id = %application_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            event_kind = SESSION_LOOP_PLAN_WAKE_EVENT,
            reason_code = %reason_code,
            error = %error,
            "Plan loop execution-control wake failed; continuing legacy shell adapter"
        );
    }

    if let Some(waker) = state.loops.plan_loop_wakers.read().await.get(application_id) {
        waker.wake();
    }
}

/// Record worker-loop wake(s) through execution control, then wake local controllers.
pub async fn wake_worker_loops_and_notify_local(
    state: &Arc<AppState>,
    coordinator: &ExecutionControlSessionLoopCoordinator,
    application_id: &ApplicationId,
    session_id: Option<&str>,
    reason_code: &str,
    detail: Option<String>,
) {
    let trace = TraceContext::new(format!(
        "session-loop-wake-worker:{}",
        application_id.0
    ));
    let wake_request = SessionLoopWakeRequest {
        application_id: *application_id,
        session_id: session_id.map(str::to_string),
        loop_kind: SessionLoopKind::Worker,
        trace,
        reason_code: reason_code.to_string(),
        detail,
    };

    if let Err(error) = coordinator.record_loop_wake(wake_request).await {
        warn!(
            application_id = %application_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            event_kind = SESSION_LOOP_WORKER_WAKE_EVENT,
            reason_code = %reason_code,
            error = %error,
            "Worker loop execution-control wake failed; continuing legacy shell adapter"
        );
    }

    if let Some(wakers) = state.loops.worker_loop_wakers.read().await.get(application_id) {
        for waker in wakers {
            waker.wake();
        }
    }
}

/// Record session-loop shutdown through execution control during application cleanup.
pub async fn shutdown_session_loops_via_execution_control(
    coordinator: &ExecutionControlSessionLoopCoordinator,
    application_id: &ApplicationId,
    reason_code: &str,
) {
    let trace = TraceContext::new(format!(
        "session-loop-shutdown:{}",
        application_id.0
    ));
    let request = SessionLoopShutdownRequest {
        application_id: *application_id,
        session_id: None,
        trace,
        reason_code: reason_code.to_string(),
    };

    if let Err(error) = coordinator.record_session_loop_shutdown(request).await {
        warn!(
            application_id = %application_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            reason_code = %reason_code,
            error = %error,
            "Session-loop shutdown execution-control recording failed; continuing local cleanup"
        );
    }
}

/// Type alias documenting the legacy local waker seam retained by the shell.
pub type LegacyPlanLoopWaker = PlanLoopWaker;

/// Type alias documenting the legacy local waker seam retained by the shell.
pub type LegacyWorkerLoopWaker = WorkerLoopWaker;
