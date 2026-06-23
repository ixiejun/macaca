//! Executor registry and application cleanup adapter.
//!
//! Ensures app-scoped executors exist before chat execution and tears down
//! loops, tasks, goals, and executor registrations on session boundaries.

use std::sync::Arc;

use macaca_proto::{
    AgentInfo, ApplicationId, ApplicationMetadataQueryCommand, ApplicationServiceScope,
    ApplicationStatusCommand, TraceContext,
};

use crate::application_shell_adapter::{app_runtime_agent_ids, select_app_scoped_agent_manifests};
use crate::session_loop_shell_adapter::{
    shutdown_session_loops_via_execution_control, REASON_SESSION_LOOP_APPLICATION_CLEANUP,
};
use crate::state::AppState;

/// Shared cleanup logic: stop loops, shutdown executor, reset agents, cancel tasks/goals.
///
/// Used by both `post_chat_stop` (explicit user stop) and `post_chat_v2` (new session creation).
pub(crate) async fn cleanup_app_state(state: &Arc<AppState>, app_id: &ApplicationId) {
    // 1. Unregister executor entirely so the next session can rebuild a fresh worker.
    let _ = state.executor_registry.unregister(app_id).await;

    // 2. Force all agent activities to Idle immediately
    {
        let agents = state.kernel.list_agents().await;
        for agent in &agents {
            state
                .kernel
                .update_agent_activity(&agent.id, macaca_proto::AgentActivity::Idle)
                .await;
        }
    }

    // 3. Cancel ALL non-terminal tasks
    {
        let all_todos = state.persist.todo_store.list_all_todos(app_id).await;
        for mut task in all_todos {
            if !matches!(
                task.status,
                macaca_proto::TodoStatus::Completed
                    | macaca_proto::TodoStatus::Cancelled
                    | macaca_proto::TodoStatus::Failed
            ) {
                task.status = macaca_proto::TodoStatus::Cancelled;
                task.updated_at = chrono::Utc::now();
                state.persist.todo_store.save_todo(&task).await;
            }
        }
        // Also cancel all non-terminal goals to prevent PlanLoop from re-decomposing old goals
        let goals = state.persist.todo_store.list_goals(app_id).await;
        let mut cancelled_goals = 0u32;
        for mut goal in goals {
            if !matches!(
                goal.status,
                macaca_proto::TodoGoalStatus::Completed
                    | macaca_proto::TodoGoalStatus::Cancelled
                    | macaca_proto::TodoGoalStatus::Failed
            ) {
                goal.status = macaca_proto::TodoGoalStatus::Cancelled;
                state.persist.todo_store.save_goal(&goal).await;
                cancelled_goals += 1;
            }
        }
        if cancelled_goals > 0 {
            tracing::info!(app_id = %app_id, cancelled_goals, "Cancelled non-terminal goals during cleanup");
        }
    }

    // 3.5 Record session-loop shutdown through execution control before tearing down
    // local waker maps.  This keeps audit replay aligned with goal/fork-join cleanup
    // paths that already route pause/resume via service.execution_control.
    {
        let coordinator =
            macaca_host_composition::execution_control::ExecutionControlSessionLoopCoordinator::new(
                Arc::clone(&state.service_runtime),
            );
        shutdown_session_loops_via_execution_control(
            &coordinator,
            app_id,
            REASON_SESSION_LOOP_APPLICATION_CLEANUP,
        )
        .await;
    }

    // 4. Signal all local task controllers and remove runtime-host handles so
    // the next session can start from a clean controller set.
    state
        .loops
        .local_runtime
        .shutdown_application_loops(app_id)
        .await;

    tracing::info!(app_id = %app_id, "Application state cleaned up: loops stopped, agents idle, tasks cancelled");
}

/// Ensure one executor exists for the target application and that the executor
/// is scoped to application-declared agents only.
///
/// Why fail-closed:
/// - This API path is used by end-user chat entrypoints.
/// - If an application does not declare executable agents, silently falling
///   back to global coordinator agents would couple shell behavior to unrelated
///   framework internals and produce non-auditable execution chains.
/// - Returning an explicit error keeps behavior deterministic and debuggable.
pub(crate) async fn ensure_app_executor(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Result<(), String> {
    if state.executor_registry.get(app_id).await.is_some() {
        return Ok(());
    }

    let metadata_command = ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-ensure-executor-metadata"),
        *app_id,
    );
    let (app_name, app_agent_names) = match metadata_command {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => (
                view.application.name,
                view.application
                    .agents
                    .into_iter()
                    .map(|agent| agent.name)
                    .collect::<Vec<_>>(),
            ),
            Err(error) => {
                tracing::warn!(
                    app_id = %app_id,
                    error = %error,
                    "Application metadata query failed while ensuring executor; using status fallback"
                );
                service_executor_metadata(state, app_id).await
            }
        },
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata command rejected while ensuring executor; using status fallback"
            );
            service_executor_metadata(state, app_id).await
        }
    };

    let runtime_agent_ids =
        app_runtime_agent_ids(state, app_id, "web-chat-executor-runtime-agent-ids").await;
    let app_manifests = select_app_scoped_agent_manifests(
        state.kernel.list_agents().await,
        &runtime_agent_ids,
        Some(&app_agent_names),
    );
    let app_agents: Vec<AgentInfo> = app_manifests
        .into_iter()
        .map(|agent| AgentInfo {
            id: agent.id.0.to_string(),
            name: agent.name,
            capabilities: agent.capabilities.into_iter().map(|c| c.name).collect(),
            current_load: 0,
            max_load: 4,
            available: true,
        })
        .collect();
    if app_agent_names.is_empty() {
        tracing::warn!(
            app_id = %app_id,
            "Application metadata exposed no app-scoped agents; refusing executor fallback"
        );
        return Err(format!(
            "application {app_id} has no executable app-scoped agents; chat execution is denied"
        ));
    }
    if app_agents.is_empty() {
        tracing::warn!(
            app_id = %app_id,
            declared_agent_count = app_agent_names.len(),
            "Application declared agents but none are currently available in kernel registry"
        );
        return Err(format!(
            "application {app_id} declared agents but none are available in kernel registry"
        ));
    }

    let _ = state
        .executor_registry
        .register_application(app_id.clone(), app_name, app_agents)
        .await;
    Ok(())
}

pub(crate) async fn service_executor_metadata(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> (String, Vec<String>) {
    match state
        .application_client
        .status(ApplicationStatusCommand {
            trace: TraceContext::new("web-chat-ensure-executor-status"),
            scope: ApplicationServiceScope::application(*app_id),
        })
        .await
    {
        Ok(views) => views
            .into_iter()
            .find(|view| view.id == *app_id)
            .map(|view| {
                (
                    view.name,
                    view.agents
                        .into_iter()
                        .map(|agent| agent.name)
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_else(|| (app_id.0.to_string(), Vec::new())),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application Service status failed while ensuring executor"
            );
            (app_id.0.to_string(), Vec::new())
        }
    }
}
