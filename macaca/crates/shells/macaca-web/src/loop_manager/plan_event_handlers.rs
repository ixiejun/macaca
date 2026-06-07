//! PlanEvent handlers for review, anomaly, and batch completion signals.
//!
//! These handlers implement the Strategy pattern: each variant owns its side
//! effects (trace, planner delegation, SSE) without bloating the consumer loop.

use axum::response::sse::Event;
use macaca_proto::TaskId;

use super::agent_execution_adapter::{run_planner_framework_call, PlannerFrameworkCallKind};
use super::execution_control_adapter::session_loop_coordinator;
use super::plan_event_context::PlanEventConsumerCtx;
use super::planner_helpers::planner_notebook_mark_review;
use crate::session_loop_shell_adapter::{
    wake_worker_loops_and_notify_local, REASON_SESSION_LOOP_REVIEW_DELEGATE_COMPLETE,
};
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};

pub(crate) async fn handle_plan_event_review_needed(
    ctx: &PlanEventConsumerCtx,
    task_id: TaskId,
    agent: String,
    title: String,
    summary: String,
    criteria: Vec<String>,
    session_id: Option<String>,
) {
crate::run_trace::emit_for_scope(
    &ctx.state.persist.run_tracer,
    session_id.as_deref(),
    &ctx.app_id,
    crate::run_trace::phase::PLAN_REVIEW_NEEDED,
    "plan_loop",
    crate::run_trace::status::WAITING,
    Some(format!("title={title} agent={agent}")),
    Some(task_id.to_string()),
    None,
    None,
)
.await;
planner_notebook_mark_review(
    &ctx.state,
    &ctx.app_id,
    session_id.as_deref(),
    task_id,
    &title,
)
.await;
{
    let prompt = format!(
        "Review this completed task using review_todo:\n\
         Task ID: {}\n Agent: {}\n Title: {}\n\
         Summary: {}\n Criteria: {:?}\n\n\
         Verify the work meets the criteria. Use review_todo with passed=true/false and feedback.",
        task_id, agent, title, summary, criteria
    );
    let _ = run_planner_framework_call(
        &ctx.state,
        &ctx.app_id,
        &ctx.plan_agent_name,
        session_id.clone(),
        task_id,
        prompt,
        format!("Reviewing {} task: {}", agent, title),
        PlannerFrameworkCallKind::Review,
    )
    .await;
    crate::run_trace::emit_for_scope(
        &ctx.state.persist.run_tracer,
        session_id.as_deref(),
        &ctx.app_id,
        crate::run_trace::phase::PLAN_REVIEW_DELEGATE,
        "plan_loop",
        crate::run_trace::status::INFO,
        Some("delegated_review_to_planner".into()),
        Some(task_id.to_string()),
        None,
        None,
    )
    .await;
    // Review delegate completed — wake worker loops via execution control.
    let coordinator = session_loop_coordinator(&ctx.state);
    wake_worker_loops_and_notify_local(
        &ctx.state,
        &coordinator,
        &ctx.app_id,
        session_id.as_deref(),
        REASON_SESSION_LOOP_REVIEW_DELEGATE_COMPLETE,
        Some(task_id.to_string()),
    )
    .await;
    // Broadcast review result as SSE + EventLog
    let review_payload = serde_json::json!({
        "decision_type": "task_reviewed",
        "task_id": task_id.to_string(),
        "agent": agent,
        "title": title,
        "message": format!("Task '{}' reviewed for agent {}", title, agent),
    });
    let review_event = Event::default()
        .event("plan_decision")
        .data(review_payload.to_string());
    broadcast_to_app_sessions(
        &ctx.state,
        &ctx.app_id,
        review_event,
        review_payload,
    )
    .await;
}
// Emit SSE decision event
let msg =
    format!("Task '{}' submitted for review by {}", title, agent);
let plan_payload = serde_json::json!({
    "decision_type": "review_needed",
    "task_id": task_id.to_string(),
    "agent": agent,
    "title": title,
    "message": msg,
});
let sse_event = Event::default()
    .event("plan_decision")
    .data(plan_payload.to_string());
broadcast_to_app_sessions(
    &ctx.state,
    &ctx.app_id,
    sse_event,
    plan_payload,
)
.await;
// Persist decision
save_plan_decision(&ctx.session_store, &ctx.app_id, PlanDecisionEvent {
        decision_type: "review_needed".into(),
        message: msg,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({ "task_id": task_id.to_string(), "agent": agent, "title": title }),
    }).await;
}

pub(crate) async fn handle_plan_event_all_tasks_done(
    ctx: &PlanEventConsumerCtx,
    completed: usize,
    failed: usize,
) {
tracing::info!(completed, failed, "All tasks done for app");
crate::run_trace::emit_for_scope(
    &ctx.state.persist.run_tracer,
    None,
    &ctx.app_id,
    crate::run_trace::phase::PLAN_ALL_TASKS_DONE,
    "plan_loop",
    crate::run_trace::status::INFO,
    Some(format!("completed={completed} failed={failed}")),
    None,
    None,
    Some(serde_json::json!({ "completed": completed, "failed": failed })),
)
.await;
}

pub(crate) async fn handle_plan_event_anomaly(
    ctx: &PlanEventConsumerCtx,
    message: String,
) {
tracing::warn!(message, "Plan loop anomaly detected");
crate::run_trace::emit_for_scope(
    &ctx.state.persist.run_tracer,
    None,
    &ctx.app_id,
    crate::run_trace::phase::PLAN_ANOMALY,
    "plan_loop",
    crate::run_trace::status::ERROR,
    Some(message.clone()),
    None,
    None,
    None,
)
.await;
ctx.state
    .config
    .alert_manager
    .fire(macaca_sdk::kernel::alert::Alert::warning(
        "Task Anomaly",
        message.clone(),
        "plan_loop",
    ))
    .await;
// Emit SSE decision event
let msg = message.clone();
let plan_payload = serde_json::json!({
    "decision_type": "anomaly",
    "message": msg,
});
let sse_event = Event::default()
    .event("plan_decision")
    .data(plan_payload.to_string());
broadcast_to_app_sessions(
    &ctx.state,
    &ctx.app_id,
    sse_event,
    plan_payload,
)
.await;
// Persist decision
save_plan_decision(
    &ctx.session_store,
    &ctx.app_id,
    PlanDecisionEvent {
        decision_type: "anomaly".into(),
        message: msg.clone(),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({ "message": msg }),
    },
)
.await;
}
