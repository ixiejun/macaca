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
                "Application Service reload failed; falling back to legacy registry"
            );
            crate::application_shell_adapter::reload_legacy_registry(&state)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        }
    };

    // Build app info from discovered apps
    if let Some(views) = service_status_views(&state, "web-route-apps-reload-status").await {
        return Ok(Json(ReloadResponse {
            discovered_count,
            apps: views.iter().map(app_info_from_service_view).collect(),
        }));
    }

    let apps = crate::application_shell_adapter::list_runtime_apps(&state).await;
    let agent_count = state.kernel.agent_count().await;

    let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
    let app_infos: Vec<AppInfo> = apps
        .into_iter()
        .map(|(id, name, status)| {
            let (description, icon) = registry
                .get_app_by_name(&name)
                .map(|app| {
                    let desc = app
                        .manifest
                        .description
                        .as_deref()
                        .unwrap_or("An Agent OS application.");
                    (desc.to_string(), "cube".to_string())
                })
                .unwrap_or_else(|| ("An Agent OS application.".to_string(), "cube".to_string()));
            AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count,
                entry_agent: None,
                description,
                icon,
                ui: None,
            }
        })
        .collect();
    drop(registry);

    Ok(Json(ReloadResponse {
        discovered_count,
        apps: app_infos,
    }))
}
