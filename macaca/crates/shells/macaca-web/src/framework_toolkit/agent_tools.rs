//! Per-agent todo/task tool registration (capability-driven Strategy contributors).
//!
//! Registers goal-manager, planner, or worker tool sets based on resolved policy.
//! Goal creation callbacks wire execution-control pause/resume for auditable traces.

use std::collections::HashMap;
use std::sync::Arc;

use macaca_sdk::framework::adapter::SingleToolAdapter;
use macaca_sdk::framework::execution::ExecutionContext;
use macaca_sdk::framework::session::{load_module_state, save_module_state};
use macaca_sdk::framework::tool::Toolkit;
use macaca_proto::ApplicationId;

use crate::state::AppState;

use super::policy::{AgentToolPolicy, TodoToolPolicy};

/// Register per-agent todo tools into the toolkit.
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
    match policy.todo_policy {
        TodoToolPolicy::GoalManager => {
            let space = Arc::new(macaca_sdk::task::TaskSpace::for_session(
                app_id.clone(),
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            let rt = Arc::clone(&state.persist.run_tracer);
            let app = app_id.clone();
            let goal_to_session = Arc::clone(&state.sessions.goal_to_session);
            let framework_session_store = Arc::clone(&state.sessions.framework_session_store);
            let service_runtime = Arc::clone(&state.service_runtime);
            let owner_agent = agent_name.to_string();
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CreateGoalTool {
                        space: Arc::clone(&space),
                        on_created: None,
                        on_goal_recorded: Some(Arc::new(move |goal: macaca_proto::TodoGoal| {
                            let rt = Arc::clone(&rt);
                            let app = app.clone();
                            let goal_to_session = Arc::clone(&goal_to_session);
                            let framework_session_store = Arc::clone(&framework_session_store);
                            let service_runtime = Arc::clone(&service_runtime);
                            let owner_agent = owner_agent.clone();
                            tokio::spawn(async move {
                                if let Some(session_id) = goal.session_id.clone() {
                                    goal_to_session
                                        .write()
                                        .await
                                        .insert(goal.id.to_string(), session_id.clone());
                                    let goal_coordinator =
                                        macaca_sdk::runtime_host::ExecutionControlGoalLifecycleCoordinator::new(
                                            service_runtime,
                                        );
                                    crate::goal_lifecycle_shell_adapter::register_goal_wait_via_execution_control(
                                        &goal_coordinator,
                                        app.clone(),
                                        session_id.clone(),
                                        owner_agent.clone(),
                                        goal.id,
                                    )
                                    .await;
                                    let mut ctx = ExecutionContext::new(
                                        session_id.clone(),
                                        app.0.to_string(),
                                        owner_agent.clone(),
                                    );
                                    let _ = load_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &mut ctx,
                                    )
                                    .await;
                                    ctx.mark_paused(Some(format!(
                                        "waiting_goal_completion:{}",
                                        goal.id
                                    )));
                                    let _ = save_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &ctx,
                                    )
                                    .await;
                                }
                                crate::run_trace::emit_for_scope(
                                    &rt,
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
                        })),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CheckTodoProgressTool { space },
                ))),
                Some("todo"),
            );
        }
        TodoToolPolicy::Planner => {
            let space = Arc::new(macaca_sdk::task::TaskSpace::for_session(
                app_id.clone(),
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CreateTodoTool {
                        space: Arc::clone(&space),
                        coordinator_name: agent_name.to_string(),
                        disallowed_assignees: policy
                            .disallowed_task_assignees
                            .iter()
                            .cloned()
                            .collect(),
                        assignee_capabilities: assignee_capabilities.clone(),
                        active_goal_id: goal_id,
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CreateTodosTool {
                        create_todo: macaca_sdk::tools::CreateTodoTool {
                            space: Arc::clone(&space),
                            coordinator_name: agent_name.to_string(),
                            disallowed_assignees: policy
                                .disallowed_task_assignees
                                .iter()
                                .cloned()
                                .collect(),
                            assignee_capabilities: assignee_capabilities.clone(),
                            active_goal_id: goal_id,
                        },
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::ReviewTodoTool {
                        space: Arc::clone(&space),
                        on_reviewed: None,
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CheckTodoProgressTool {
                        space: Arc::clone(&space),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::ReassignTaskTool {
                        space: Arc::clone(&space),
                    },
                ))),
                Some("todo"),
            );
            let rt = Arc::clone(&state.persist.run_tracer);
            let app = app_id.clone();
            let framework_session_store = Arc::clone(&state.sessions.framework_session_store);
            let owner_agent = agent_name.to_string();
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::CreateGoalTool {
                        space,
                        on_created: None,
                        on_goal_recorded: Some(Arc::new(move |goal: macaca_proto::TodoGoal| {
                            let rt = Arc::clone(&rt);
                            let app = app.clone();
                            let framework_session_store = Arc::clone(&framework_session_store);
                            let owner_agent = owner_agent.clone();
                            tokio::spawn(async move {
                                if let Some(session_id) = goal.session_id.clone() {
                                    let mut ctx = ExecutionContext::new(
                                        session_id.clone(),
                                        app.0.to_string(),
                                        owner_agent.clone(),
                                    );
                                    let _ = load_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &mut ctx,
                                    )
                                    .await;
                                    ctx.mark_paused(Some(format!(
                                        "waiting_goal_completion:{}",
                                        goal.id
                                    )));
                                    let _ = save_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &ctx,
                                    )
                                    .await;
                                }
                                crate::run_trace::emit_for_scope(
                                    &rt,
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
                        })),
                    },
                ))),
                Some("todo"),
            );
        }
        TodoToolPolicy::Worker => {
            // Worker agents: task board tools.
            let board = Arc::new(macaca_sdk::task::TaskBoard::for_agent(
                app_id.clone(),
                agent_name,
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::ClaimTaskTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::StartTaskTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::UpdateTaskProgressTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::SubmitTaskForReviewTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_sdk::tools::ListMyTasksTool { board },
                ))),
                Some("todo"),
            );
        }
    }
}
