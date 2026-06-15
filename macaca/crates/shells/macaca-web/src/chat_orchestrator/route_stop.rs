//! POST /api/chat/stop route handler.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::Json;
use macaca_host_composition::framework::execution::ExecutionContext;
use macaca_proto::ApplicationId;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

use super::dto::StopRequest;
use super::executor_adapter::cleanup_app_state;
use super::session_persistence_adapter::persist_execution_context;

/// POST /api/chat/stop — terminate all running processes for an application.
///
/// Sets the cancel flag for the coordinator loop, shuts down the executor,
/// and signals all PlanLoop/WorkerLoop instances to stop.
pub(crate) async fn post_chat_stop(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StopRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    // 1. Set coordinator cancel flag
    {
        let flags = state.sessions.cancel_flags.read().await;
        if let Some(flag) = flags.get(&req.app_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    // 2. Shared cleanup: executor, agents, tasks, loops
    cleanup_app_state(&state, &app_id).await;

    // 2a. Mark framework execution contexts as stopped for active sessions of this app.
    let stopped_session_ids: Vec<String> = state
        .sessions
        .active_sessions
        .read()
        .await
        .values()
        .filter(|s| s.app_id == app_id)
        .map(|s| s.session_id.clone())
        .collect();
    for sid in &stopped_session_ids {
        let mut ctx = ExecutionContext::new(sid.clone(), app_id.0.to_string(), "unknown");
        if let Some(restored) = crate::framework_state_memento::load_execution_context(
            state.sessions.framework_session_store.as_ref(),
            &app_id.0.to_string(),
            sid,
        )
        .await
        {
            ctx = restored;
        }
        ctx.mark_stopped(Some("user_stop_all_processes".into()));
        persist_execution_context(&state, &ctx).await;
    }
    if !stopped_session_ids.is_empty() {
        state
            .sessions
            .execution_control_local_notifications
            .remove_many(&stopped_session_ids)
            .await;
        tracing::info!(
            app_id = %app_id,
            cleared_sessions = stopped_session_ids.len(),
            "Cleared runtime-host execution-control local notification handles for stopped sessions"
        );
    }

    // 3. Broadcast stop event to all sessions for this app
    let event = Event::default()
        .event("stopped")
        .data(serde_json::json!({"reason": "User terminated all processes"}).to_string());
    crate::sse::broadcast_to_app_sessions(
        &state,
        &app_id,
        event,
        serde_json::json!({"event": "terminated", "reason": "user_action"}),
    )
    .await;

    tracing::info!(app_id = %app_id, "All processes terminated");

    Ok(Json(serde_json::json!({
        "status": "terminated",
    })))
}
