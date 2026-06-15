//! Driver inventory and hot-reload routes.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use macaca_proto::TraceContext;
use macaca_sdk::{
    DriverInventoryCommand, DriverLoadServiceCommand, DriverLoadStatus, DriverServiceScope,
};

use crate::state::AppState;

use super::shared::{proto_err, ErrorResponse};

// ---------------------------------------------------------------------------
// GET /api/drivers — List loaded drivers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DriverInfo {
    pub name: String,
    pub version: String,
    pub driver_type: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub tools_count: usize,
}

#[derive(Serialize)]
pub struct DriversResponse {
    pub drivers: Vec<DriverInfo>,
    pub total: usize,
}

pub async fn get_drivers(State(state): State<Arc<AppState>>) -> Json<DriversResponse> {
    let command = DriverInventoryCommand {
        trace: TraceContext::new("web-route-driver-inventory"),
        scope: DriverServiceScope::default(),
        include_tools: true,
    };
    let result = state.driver_client.inventory(command).await;
    let drivers: Vec<DriverInfo> = result
        .map(|inventory| {
            inventory
                .entries
                .into_iter()
                .map(|item| DriverInfo {
                    name: item.name,
                    version: item.version.unwrap_or_default(),
                    driver_type: item.driver_type,
                    description: item.description,
                    capabilities: item.capabilities,
                    tools_count: item.tool_count,
                })
                .collect()
        })
        .unwrap_or_else(|error| {
            tracing::warn!(error = %error, "Driver service inventory failed; returning empty list");
            Vec::new()
        });
    let total = drivers.len();
    Json(DriversResponse { drivers, total })
}

// ---------------------------------------------------------------------------
// POST /api/drivers/reload — Rescan and reload drivers directory
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DriverReloadResult {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct DriverReloadResponse {
    pub loaded: usize,
    pub failed: usize,
    pub results: Vec<DriverReloadResult>,
}

pub async fn reload_drivers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DriverReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let report = state
        .driver_client
        .reload(
            DriverLoadServiceCommand::new(TraceContext::new("web-route-driver-reload"), true)
                .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?,
        )
        .await
        .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let mut results = Vec::new();

    for entry in &report.entries {
        match entry.status {
            DriverLoadStatus::Loaded => {
                tracing::info!(
                    driver = %entry.name,
                    tools = entry.tool_count.unwrap_or_default(),
                    "Driver reloaded; tools will be available to agents on next execution"
                );
                results.push(DriverReloadResult {
                    name: entry.name.clone(),
                    status: "ok".to_string(),
                    error: None,
                });
            }
            DriverLoadStatus::Failed => {
                results.push(DriverReloadResult {
                    name: entry.name.clone(),
                    status: "error".to_string(),
                    error: entry.error.clone(),
                });
            }
        }
    }

    Ok(Json(DriverReloadResponse {
        loaded: report.loaded,
        failed: report.failed,
        results,
    }))
}
