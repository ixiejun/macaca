//! GET /api/status — thin system facade boundary.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /api/status
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub agent_count: usize,
    pub app_count: usize,
    pub llm_provider: String,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    tracing::info!(
        operation = "status_snapshot",
        "web status route entering SDK status facade"
    );
    match state.status_client.status_snapshot().await {
        Ok(snapshot) => Json(StatusResponse {
            version: snapshot.version,
            agent_count: snapshot.agent_count,
            app_count: snapshot.loaded_apps,
            llm_provider: snapshot.llm_provider,
        }),
        Err(error) => {
            tracing::warn!(
                operation = "status_snapshot",
                reason_code = "status_client_failed",
                error = %error,
                "web status route preserving bounded unavailable response"
            );
            Json(StatusResponse {
                version: env!("CARGO_PKG_VERSION").into(),
                agent_count: 0,
                app_count: 0,
                llm_provider: "service.llm.unavailable".into(),
            })
        }
    }
}
