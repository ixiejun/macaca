//! Thin orchestrator composing PlanLoop and WorkerLoop startup.
//!
//! Resolves entry/planner agents once, migrates legacy todos, then delegates to
//! `plan_loop_orchestrator` and `worker_loop_orchestrator` sub-adapters.

use std::sync::Arc;

use macaca_sdk::app::app_entry_agent_name;
use macaca_proto::ApplicationId;

use super::planner_helpers::select_entry_and_plan_agents;
use crate::state::AppState;

use super::plan_loop_orchestrator::ensure_plan_loop;
use super::worker_loop_orchestrator::ensure_worker_loops;

/// Ensure PlanLoop and WorkerLoops are running for an application (idempotent).
pub(crate) async fn ensure_plan_and_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<String>,
) {
    // Determine entry/planner agents from declarative config + capabilities.
    let manifest_entry = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry.get_app(app_id).map(|app| {
            app_entry_agent_name(&app.manifest)
                .unwrap_or("entry_agent")
                .to_string()
        })
    };
    let (entry_agent_name, plan_agent_name): (String, String) =
        if let Some(executor) = state.executor_registry.get(app_id).await {
            let agents = executor.list_agents().await;
            select_entry_and_plan_agents(&agents, manifest_entry.as_deref())
        } else {
            let entry = manifest_entry.unwrap_or_else(|| "entry_agent".to_string());
            (entry.clone(), entry)
        };

    // ── Migrate legacy tasks (one-time, idempotent) ──
    state
        .persist
        .todo_store
        .migrate_sequence_numbers(app_id)
        .await;


    ensure_plan_loop(
        state,
        app_id,
        &session_id,
        &entry_agent_name,
        &plan_agent_name,
    )
    .await;

    ensure_worker_loops(
        state,
        app_id,
        &session_id,
        &entry_agent_name,
        &plan_agent_name,
    )
    .await;
}
