//! POST /api/apps/reload — rediscover applications from disk.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use macaca_proto::{ApplicationDiscoverCommand, TraceContext};

use crate::state::AppState;

use super::apps::{app_info_from_service_view, service_status_views, AppInfo};
use super::shared::{err, ErrorResponse};

// ---------------------------------------------------------------------------
// POST /api/apps/reload — Reload apps from disk
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ReloadResponse {
    pub discovered_count: usize,
    pub apps: Vec<AppInfo>,
}

pub async fn reload_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Reload the registry
    let discovered_count = match state
        .application_client
        .discover(
            ApplicationDiscoverCommand::new(TraceContext::new("web-route-apps-reload"))
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        )
        .await
    {
        Ok(discovered) => {
            tracing::info!(
                count = discovered.len(),
                "Application reload discovery served through Application Service"
            );
            discovered.len()
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "Application Service reload failed"
            );
            return Err(err(StatusCode::BAD_GATEWAY, error.to_string()));
        }
    };

    // Build app info from discovered apps
    if let Some(views) = service_status_views(&state, "web-route-apps-reload-status").await {
        return Ok(Json(ReloadResponse {
            discovered_count,
            apps: views.iter().map(app_info_from_service_view).collect(),
        }));
    }

    Ok(Json(ReloadResponse {
        discovered_count,
        apps: Vec::new(),
    }))
}
