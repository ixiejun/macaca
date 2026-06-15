//! Per-agent task tool registration through runtime-host factories.
//!
//! Web resolves application policy and supplies shell-specific observer ports,
//! but runtime-host owns `TaskSpace`, `TaskBoard`, and concrete todo tool
//! construction. This keeps the framework toolkit Builder as a thin Adapter
//! over host-owned task service capabilities.

use std::collections::HashMap;
use std::sync::Arc;

use macaca_host_composition::framework::execution::ExecutionContext;
use macaca_host_composition::framework::runtime_context::AgentSessionStore;
use macaca_host_composition::framework::tool::Toolkit;
use macaca_host_composition::tool_bootstrap::{
    bootstrap_task_toolkit_tools, GoalRecordedObserver, TaskToolkitBootstrapRequest,
    TaskToolkitPolicy,
};
use macaca_proto::{ApplicationId, TodoGoal};

use crate::framework_adapter::SingleToolAdapter;
use crate::state::AppState;

use super::policy::{AgentToolPolicy, TodoToolPolicy};

/// Register per-agent task tools into the framework toolkit.
pub(super) fn register_agent_tools(
    toolkit: &mut Toolkit,
    state: &AppState,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    goal_id: Option<macaca_proto::TaskId>,
    policy: &AgentToolPolicy,
    assignee_capabilities: &HashMap<String, Vec<String>>,
) {
    let task_policy = task_toolkit_policy(policy.todo_policy);
    let on_goal_recorded = goal_recorded_observer(state, app_id, agent_name, policy.todo_policy);
    let tools = bootstrap_task_toolkit_tools(
        TaskToolkitBootstrapRequest {
            application_id: *app_id,
            agent_name: agent_name.to_string(),
            session_id,
            active_goal_id: goal_id,
            policy: task_policy,
            todo_store: Arc::clone(&state.persist.todo_store),
            disallowed_task_assignees: policy.disallowed_task_assignees.iter().cloned().collect(),
            assignee_capabilities: assignee_capabilities.clone(),
            on_goal_recorded,
        },
        "web-framework-task-toolkit",
    );

    for tool in tools {
        toolkit.register(Box::new(SingleToolAdapter::new(tool)), Some("todo"));
    }
}

fn task_toolkit_policy(policy: TodoToolPolicy) -> TaskToolkitPolicy {
    match policy {
        TodoToolPolicy::GoalManager => TaskToolkitPolicy::GoalManager,
        TodoToolPolicy::Planner => TaskToolkitPolicy::Planner,
        TodoToolPolicy::Worker => TaskToolkitPolicy::Worker,
    }
}

fn goal_recorded_observer(
    state: &AppState,
    app_id: &ApplicationId,
    agent_name: &str,
    policy: TodoToolPolicy,
) -> Option<GoalRecordedObserver> {
    if matches!(policy, TodoToolPolicy::Worker) {
        return None;
    }

    let run_tracer = Arc::clone(&state.persist.run_tracer);
    let app = *app_id;
    let framework_session_store = Arc::clone(&state.sessions.framework_session_store);
    let owner_agent = agent_name.to_string();
    let service_runtime = Arc::clone(&state.service_runtime);
    let goal_to_session = Arc::clone(&state.sessions.goal_to_session);
    let register_execution_control_wait = matches!(policy, TodoToolPolicy::GoalManager);

    Some(Arc::new(move |goal: TodoGoal| {
        let run_tracer = Arc::clone(&run_tracer);
        let framework_session_store = Arc::clone(&framework_session_store);
        let service_runtime = Arc::clone(&service_runtime);
        let goal_to_session = Arc::clone(&goal_to_session);
        let owner_agent = owner_agent.clone();

        tokio::spawn(async move {
            if let Some(session_id) = goal.session_id.clone() {
                if register_execution_control_wait {
                    goal_to_session
                        .write()
                        .await
                        .insert(goal.id.to_string(), session_id.clone());
                    let goal_coordinator =
                        macaca_host_composition::execution_control::ExecutionControlGoalLifecycleCoordinator::new(
                            service_runtime,
                        );
                    crate::goal_lifecycle_shell_adapter::register_goal_wait_via_execution_control(
                        &goal_coordinator,
                        app,
                        session_id.clone(),
                        owner_agent.clone(),
                        goal.id,
                    )
                    .await;
                }

                persist_paused_goal_context(
                    Arc::clone(&framework_session_store),
                    app,
                    &session_id,
                    &owner_agent,
                    goal.id,
                )
                .await;
            }

            crate::run_trace::emit_for_scope(
                &run_tracer,
                goal.session_id.as_deref(),
                &app,
                crate::run_trace::phase::GOAL_CREATE_TOOL,
                "create_goal_tool",
                crate::run_trace::status::OK,
                Some(format!("goal_id={}", goal.id)),
                None,
                Some(goal.id.to_string()),
                None,
            )
            .await;
        });
    }))
}

async fn persist_paused_goal_context(
    framework_session_store: Arc<dyn AgentSessionStore>,
    app: ApplicationId,
    session_id: &str,
    owner_agent: &str,
    goal_id: macaca_proto::TaskId,
) {
    let mut ctx = ExecutionContext::new(session_id.to_string(), app.0.to_string(), owner_agent);
    if let Some(restored) = crate::framework_state_memento::load_execution_context(
        framework_session_store.as_ref(),
        &app.0.to_string(),
        session_id,
    )
    .await
    {
        ctx = restored;
    }
    ctx.mark_paused(Some(format!("waiting_goal_completion:{}", goal_id)));
    crate::framework_state_memento::save_execution_context(
        framework_session_store.as_ref(),
        &mut ctx,
    )
    .await;
}
