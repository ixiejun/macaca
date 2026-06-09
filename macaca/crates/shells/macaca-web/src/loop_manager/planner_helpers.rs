//! Planner notebook persistence and capability-driven agent selection helpers.
//!
//! Keeps PlanLoop consumers free of direct framework session-store details while
//! recording decomposition/review milestones in `PlanNotebook` module state.

use std::sync::Arc;

use macaca_proto::ApplicationId;
use macaca_sdk::framework::plan::PlanNotebook;
use macaca_sdk::runtime_host::executor::{ExecutorEvent, ExecutorEventFactory};
use macaca_sdk::runtime_host::AgentInfo;

use crate::state::AppState;

pub(crate) fn planner_scope_session_id(app_id: &ApplicationId, session_id: Option<&str>) -> String {
    session_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("_macaca_app_{}", app_id.0))
}

async fn persist_planner_notebook_update(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    update: impl FnOnce(&mut PlanNotebook),
) {
    let sid = planner_scope_session_id(app_id, session_id);
    let mut notebook = crate::framework_state_memento::load_plan_notebook(
        state.sessions.framework_session_store.as_ref(),
        &app_id.0.to_string(),
        &sid,
    )
    .await;

    update(&mut notebook);

    crate::framework_state_memento::save_plan_notebook(
        state.sessions.framework_session_store.as_ref(),
        &app_id.0.to_string(),
        &sid,
        &notebook,
    )
    .await;
}

pub(crate) fn select_entry_and_plan_agents(
    agents: &[AgentInfo],
    manifest_entry: Option<&str>,
) -> (String, String) {
    let entry = manifest_entry
        .map(str::to_string)
        .or_else(|| agents.first().map(|a| a.name.clone()))
        .unwrap_or_else(|| "entry_agent".to_string());
    let planner = agents
        .iter()
        .find(|a| a.capabilities.iter().any(|c| c == "task_planning"))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| entry.clone());
    (entry, planner)
}

pub(crate) fn mark_decomposition_in_notebook(
    notebook: &mut PlanNotebook,
    goal_id: macaca_proto::TaskId,
    description: &str,
) {
    notebook.create_plan(
        format!("goal:{}", goal_id),
        description.to_string(),
        "Decompose goal into executable todos",
    );
    if let Some(plan_mut) = notebook.current_plan_mut() {
        plan_mut.add_subtask(
            "decompose_goal",
            format!("Decompose goal {}", goal_id),
            "Todos created and persisted to TodoBoard",
        );
        let _ = plan_mut.start_subtask(0);
        let _ = plan_mut.finish_subtask(0, "decomposition delegated to planner");
    }
    let _ = notebook.finish_plan(format!("goal {} decomposition recorded", goal_id));
}

pub(crate) async fn planner_notebook_mark_decomposition(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    description: &str,
) {
    persist_planner_notebook_update(state, app_id, session_id, |notebook| {
        mark_decomposition_in_notebook(notebook, goal_id, description);
    })
    .await;
}

pub(crate) fn mark_review_in_notebook(
    notebook: &mut PlanNotebook,
    task_id: macaca_proto::TaskId,
    task_title: &str,
) {
    notebook.create_plan(
        format!("review:{}", task_id),
        format!("Review task '{}'", task_title),
        "Task review decision persisted via review_todo",
    );
    if let Some(plan_mut) = notebook.current_plan_mut() {
        plan_mut.add_subtask(
            "review_todo",
            format!("Review todo {}", task_id),
            "Todo status updated to completed/needs_optimization/failed",
        );
        let _ = plan_mut.start_subtask(0);
        let _ = plan_mut.finish_subtask(0, "review delegated to planner");
    }
    let _ = notebook.finish_plan(format!("task {} review recorded", task_id));
}

pub(crate) async fn planner_notebook_mark_review(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    task_id: macaca_proto::TaskId,
    task_title: &str,
) {
    persist_planner_notebook_update(state, app_id, session_id, |notebook| {
        mark_review_in_notebook(notebook, task_id, task_title);
    })
    .await;
}

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

pub(crate) fn executor_task_started(task_id: macaca_proto::TaskId, agent: &str) -> ExecutorEvent {
    ExecutorEventFactory::new(task_id, agent).started()
}

pub(crate) fn executor_task_completed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    output: impl Into<String>,
) -> ExecutorEvent {
    ExecutorEventFactory::new(task_id, agent).completed(output)
}

pub(crate) fn executor_task_failed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    error: impl Into<String>,
) -> ExecutorEvent {
    ExecutorEventFactory::new(task_id, agent).failed(error)
}

pub(crate) fn goal_has_decomposed_tasks(
    tasks: &[macaca_proto::TodoItem],
    goal_id: macaca_proto::TaskId,
) -> bool {
    tasks.iter().any(|task| task.parent_task == Some(goal_id))
}
