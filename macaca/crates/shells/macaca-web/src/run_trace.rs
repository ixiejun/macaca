//! Cross-cutting run trace — sparse checkpoints for “where is the system stuck?”.
//!
//! Emits `event_type: "run_trace"` into [`EventLog`] and optional Prometheus counters.
//! Phases are plain strings so any application can add domain-specific names without
//! changing this module.

use std::sync::Arc;

use macaca_sdk::runtime_host::persist::{AppendEventCommand, EventLog};
use macaca_proto::{ApplicationId, RunTracePayload};
use serde_json::json;

/// Well-known phase names (OS-level). Apps may emit additional custom phases.
pub mod phase {
    pub const CHAT_REQUEST: &str = "chat.request";
    pub const WORKFLOW_START: &str = "workflow.start";
    pub const COORDINATOR_ITERATION: &str = "coordinator.iteration";
    pub const COORDINATOR_LOOP_PAUSED: &str = "coordinator.loop_paused";
    pub const COORDINATOR_LOOP_RESUMED: &str = "coordinator.loop_resumed";
    pub const COORDINATOR_LLM_ERROR: &str = "coordinator.llm_error";
    pub const COORDINATOR_STOPPED: &str = "coordinator.stopped";
    pub const COORDINATOR_DONE: &str = "coordinator.done";
    pub const WORKFLOW_ERROR: &str = "workflow.error";
    pub const DELEGATE_TASK_START: &str = "delegate.task_start";
    pub const DELEGATE_TASK_COMPLETE: &str = "delegate.task_complete";
    pub const DELEGATE_TASK_FAILED: &str = "delegate.task_failed";

    /// PlanLoop picked a goal for decomposition (before delegating to planner).
    pub const PLAN_GOAL_READY: &str = "plan.goal_ready";
    /// PlanLoop saw a task pending review (before delegating review prompt).
    pub const PLAN_REVIEW_NEEDED: &str = "plan.review_needed";
    /// PlanLoop delegated review work to the plan agent.
    pub const PLAN_REVIEW_DELEGATE: &str = "plan.review_delegate";
    pub const PLAN_ANOMALY: &str = "plan.anomaly";
    pub const PLAN_EVALUATE_GOAL: &str = "plan.evaluate_goal";
    pub const PLAN_GOAL_COMPLETED: &str = "plan.goal_completed";
    pub const PLAN_ALL_TASKS_DONE: &str = "plan.all_tasks_done";
    pub const PLAN_LOOP_STARTED: &str = "plan.loop_started";
    pub const PLAN_GOAL_DELEGATE: &str = "plan.goal_delegate";
    pub const PLAN_GOAL_SATISFIED: &str = "plan.goal_satisfied";
    pub const PLAN_GOAL_NEEDS_WORK: &str = "plan.goal_needs_work";
    pub const PLAN_GOAL_EVAL_FALLBACK: &str = "plan.goal_eval_fallback";

    pub const GOAL_CREATE_HTTP: &str = "goal.create_http";
    pub const GOAL_CREATE_TOOL: &str = "goal.create_tool";

    pub const WORKER_TASK_CLAIMED: &str = "worker.task_claimed";
    pub const WORKER_DELEGATE_START: &str = "worker.delegate_start";
    pub const WORKER_TASK_SUCCESS: &str = "worker.task_success";
    pub const WORKER_TASK_FAILED: &str = "worker.task_failed";
    pub const WORKER_SUBMIT_REVIEW: &str = "worker.submit_review";
    pub const WORKER_RETRY_START: &str = "worker.retry_start";
    pub const WORKER_DELEGATE_ERROR: &str = "worker.delegate_error";

    /// Planner called `review_todo` and the store accepted it.
    pub const TODO_REVIEW_DECIDED: &str = "todo.review_decided";
}

pub mod status {
    pub const OK: &str = "ok";
    pub const ERROR: &str = "error";
    pub const WAITING: &str = "waiting";
    pub const INFO: &str = "info";
}

/// Event-log key: real session, or synthetic `_macaca_app_{uuid}` when scope is app-wide.
pub fn trace_session_key(session_id: Option<&str>, app_id: &ApplicationId) -> String {
    session_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("_macaca_app_{}", app_id.0))
}

/// Emit a checkpoint under the best-known session scope (fallback app bucket).
pub async fn emit_for_scope(
    tracer: &RunTracer,
    session_id: Option<&str>,
    app_id: &ApplicationId,
    phase: &str,
    component: &str,
    st: &str,
    message: Option<String>,
    task_id: Option<String>,
    goal_id: Option<String>,
    extra: Option<serde_json::Value>,
) {
    let sid = trace_session_key(session_id, app_id);
    tracer
        .emit(&sid, phase, component, st, message, task_id, goal_id, extra)
        .await;
}

/// Emits structured `run_trace` rows into the session event log.
#[derive(Clone)]
pub struct RunTracer {
    log: Arc<EventLog>,
}

impl RunTracer {
    pub fn new(log: Arc<EventLog>) -> Self {
        Self { log }
    }

    /// Record a checkpoint. Safe to call from hot paths; failures are swallowed after metrics.
    pub async fn emit(
        &self,
        session_id: &str,
        phase: &str,
        component: &str,
        st: &str,
        message: Option<String>,
        task_id: Option<String>,
        goal_id: Option<String>,
        extra: Option<serde_json::Value>,
    ) {
        crate::metrics::record_run_trace(phase, st);

        let payload = RunTracePayload {
            phase: phase.to_string(),
            component: component.to_string(),
            status: st.to_string(),
            message,
            task_id,
            goal_id,
            extra,
        };
        let v = serde_json::to_value(&payload).unwrap_or_else(|_| json!({}));
        self.log
            .append_command(AppendEventCommand::new(
                session_id,
                "run_trace",
                "run_tracer",
                v,
            ))
            .await;
    }
}
