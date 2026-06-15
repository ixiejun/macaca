//! Application list and detail routes (Application Service adapter).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use macaca_proto::{
    ApplicationId, ApplicationMetadataQueryCommand, ApplicationMetadataView,
    ApplicationServiceAppView, ApplicationServiceScope, ApplicationStatusCommand,
    ApplicationUiRuntimeView, TraceContext,
};

use crate::state::AppState;

use super::shared::ErrorResponse;

// ---------------------------------------------------------------------------
// GET /api/apps
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub agent_count: usize,
    /// Manifest-declared entry agent.  Workspace clients treat this identity
    /// as the main thread and can hide it from delegated-agent tabs without
    /// hard-coding application-specific coordinator names.
    pub entry_agent: Option<String>,
    pub description: String,
    pub icon: String,
    pub ui: Option<ApplicationUiRuntimeView>,
}

pub(crate) fn app_info_from_service_view(view: &ApplicationServiceAppView) -> AppInfo {
    AppInfo {
        id: view.id.0.to_string(),
        name: view.name.clone(),
        status: view.runtime.runtime_status.clone(),
        agent_count: view.agents.len(),
        entry_agent: view.entry_agent.clone(),
        description: view
            .description
            .clone()
            .unwrap_or_else(|| "An Agent OS application.".to_string()),
        icon: "cube".to_string(),
        ui: view.ui.clone(),
    }
}

pub(crate) fn app_info_from_metadata_view(view: &ApplicationMetadataView) -> AppInfo {
    app_info_from_service_view(&view.application)
}

pub(crate) async fn service_status_views(
    state: &Arc<AppState>,
    trace_id: &'static str,
) -> Option<Vec<ApplicationServiceAppView>> {
    let command = ApplicationStatusCommand {
        trace: TraceContext::new(trace_id),
        scope: ApplicationServiceScope::default(),
    };
    match state.application_client.status(command).await {
        Ok(views) => Some(views),
        Err(error) => {
            tracing::warn!(error = %error, trace_id, "Application Service status failed");
            None
        }
    }
}

pub(crate) async fn service_metadata_view(
    state: &Arc<AppState>,
    app_id: ApplicationId,
    trace_id: &'static str,
) -> Option<ApplicationMetadataView> {
    let command =
        match ApplicationMetadataQueryCommand::application(TraceContext::new(trace_id), app_id) {
            Ok(command) => command,
            Err(error) => {
                tracing::warn!(error = %error, trace_id, "Application metadata command rejected");
                return None;
            }
        };
    match state.application_client.metadata(command).await {
        Ok(view) => Some(view),
        Err(error) => {
            tracing::warn!(
                error = %error,
                trace_id,
                "Application metadata query failed"
            );
            None
        }
    }
}

pub async fn get_apps(State(state): State<Arc<AppState>>) -> Json<Vec<AppInfo>> {
    if let Some(views) = service_status_views(&state, "web-route-apps-status").await {
        tracing::info!(
            count = views.len(),
            "Application list served from Application Service"
        );
        return Json(views.iter().map(app_info_from_service_view).collect());
    }

    tracing::warn!("Application list unavailable because Application Service returned no status");
    Json(Vec::new())
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id — Get single app info
// ---------------------------------------------------------------------------

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<AppInfo>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid app_id".into(),
            }),
        )
    })?;
    let app_id = macaca_proto::ApplicationId(app_uuid);

    if let Some(view) = service_metadata_view(&state, app_id, "web-route-app-detail-metadata").await
    {
        tracing::info!(
            app_id = %app_id,
            "Application detail served from sanitized metadata view"
        );
        return Ok(Json(app_info_from_metadata_view(&view)));
    }

    if let Some(views) = service_status_views(&state, "web-route-app-detail-status").await {
        if let Some(view) = views.iter().find(|view| view.id == app_id) {
            tracing::info!(app_id = %app_id, "Application detail served from Application Service status");
            return Ok(Json(app_info_from_service_view(view)));
        }
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "App not found".into(),
        }),
    ))
}
