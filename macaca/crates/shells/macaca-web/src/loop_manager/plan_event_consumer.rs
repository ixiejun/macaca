//! PlanEvent consumer dispatch loop (Strategy registry).
//!
//! Spawns alongside macaca-task PlanLoop and routes each event to a dedicated
//! handler module. Keeps `plan_loop_orchestrator` focused on startup only.

use macaca_task::PlanEvent;

use super::plan_event_context::PlanEventConsumerCtx;
use super::plan_event_goal_lifecycle::{
    handle_plan_event_evaluate_goal, handle_plan_event_goal_completed,
};
use super::plan_event_goal_ready::handle_plan_event_goal_ready;
use super::plan_event_handlers::{
    handle_plan_event_all_tasks_done, handle_plan_event_anomaly,
    handle_plan_event_review_needed,
};

/// Dispatch one `PlanEvent` to the appropriate Strategy handler.
pub(crate) async fn dispatch_plan_event(ctx: &PlanEventConsumerCtx, event: PlanEvent) {
    match event {
        PlanEvent::GoalReady { goal_id, description, session_id } => {
            handle_plan_event_goal_ready(ctx, goal_id, description, session_id).await;
        }
        PlanEvent::ReviewNeeded { task_id, agent, title, summary, criteria, session_id } => {
            handle_plan_event_review_needed(
                ctx, task_id, agent, title, summary, criteria, session_id,
            )
            .await;
        }
        PlanEvent::AllTasksDone { completed, failed } => {
            handle_plan_event_all_tasks_done(ctx, completed, failed).await;
        }
        PlanEvent::AnomalyDetected { message } => {
            handle_plan_event_anomaly(ctx, message).await;
        }
        PlanEvent::EvaluateGoalCompletion {
            goal_id,
            goal_description,
            completed_count,
            failed_count,
            task_summaries,
            session_id,
        } => {
            handle_plan_event_evaluate_goal(
                ctx,
                goal_id,
                goal_description,
                completed_count,
                failed_count,
                task_summaries,
                session_id,
            )
            .await;
        }
        PlanEvent::GoalCompleted { goal_id, description } => {
            handle_plan_event_goal_completed(ctx, goal_id, description).await;
        }
    }
}
