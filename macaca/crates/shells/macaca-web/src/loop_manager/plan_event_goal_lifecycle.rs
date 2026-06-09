//! PlanEvent handlers for goal evaluation and coordinator resume.
//!
//! Goal evaluation delegates to the planning agent via agent-execution service.
//! Goal completion routes coordinator resume through execution_control + the
//! goal-lifecycle shell adapter (audit-first, legacy channel compat).

use std::sync::Arc;

use axum::response::sse::Event;
use macaca_proto::TaskId;
use macaca_sdk::task::TaskSummary;

use super::agent_execution_adapter::{run_planner_framework_call, PlannerFrameworkCallKind};
use super::plan_event_context::PlanEventConsumerCtx;
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};

pub(crate) async fn handle_plan_event_evaluate_goal(
    ctx: &PlanEventConsumerCtx,
    goal_id: TaskId,
    goal_description: String,
    completed_count: usize,
    failed_count: usize,
    task_summaries: Vec<TaskSummary>,
    session_id: Option<String>,
) {
tracing::info!(goal_id = %goal_id, "Evaluating goal completion");
crate::run_trace::emit_for_scope(
    &ctx.state.persist.run_tracer,
    session_id.as_deref(),
    &ctx.app_id,
    crate::run_trace::phase::PLAN_EVALUATE_GOAL,
    "plan_loop",
    crate::run_trace::status::INFO,
    Some(format!("completed={completed_count} failed={failed_count}")),
    None,
    Some(goal_id.to_string()),
    Some(serde_json::json!({
        "description": goal_description.chars().take(200).collect::<String>(),
    })),
)
.await;
let evaluation_prompt = macaca_sdk::task::GoalEvaluator::build_prompt(
    &goal_description,
    &task_summaries,
    completed_count,
    failed_count,
);
let evaluation_result = run_planner_framework_call(
    &ctx.state,
    &ctx.app_id,
    &ctx.plan_agent_name,
    session_id.clone(),
    goal_id,
    evaluation_prompt,
    format!(
        "Evaluating goal completion: {}",
        goal_description.chars().take(80).collect::<String>()
    ),
    PlannerFrameworkCallKind::GoalEvaluation,
)
.await
.map(|reply| {
    macaca_sdk::task::GoalEvaluator::parse_eval_response(&reply)
});
match evaluation_result {
    Ok(macaca_sdk::task::GoalEvaluation::Satisfied { summary }) => {
        tracing::info!(goal_id = %goal_id, summary = %summary, "Goal satisfied");
        let space = macaca_sdk::task::TaskSpace::for_session(
            ctx.app_id.clone(),
            None,
            Arc::clone(&ctx.state.persist.todo_store),
        );
        space.complete_goal(&goal_id).await;
        let _ = ctx.event_tx
            .send(macaca_sdk::task::PlanEvent::GoalCompleted {
                goal_id: goal_id.clone(),
                description: goal_description.clone(),
            })
            .await;
        // Emit SSE decision event
        let msg = format!("Goal completed: {}", summary);
        let plan_payload = serde_json::json!({
            "decision_type": "goal_satisfied",
            "goal_id": goal_id.to_string(),
            "description": goal_description,
            "summary": summary,
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
                decision_type: "goal_satisfied".into(),
                message: msg,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "summary": summary }),
            }).await;
        crate::run_trace::emit_for_scope(
            &ctx.state.persist.run_tracer,
            session_id.as_deref(),
            &ctx.app_id,
            crate::run_trace::phase::PLAN_GOAL_SATISFIED,
            "plan_loop",
            crate::run_trace::status::OK,
            Some(summary.chars().take(200).collect::<String>()),
            None,
            Some(goal_id.to_string()),
            None,
        )
        .await;
    }
    Ok(macaca_sdk::task::GoalEvaluation::NeedsMoreWork {
        reason,
        suggestions,
    }) => {
        tracing::info!(goal_id = %goal_id, reason = %reason, "Goal needs more work");
        {
            let prompt = format!(
                "The goal '{}' needs additional work. Reason: {}\nSuggestions:\n{}\n\nCreate follow-up tasks using create_todo.",
                goal_description, reason,
                suggestions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
            );
            let _ = run_planner_framework_call(
                &ctx.state,
                &ctx.app_id,
                &ctx.plan_agent_name,
                session_id.clone(),
                goal_id,
                prompt,
                format!(
                    "Planning follow-up work: {}",
                    reason.chars().take(80).collect::<String>()
                ),
                PlannerFrameworkCallKind::FollowUp,
            )
            .await;
        }
        // Emit SSE decision event
        let msg = format!("Goal needs more work: {}", reason);
        let plan_payload = serde_json::json!({
            "decision_type": "goal_needs_work",
            "goal_id": goal_id.to_string(),
            "description": goal_description,
            "reason": reason,
            "suggestions": suggestions,
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
                decision_type: "goal_needs_work".into(),
                message: msg,
                timestamp: chrono::Utc::now(),
                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "reason": reason, "suggestions": suggestions }),
            }).await;
        crate::run_trace::emit_for_scope(
            &ctx.state.persist.run_tracer,
            session_id.as_deref(),
            &ctx.app_id,
            crate::run_trace::phase::PLAN_GOAL_NEEDS_WORK,
            "plan_loop",
            crate::run_trace::status::INFO,
            Some(reason.clone()),
            None,
            Some(goal_id.to_string()),
            None,
        )
        .await;
    }
    Err(e) => {
        tracing::warn!(error = %e, "GoalEvaluator failed, marking complete by default");
        crate::run_trace::emit_for_scope(
            &ctx.state.persist.run_tracer,
            session_id.as_deref(),
            &ctx.app_id,
            crate::run_trace::phase::PLAN_GOAL_EVAL_FALLBACK,
            "plan_loop",
            crate::run_trace::status::ERROR,
            Some(e.clone()),
            None,
            Some(goal_id.to_string()),
            None,
        )
        .await;
        let space = macaca_sdk::task::TaskSpace::for_session(
            ctx.app_id.clone(),
            None,
            Arc::clone(&ctx.state.persist.todo_store),
        );
        space.complete_goal(&goal_id).await;
    }
}
}

pub(crate) async fn handle_plan_event_goal_completed(
    ctx: &PlanEventConsumerCtx,
    goal_id: TaskId,
    description: String,
) {
tracing::info!(goal_id = %goal_id, "Goal completed: {}", description);

// ===== RESUME COORDINATOR IF PAUSED =====
let goal_id_str = goal_id.to_string();
let mut gts =
    ctx.state.sessions.goal_to_session.write().await;
let trace_sid = gts.get(&goal_id_str).cloned();
let waiting_session = gts.remove(&goal_id_str);
drop(gts);

crate::run_trace::emit_for_scope(
    &ctx.state.persist.run_tracer,
    trace_sid.as_deref(),
    &ctx.app_id,
    crate::run_trace::phase::PLAN_GOAL_COMPLETED,
    "plan_loop",
    crate::run_trace::status::OK,
    Some(description.chars().take(200).collect::<String>()),
    None,
    Some(goal_id_str.clone()),
    None,
)
.await;

if let Some(sid) = waiting_session {
    // PlanLoop goal completion is a task-service lifecycle signal.
    // Route resume through execution_control (authoritative) and
    // the goal-lifecycle shell adapter (legacy channel compat).
    let goal_coordinator =
        macaca_sdk::runtime_host::ExecutionControlGoalLifecycleCoordinator::new(
            Arc::clone(&ctx.state.service_runtime),
        );
    let sessions =
        ctx.state.sessions.active_sessions.read().await;
    let active_session = sessions.get(&sid);
    crate::goal_lifecycle_shell_adapter::deliver_goal_resume_and_notify_parent(
        &ctx.state,
        &goal_coordinator,
        &ctx.app_id,
        &sid,
        &ctx.entry_agent_name,
        goal_id,
        &description,
        active_session,
    )
    .await;
} else {
    tracing::warn!(
        goal_id = %goal_id,
        app_id = %ctx.app_id,
        "Goal completed but no exact paused coordinator mapping was found"
    );
}

// Emit SSE decision event
let msg = format!("Goal completed: {}", description);
let plan_payload = serde_json::json!({
    "decision_type": "goal_completed",
    "goal_id": goal_id.to_string(),
    "description": description,
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
        decision_type: "goal_completed".into(),
        message: msg,
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": description }),
    })
    .await;
}
