//! Web adapters for Interactive Agent Workbench operations.
//!
//! The route in this module is intentionally read-only and data-driven.  Web is
//! allowed to bind HTTP, attach trace context, call focused SDK clients, and
//! serialize bounded diagnostics.  Policy, approval, file/process/sandbox
//! behavior, plugin lifecycle, tool routing, and workflow semantics stay in the
//! owning services.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{
    ApplicationId, ProtoErrorAdapter, TraceContext, WorkbenchCommand, WorkbenchCommandScope,
    WorkbenchCommandStatus, WorkbenchServiceDescriptor,
};
use macaca_sdk::{
    ServiceBackedWorkbenchClient, SystemServiceClient, SystemWorkbenchClient,
    WorkbenchClientCatalog,
};
use serde::Serialize;
use serde_json::Value;

use crate::routes::{err, proto_err, ErrorResponse};
use crate::service_runtime_client::WebRuntimeSystemServiceClient;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct WorkbenchOperationsResponse {
    pub app_id: String,
    pub trace_id: String,
    pub services: Vec<WorkbenchServiceView>,
    pub adapter_surfaces: Vec<WorkbenchAdapterSurface>,
    pub count: WorkbenchOperationsCount,
}

#[derive(Debug, Serialize)]
pub struct WorkbenchServiceView {
    pub service_id: String,
    pub capability_id: Option<String>,
    pub lifecycle_state: String,
    pub health: Value,
    pub commands: Vec<&'static str>,
    pub event_schema: &'static str,
    pub audit_schema: &'static str,
    pub snapshot_schema: &'static str,
    pub snapshot: Value,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct WorkbenchAdapterSurface {
    pub service_id: &'static str,
    pub surface: &'static str,
    pub endpoint: String,
}

#[derive(Debug, Serialize)]
pub struct WorkbenchOperationsCount {
    pub services: usize,
    pub available: usize,
    pub unavailable: usize,
    pub failed: usize,
    pub adapter_surfaces: usize,
}

/// GET /api/apps/{app_id}/workbench/operations
/// returns a bounded diagnostics snapshot for shell/frontend rendering.
pub async fn get_workbench_operations(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<WorkbenchOperationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-workbench-operations-{}", app_id.0));
    let scope = WorkbenchCommandScope::new(app_id, "web-workbench-operations")
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let runtime_client: Arc<dyn SystemServiceClient> =
        Arc::new(WebRuntimeSystemServiceClient::new(
            Arc::clone(&state.service_runtime),
            "web.workbench.operations",
        ));
    let clients = WorkbenchClientCatalog::service_backed(runtime_client);

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route collecting workbench diagnostics through focused SDK clients"
    );

    let mut services = Vec::new();
    for (label, client) in focused_clients(&clients) {
        services.push(read_service(label, client, scope.clone(), trace.clone()).await);
    }
    let app_id_text = app_id.0.to_string();
    let count = summarize(&services, adapter_surfaces(&app_id_text).len());

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        services = count.services,
        available = count.available,
        unavailable = count.unavailable,
        failed = count.failed,
        "web route emitted workbench diagnostics snapshot"
    );

    Ok(Json(WorkbenchOperationsResponse {
        app_id: app_id_text.clone(),
        trace_id: trace.trace_id,
        services,
        adapter_surfaces: adapter_surfaces(&app_id_text),
        count,
    }))
}

fn focused_clients<'a>(
    clients: &'a macaca_sdk::WorkbenchClientCatalog<ServiceBackedWorkbenchClient>,
) -> Vec<(&'static str, &'a ServiceBackedWorkbenchClient)> {
    vec![
        ("interaction", &clients.interaction),
        ("app_protocol", &clients.app_protocol),
        ("file", &clients.file),
        ("process", &clients.process),
        ("sandbox", &clients.sandbox),
        ("approval", &clients.approval),
        ("hook", &clients.hook),
        ("config", &clients.config),
        ("plugin_marketplace", &clients.plugin_marketplace),
        ("code_intelligence", &clients.code_intelligence),
        ("git", &clients.git),
        ("review", &clients.review),
        ("diagnostics", &clients.diagnostics),
        ("realtime", &clients.realtime),
        ("remote_environment", &clients.remote_environment),
    ]
}

async fn read_service(
    label: &'static str,
    client: &ServiceBackedWorkbenchClient,
    scope: WorkbenchCommandScope,
    trace: TraceContext,
) -> WorkbenchServiceView {
    let descriptor = client.descriptor().clone();
    let command = WorkbenchCommand::new(scope, trace.clone(), "service.snapshot", Value::Null);
    let result = match command {
        Ok(command) => client
            .call("service.snapshot", command)
            .await
            .map(|result| {
                let status = status_label(&result.status);
                let output = serde_json::to_value(result).unwrap_or(Value::Null);
                (status, output)
            }),
        Err(error) => Err(error),
    };
    match result {
        Ok((status, snapshot)) => view_from_descriptor(descriptor, status, snapshot),
        Err(error) => {
            tracing::warn!(
                service = label,
                trace_id = %trace.trace_id,
                error = %error,
                "workbench diagnostics snapshot failed; returning structured failed row"
            );
            view_from_descriptor(
                descriptor,
                "failed".into(),
                serde_json::json!({
                    "status": "failed",
                    "reason": error.display_message(),
                    "trace_id": trace.trace_id,
                }),
            )
        }
    }
}

fn view_from_descriptor(
    descriptor: WorkbenchServiceDescriptor,
    status: String,
    snapshot: Value,
) -> WorkbenchServiceView {
    WorkbenchServiceView {
        service_id: descriptor.base.id.to_string(),
        capability_id: descriptor
            .base
            .capabilities
            .first()
            .map(|capability| capability.id.to_string()),
        lifecycle_state: format!("{:?}", descriptor.base.lifecycle_state),
        health: serde_json::to_value(&descriptor.base.health).unwrap_or(Value::Null),
        commands: descriptor.commands,
        event_schema: descriptor.event_schema,
        audit_schema: descriptor.audit_schema,
        snapshot_schema: descriptor.snapshot_schema,
        snapshot,
        status,
    }
}

fn status_label(status: &WorkbenchCommandStatus) -> String {
    match status {
        WorkbenchCommandStatus::Unavailable { .. } => "unavailable",
        WorkbenchCommandStatus::Unsupported { .. } => "unsupported",
        WorkbenchCommandStatus::Denied { .. } => "denied",
        WorkbenchCommandStatus::Failed { .. } => "failed",
        WorkbenchCommandStatus::PendingApproval { .. } => "pending_approval",
        WorkbenchCommandStatus::Cancelled { .. } => "cancelled",
        WorkbenchCommandStatus::Timeout { .. } => "timeout",
        WorkbenchCommandStatus::ArtifactBacked { .. } => "artifact_backed",
        WorkbenchCommandStatus::Completed => "completed",
    }
    .into()
}

fn adapter_surfaces(app_id: &str) -> Vec<WorkbenchAdapterSurface> {
    vec![
        WorkbenchAdapterSurface {
            service_id: "service.tool",
            surface: "tool diagnostics",
            endpoint: format!("/api/apps/{app_id}/tools/operations"),
        },
        WorkbenchAdapterSurface {
            service_id: "service.skill",
            surface: "skill operator diagnostics",
            endpoint: format!("/api/apps/{app_id}/skills/operations"),
        },
        WorkbenchAdapterSurface {
            service_id: "service.mcp",
            surface: "mcp runtime status",
            endpoint: "/api/mcp".into(),
        },
        WorkbenchAdapterSurface {
            service_id: "service.plugin_control",
            surface: "plugin control list",
            endpoint: "/api/plugins".into(),
        },
    ]
}

fn summarize(
    services: &[WorkbenchServiceView],
    adapter_surfaces: usize,
) -> WorkbenchOperationsCount {
    let unavailable = services
        .iter()
        .filter(|service| service.status == "unavailable")
        .count();
    let failed = services
        .iter()
        .filter(|service| service.status == "failed")
        .count();
    WorkbenchOperationsCount {
        services: services.len(),
        available: services.len().saturating_sub(unavailable + failed),
        unavailable,
        failed,
        adapter_surfaces,
    }
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn workbench_routes_remain_thin_sdk_adapters() {
        let source = include_str!("workbench_routes.rs");
        assert!(source.contains("SystemWorkbenchClient"));
        assert!(source.contains("WebRuntimeSystemServiceClient"));
        assert!(source.contains("service.snapshot"));
        assert!(!source.contains(&format!("{}{}", "Local", "FileProvider")));
        assert!(!source.contains(&format!("{}{}", "Local", "ProcessProvider")));
        assert!(!source.contains(&format!("{}{}", "Policy", "Denied")));
        assert!(!source.contains(&format!("{}{}", "Approval", "Decision")));
    }
}
