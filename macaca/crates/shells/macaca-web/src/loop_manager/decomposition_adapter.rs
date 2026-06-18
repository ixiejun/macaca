//! Goal decomposition fallback chain and decomposition status transitions.
//!
//! Web is only a shell adapter here. Planner failure recovery is requested
//! through the Task Service fallback-decomposition command, and every task-board
//! mutation still flows through `task.create_assignment`. This keeps fallback
//! planning rules service-owned, traceable, and replaceable.

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, BuildFallbackDecompositionCommand, CreateTaskAssignmentCommand,
    FallbackDecompositionWorkerProfile, TraceContext,
};
use macaca_sdk::ServiceBackedTaskBoardDataSource;

use super::agent_execution_adapter::list_goal_todos_for_scope;
use super::execution_control_adapter::session_loop_coordinator;
use crate::session_loop_shell_adapter::{
    wake_worker_loops_and_notify_local, REASON_SESSION_LOOP_GOAL_DECOMPOSITION_READY,
};
use crate::state::AppState;

/// Wake worker loops after recording an auditable execution-control checkpoint.
pub(crate) async fn wake_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    reason_code: &str,
    detail: Option<String>,
) {
    let coordinator = session_loop_coordinator(state);
    wake_worker_loops_and_notify_local(
        state,
        &coordinator,
        app_id,
        session_id,
        reason_code,
        detail,
    )
    .await;
}

pub(crate) async fn mark_goal_decomposition_ready(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    task_count: usize,
) {
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::InProgress)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_ready",
        "plan_loop",
        crate::run_trace::status::OK,
        Some(format!("tasks={task_count}")),
        None,
        Some(goal_id.to_string()),
        None,
    )
    .await;
    wake_worker_loops(
        state,
        app_id,
        session_id,
        REASON_SESSION_LOOP_GOAL_DECOMPOSITION_READY,
        Some(format!("tasks={task_count}")),
    )
    .await;
}

pub(crate) async fn mark_goal_decomposition_failed(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    error: &str,
) {
    let mut cancelled = 0usize;
    let mut tasks = list_goal_todos_for_scope(state, app_id, session_id, goal_id).await;
    for task in &mut tasks {
        if matches!(
            task.status,
            macaca_proto::TodoStatus::Pending
                | macaca_proto::TodoStatus::Blocked
                | macaca_proto::TodoStatus::Assigned
        ) {
            task.status = macaca_proto::TodoStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
            state.persist.todo_store.save_todo(task).await;
            cancelled += 1;
        }
    }
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::Failed)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_failed",
        "plan_loop",
        crate::run_trace::status::ERROR,
        Some(error.chars().take(200).collect::<String>()),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({ "cancelled_partial_todos": cancelled })),
    )
    .await;
}

#[derive(Clone, Debug)]
pub(crate) struct PlannerWorkerDossier {
    pub(crate) name: String,
    pub(crate) capabilities: Vec<String>,
}

pub(crate) async fn create_fallback_decomposition_tasks(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    plan_agent_name: &str,
    goal_description: &str,
    workers: &[PlannerWorkerDossier],
    initial_dependency: Option<macaca_proto::TaskId>,
    planner_error: &str,
) -> Vec<macaca_proto::TodoItem> {
    let task_client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let mut planning_trace = TraceContext::new(format!(
        "task-fallback-plan-{}-{}",
        goal_id,
        uuid::Uuid::new_v4()
    ));
    planning_trace.session_id = session_id.map(str::to_string);
    planning_trace.task_id = Some(goal_id.to_string());
    let plan = match task_client
        .build_fallback_decomposition(BuildFallbackDecompositionCommand {
            app_id: app_id.clone(),
            session_id: session_id.map(str::to_string),
            goal_id,
            goal_description: goal_description.to_string(),
            workers: workers
                .iter()
                .cloned()
                .map(|worker| FallbackDecompositionWorkerProfile {
                    name: worker.name,
                    capabilities: worker.capabilities,
                })
                .collect(),
            initial_dependency,
            planner_error: planner_error.to_string(),
            trace: Some(planning_trace),
        })
        .await
    {
        Ok(plan) => plan,
        Err(error) => {
            tracing::error!(
                goal_id = %goal_id,
                error = %error,
                "task service failed to build fallback decomposition plan"
            );
            return Vec::new();
        }
    };
    if plan.assignments.is_empty() {
        return Vec::new();
    }

    tracing::warn!(
        goal_id = %goal_id,
        planner_error = %planner_error,
        fallback_tasks = plan.assignments.len(),
        "Planner produced no todos; task service returned fallback task chain"
    );

    let mut created = Vec::new();
    let mut previous: Option<macaca_proto::TaskId> = None;

    for (index, assignment) in plan.assignments.into_iter().enumerate() {
        let mut depends_on = assignment.depends_on;
        if let Some(previous_task_id) = previous {
            depends_on = vec![previous_task_id];
        }
        let mut trace = TraceContext::new(format!(
            "task-fallback-assignment-{}-{}",
            goal_id,
            uuid::Uuid::new_v4()
        ));
        trace.session_id = session_id.map(str::to_string);
        trace.task_id = Some(goal_id.to_string());
        let item = match task_client
            .create_task_assignment(CreateTaskAssignmentCommand {
                app_id: app_id.clone(),
                session_id: session_id.map(str::to_string),
                agent_name: assignment.agent_name.clone(),
                created_by: plan_agent_name.to_string(),
                title: assignment.title,
                description: assignment.description,
                acceptance_criteria: assignment.acceptance_criteria,
                priority: assignment.priority,
                depends_on,
                parent_task: Some(goal_id),
                // Fallback tasks are generic application-execution records owned by Task Service.
                graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
                graph_id: None,
                trace: Some(trace),
            })
            .await
        {
            Ok(item) => item,
            Err(error) => {
                tracing::error!(
                    goal_id = %goal_id,
                    agent = %assignment.agent_name,
                    error = %error,
                    "task service failed to create fallback decomposition task"
                );
                break;
            }
        };
        previous = Some(item.id);
        created.push(item);
    }

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_fallback_ready",
        "plan_loop",
        crate::run_trace::status::INFO,
        Some(format!(
            "planner_error={}; fallback_tasks={}",
            planner_error.chars().take(160).collect::<String>(),
            created.len()
        )),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({
            "task_count": created.len(),
            "planner_error": planner_error,
            "agents": created.iter().map(|task| task.assigned_agent.clone()).collect::<Vec<_>>(),
        })),
    )
    .await;

    created
}

pub(crate) fn terminal_goal_task(tasks: &[macaca_proto::TodoItem]) -> Option<macaca_proto::TaskId> {
    let dependency_ids = tasks
        .iter()
        .flat_map(|task| task.depends_on.iter().copied())
        .collect::<std::collections::HashSet<_>>();
    tasks
        .iter()
        .rev()
        .find(|task| !dependency_ids.contains(&task.id))
        .map(|task| task.id)
        .or_else(|| tasks.last().map(|task| task.id))
}
