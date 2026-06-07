//! SSE channel adapter for PlanLoop / WorkerLoop decision broadcasts.
//!
//! Centralizes plan_decision SSE emission + EventLog persistence so task-event
//! consumers stay focused on orchestration rather than transport details.

use std::sync::Arc;

use axum::response::sse::Event;
use macaca_runtime_host::persist::PersistStore;
use macaca_proto::ApplicationId;

use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};
use crate::state::AppState;

/// Broadcast one plan decision to app sessions and persist it for replay.
pub(crate) async fn broadcast_plan_decision<S>(
    state: &Arc<AppState>,
    session_store: &Arc<S>,
    app_id: &ApplicationId,
    decision_type: &str,
    message: String,
    payload: serde_json::Value,
    persist_data: serde_json::Value,
) where
    S: PersistStore + ?Sized,
{
    let sse_event = Event::default()
        .event("plan_decision")
        .data(payload.to_string());
    broadcast_to_app_sessions(state, app_id, sse_event, payload).await;
    save_plan_decision(
        session_store,
        app_id,
        PlanDecisionEvent {
            decision_type: decision_type.into(),
            message,
            timestamp: chrono::Utc::now(),
            data: persist_data,
        },
    )
    .await;
}
