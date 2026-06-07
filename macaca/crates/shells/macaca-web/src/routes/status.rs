//! GET /api/status — thin system facade boundary.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

use super::apps::service_status_views;

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
    tracing::info!("web status route entering thin system facade boundary");
    match state
        .system_facade
        .service_client()
        .inspect_services(
            &macaca_sdk::ServiceInspectionCommand::new("web-route-status-services")
                .expect("static service inspection scope is non-empty"),
        )
        .await
    {
        Ok(snapshot) => tracing::info!(
            services = snapshot.services.len(),
            "web status route inspected service runtime through facade bundle"
        ),
        Err(error) => tracing::warn!(
            error = %error,
            "web status route service inspection failed; preserving legacy response"
        ),
    }
    let agent_count = state.kernel.agent_count().await;
    let app_count = if let Some(views) = service_status_views(&state, "web-route-status-apps").await
    {
        views.len()
    } else {
        crate::application_shell_adapter::running_app_count(&state).await
    };
    let llm_provider = crate::llm_route_shell_adapter::status_provider_label(&state).await;

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").into(),
        agent_count,
        app_count,
        llm_provider,
    })
}
