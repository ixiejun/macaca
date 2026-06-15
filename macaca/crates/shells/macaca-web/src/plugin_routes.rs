//! Thin Web routes for Plugin Control.
//!
//! These handlers translate HTTP requests into SDK Plugin Control commands.
//! They intentionally do not read plugin directories, load manifests, or call
//! runtime-host internals.  That keeps Web a presentation shell while the
//! runtime-host control plane owns plugin lifecycle semantics.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{PluginId, PluginListCommand, PluginTargetCommand, TraceContext};

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

/// List plugins through the serviceized SDK client.
pub async fn list_plugins(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<macaca_proto::PluginControlRecord>>, (StatusCode, Json<ErrorResponse>)> {
    let command = PluginListCommand {
        repository_id: None,
        include_disabled: true,
        trace: TraceContext::new("web-plugin-list"),
    };
    state
        .system_facade
        .plugin_control_client()
        .list(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::SERVICE_UNAVAILABLE, &error))
}

/// Inspect one plugin through the serviceized SDK client.
pub async fn inspect_plugin(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<Json<macaca_proto::PluginControlRecord>, (StatusCode, Json<ErrorResponse>)> {
    let command = PluginTargetCommand::new(
        PluginId::new(plugin_id),
        TraceContext::new("web-plugin-inspect"),
    );
    match state
        .system_facade
        .plugin_control_client()
        .inspect(command)
        .await
    {
        Ok(Some(record)) => Ok(Json(record)),
        Ok(None) => Err(err(StatusCode::NOT_FOUND, "plugin not found".into())),
        Err(error) => Err(proto_err(StatusCode::SERVICE_UNAVAILABLE, &error)),
    }
}

/// Return sanitized plugin diagnostics through the serviceized SDK client.
pub async fn plugin_diagnostics(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<Json<macaca_proto::PluginDiagnostics>, (StatusCode, Json<ErrorResponse>)> {
    let command = PluginTargetCommand::new(
        PluginId::new(plugin_id),
        TraceContext::new("web-plugin-diagnostics"),
    );
    state
        .system_facade
        .plugin_control_client()
        .diagnostics(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::SERVICE_UNAVAILABLE, &error))
}

#[cfg(test)]
mod tests {
    #[test]
    fn plugin_routes_use_sdk_control_client_only() {
        let source = include_str!("plugin_routes.rs");
        assert!(source.contains("plugin_control_client()"));
        assert!(!source.contains(&format!("{}{}", "PluginControl", "Service::")));
        assert!(!source.contains(&format!(
            "{}{}",
            "macaca_sdk::", "runtime_host::PluginControlService"
        )));
    }
}
