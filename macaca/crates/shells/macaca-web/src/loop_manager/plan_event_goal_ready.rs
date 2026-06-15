//! PlanEvent::GoalReady handler — goal decomposition via agent-execution service.
//!
//! When macaca-task signals a new goal, this handler:
//! 1. Skips stale events if todos already exist for the goal.
//! 2. Resolves worker candidates dynamically from the executor registry.
//! 3. Delegates decomposition to the planning agent via `service.agent_execution`.
//! 4. Falls back to heuristic task creation when the planner fails.
//! 5. Broadcasts SSE + persists plan decisions for traceability.

use axum::response::sse::Event;
use macaca_host_composition::app::app_task_planning_contract;
use macaca_proto::{
    ApplicationPlanningAgentProfile, ApplicationTaskPlanningContract,
    BuildDecompositionPromptCommand, TaskId, TraceContext,
};

use super::agent_execution_adapter::{
    list_goal_todos_for_scope, run_planner_framework_call, PlannerFrameworkCallKind,
};
use super::decomposition_adapter::{
    create_fallback_decomposition_tasks, mark_goal_decomposition_failed,
    mark_goal_decomposition_ready, terminal_goal_task, PlannerWorkerDossier,
};
use super::plan_event_context::PlanEventConsumerCtx;
use super::planner_helpers::{goal_has_decomposed_tasks, planner_notebook_mark_decomposition};
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};

/// Handle `PlanEvent::GoalReady` — decompose goal into worker todos.
pub(crate) async fn handle_plan_event_goal_ready(
    ctx: &PlanEventConsumerCtx,
    goal_id: TaskId,
    description: String,
    session_id: Option<String>,
) {
    let existing_goal_tasks = match session_id.as_deref() {
        Some(sid) => {
            ctx.state
                .persist
                .todo_store
                .list_all_todos_for_session(&ctx.app_id, sid)
                .await
        }
        None => {
            ctx.state
                .persist
                .todo_store
                .list_all_todos(&ctx.app_id)
                .await
        }
    };
    if goal_has_decomposed_tasks(&existing_goal_tasks, goal_id) {
        tracing::info!(
            goal_id = %goal_id,
            session_id = ?session_id,
            "Skipping stale GoalReady event because goal already has tasks"
        );
        return;
    }

    crate::run_trace::emit_for_scope(
        &ctx.state.persist.run_tracer,
        session_id.as_deref(),
        &ctx.app_id,
        crate::run_trace::phase::PLAN_GOAL_READY,
        "plan_loop",
        crate::run_trace::status::INFO,
        Some("decompose_goal".into()),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({
            "description": description.chars().take(240).collect::<String>(),
        })),
    )
    .await;
    planner_notebook_mark_decomposition(
        &ctx.state,
        &ctx.app_id,
        session_id.as_deref(),
        goal_id,
        &description,
    )
    .await;
    {
        // Dynamically get available worker agents + capabilities (no hardcoding)
        let (worker_profiles, worker_dossiers): (
            Vec<ApplicationPlanningAgentProfile>,
            Vec<PlannerWorkerDossier>,
        ) = if let Some(executor) = ctx.state.executor_registry.get(&ctx.app_id).await {
            let agents = executor.list_agents().await;
            let manifest_by_name: std::collections::HashMap<_, _> = ctx
                .state
                .kernel
                .list_agents()
                .await
                .into_iter()
                .map(|m| (m.name.clone(), m))
                .collect();
            let workers: Vec<_> = agents
                .iter()
                .filter(|a| a.name != ctx.entry_agent_name && a.name != ctx.plan_agent_name)
                .collect();
            let dossiers = workers
                .iter()
                .map(|a| {
                    let capabilities = manifest_by_name
                        .get(&a.name)
                        .map(|m| {
                            m.capabilities
                                .iter()
                                .map(|c| format!("{}: {}", c.name, c.description))
                                .collect::<Vec<_>>()
                        })
                        .filter(|caps| !caps.is_empty())
                        .unwrap_or_else(|| a.capabilities.clone());
                    PlannerWorkerDossier {
                        name: a.name.clone(),
                        capabilities,
                    }
                })
                .collect();
            let profiles = workers
                .iter()
                .map(|a| {
                    let manifest = manifest_by_name.get(&a.name);
                    let capabilities = manifest
                        .map(|m| {
                            m.capabilities
                                .iter()
                                .map(|c| format!("{}: {}", c.name, c.description))
                                .collect::<Vec<_>>()
                        })
                        .filter(|caps| !caps.is_empty())
                        .unwrap_or_else(|| a.capabilities.clone());
                    let allowed_tools = manifest
                        .map(|m| m.permission.allowed_tools.clone())
                        .unwrap_or_default();
                    let permission_level = manifest
                        .map(|m| match m.permission.level {
                            macaca_proto::PermissionLevel::System => "system".to_string(),
                            macaca_proto::PermissionLevel::User => "user".to_string(),
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    let model = manifest
                        .map(|m| m.model.clone())
                        .filter(|model| !model.is_empty())
                        .unwrap_or_else(|| "app default".to_string());
                    ApplicationPlanningAgentProfile {
                        name: a.name.clone(),
                        capabilities,
                        available: a.available,
                        current_load: a.current_load as usize,
                        max_load: a.max_load as usize,
                        permission_level,
                        model,
                        allowed_tools,
                    }
                })
                .collect();
            (profiles, dossiers)
        } else {
            (vec![], vec![])
        };
        let task_client = macaca_sdk::ServiceBackedTaskBoardDataSource::new(
            ctx.state.system_facade.service_client(),
        );
        let prompt = {
            let registry = crate::application_shell_adapter::registry_read_guard(&ctx.state).await;
            let contract = if let Some(app) = registry.get_app(&ctx.app_id) {
                app_task_planning_contract(&app.manifest, worker_profiles.clone())
            } else {
                ApplicationTaskPlanningContract {
                    workflow_name: "default".into(),
                    entry_agent: ctx.entry_agent_name.clone(),
                    worker_agents: worker_profiles.clone(),
                }
            };
            let mut trace = TraceContext::new(format!(
                "task-build-decomposition-prompt-{}-{}",
                goal_id,
                uuid::Uuid::new_v4()
            ));
            trace.session_id = session_id.clone();
            trace.task_id = Some(goal_id.to_string());
            trace.agent = Some(ctx.plan_agent_name.clone());
            task_client
                .build_decomposition_prompt(BuildDecompositionPromptCommand {
                    goal_description: description.clone(),
                    planning_contract: contract,
                    trace: Some(trace),
                })
                .await
        };
        let prompt = match prompt {
            Ok(prompt) => prompt,
            Err(error) => {
                tracing::error!(
                    goal_id = %goal_id,
                    session_id = ?session_id,
                    error = %error,
                    "task service failed to build decomposition prompt"
                );
                mark_goal_decomposition_failed(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    &error.to_string(),
                )
                .await;
                return;
            }
        };
        let decomposition_result = run_planner_framework_call(
            &ctx.state,
            &ctx.app_id,
            &ctx.plan_agent_name,
            session_id.clone(),
            goal_id,
            prompt,
            format!(
                "Decomposing goal: {}",
                description.chars().take(80).collect::<String>()
            ),
            PlannerFrameworkCallKind::DecomposeGoal,
        )
        .await;
        let goal_tasks =
            list_goal_todos_for_scope(&ctx.state, &ctx.app_id, session_id.as_deref(), goal_id)
                .await;
        match decomposition_result {
            Ok(_) if !goal_tasks.is_empty() => {
                mark_goal_decomposition_ready(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    goal_tasks.len(),
                )
                .await;
            }
            Ok(_) => {
                let fallback_tasks = create_fallback_decomposition_tasks(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    &ctx.plan_agent_name,
                    &description,
                    &worker_dossiers,
                    None,
                    "Planner decomposition returned without creating todos",
                )
                .await;
                if fallback_tasks.is_empty() {
                    mark_goal_decomposition_failed(
                        &ctx.state,
                        &ctx.app_id,
                        session_id.as_deref(),
                        goal_id,
                        "Planner decomposition returned without creating todos",
                    )
                    .await;
                } else {
                    mark_goal_decomposition_ready(
                        &ctx.state,
                        &ctx.app_id,
                        session_id.as_deref(),
                        goal_id,
                        fallback_tasks.len(),
                    )
                    .await;
                }
            }
            Err(error) if !goal_tasks.is_empty() => {
                let existing_agents = goal_tasks
                    .iter()
                    .map(|task| task.assigned_agent.clone())
                    .collect::<std::collections::HashSet<_>>();
                let remaining_workers = worker_dossiers
                    .iter()
                    .filter(|worker| !existing_agents.contains(&worker.name))
                    .cloned()
                    .collect::<Vec<_>>();
                let fallback_tasks = create_fallback_decomposition_tasks(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    &ctx.plan_agent_name,
                    &description,
                    &remaining_workers,
                    terminal_goal_task(&goal_tasks),
                    &error,
                )
                .await;
                let total_tasks = goal_tasks.len() + fallback_tasks.len();
                tracing::warn!(
                    goal_id = %goal_id,
                    task_count = total_tasks,
                    error = %error,
                    "Planner decomposition failed after creating todos; allowing partial decomposition to proceed"
                );
                crate::run_trace::emit_for_scope(
                    &ctx.state.persist.run_tracer,
                    session_id.as_deref(),
                    &ctx.app_id,
                    "plan.goal_decomposition_partial_ready",
                    "plan_loop",
                    crate::run_trace::status::INFO,
                    Some(format!(
                        "planner_error={}; tasks={}",
                        error.chars().take(160).collect::<String>(),
                        total_tasks
                    )),
                    None,
                    Some(goal_id.to_string()),
                    Some(serde_json::json!({
                        "task_count": total_tasks,
                        "planner_created_tasks": goal_tasks.len(),
                        "fallback_added_tasks": fallback_tasks.len(),
                        "planner_error": error,
                    })),
                )
                .await;
                mark_goal_decomposition_ready(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    total_tasks,
                )
                .await;
            }
            Err(error) => {
                let fallback_tasks = create_fallback_decomposition_tasks(
                    &ctx.state,
                    &ctx.app_id,
                    session_id.as_deref(),
                    goal_id,
                    &ctx.plan_agent_name,
                    &description,
                    &worker_dossiers,
                    None,
                    &error,
                )
                .await;
                if fallback_tasks.is_empty() {
                    mark_goal_decomposition_failed(
                        &ctx.state,
                        &ctx.app_id,
                        session_id.as_deref(),
                        goal_id,
                        &error,
                    )
                    .await;
                } else {
                    mark_goal_decomposition_ready(
                        &ctx.state,
                        &ctx.app_id,
                        session_id.as_deref(),
                        goal_id,
                        fallback_tasks.len(),
                    )
                    .await;
                }
            }
        }
        crate::run_trace::emit_for_scope(
            &ctx.state.persist.run_tracer,
            session_id.as_deref(),
            &ctx.app_id,
            crate::run_trace::phase::PLAN_GOAL_DELEGATE,
            "plan_loop",
            crate::run_trace::status::INFO,
            Some(format!("delegated_to={}", ctx.plan_agent_name)),
            None,
            Some(goal_id.to_string()),
            None,
        )
        .await;
    }
    // Emit SSE decision event
    let msg = format!(
        "New goal submitted, decomposing into tasks: {}",
        description
    );
    let plan_payload = serde_json::json!({
        "decision_type": "goal_ready",
        "goal_id": goal_id.to_string(),
        "description": description,
        "message": msg,
    });
    let sse_event = Event::default()
        .event("plan_decision")
        .data(plan_payload.to_string());
    broadcast_to_app_sessions(&ctx.state, &ctx.app_id, sse_event, plan_payload).await;
    // Persist decision
    save_plan_decision(
        &ctx.session_store,
        &ctx.app_id,
        PlanDecisionEvent {
            decision_type: "goal_ready".into(),
            message: msg,
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": description }),
        },
    )
    .await;
}
