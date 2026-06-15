//! PlanLoop startup orchestrator (thin facade).
//!
//! Owns idempotent PlanLoop registration with execution_control and spawns the
//! macaca-task controller plus the PlanEvent consumer task. Event handling lives
//! in `plan_event_consumer` and per-variant Strategy handlers.

use std::sync::Arc;

use macaca_proto::{ApplicationId, PlanEvent};

use super::execution_control_adapter::session_loop_coordinator;
use super::plan_event_consumer::dispatch_plan_event;
use super::plan_event_context::PlanEventConsumerCtx;
use crate::session_loop_shell_adapter::register_plan_loop_via_execution_control;
use crate::state::AppState;

/// Start PlanLoop for `app_id` when not already running (idempotent).
pub(crate) async fn ensure_plan_loop(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &Option<String>,
    entry_agent_name: &str,
    plan_agent_name: &str,
) {
    let Some(controller) = state
        .loops
        .local_runtime
        .reserve_local_plan_loop_controller(
            app_id.clone(),
            session_id.clone(),
            Arc::clone(&state.persist.todo_store),
        )
        .await
    else {
        return;
    };

    let plan_loop = controller.plan_loop;
    let shutdown = controller.shutdown;
    let plan_waker = plan_loop.waker();
    state
        .loops
        .local_runtime
        .set_plan_loop_waker(app_id.clone(), plan_waker)
        .await;

    // Register PlanLoop with execution_control before the consumer handles events.
    // Audit replay must observe register → wake → shutdown ordering.
    let coordinator = session_loop_coordinator(state);
    register_plan_loop_via_execution_control(&coordinator, app_id.clone(), session_id.clone())
        .await;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<PlanEvent>(64);
    let event_tx_for_plan_loop = event_tx.clone();

    tokio::spawn(async move {
        plan_loop
            .run_with_default_template(shutdown, event_tx_for_plan_loop)
            .await;
    });

    let rt_plan_start = Arc::clone(&state.persist.run_tracer);
    let app_plan_start = app_id.clone();
    tokio::spawn(async move {
        crate::run_trace::emit_for_scope(
            &rt_plan_start,
            None,
            &app_plan_start,
            crate::run_trace::phase::PLAN_LOOP_STARTED,
            "plan_loop",
            crate::run_trace::status::INFO,
            Some("spawned".into()),
            None,
            None,
            None,
        )
        .await;
    });

    let consumer_ctx = PlanEventConsumerCtx {
        state: Arc::clone(state),
        app_id: app_id.clone(),
        session_store: Arc::clone(&state.persist.session_store),
        plan_agent_name: plan_agent_name.to_string(),
        entry_agent_name: entry_agent_name.to_string(),
        event_tx,
    };

    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            dispatch_plan_event(&consumer_ctx, event).await;
        }
    });

    tracing::info!(app_id = %app_id, "PlanLoop started for app");
}
