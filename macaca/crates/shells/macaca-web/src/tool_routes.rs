//! Tool diagnostics route adapters.
//!
//! This module is intentionally thin. The Web shell may normalize HTTP input,
//! attach trace context, log bounded counters, and serialize SDK DTOs, but all
//! policy, provider lifecycle, planning, invocation, artifact, and replay
//! semantics remain owned by `service.tool`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{ApplicationId, ToolGenericTraceCommand, TraceContext};

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

/// GET /api/apps/{app_id}/tools/operations
/// returns a bounded Tool Capability Plane diagnostics snapshot.
///
/// The handler is a Web Adapter over the focused SDK Facade. It does not
/// inspect concrete Driver, Skill, MCP, or provider runtime state. Instead, it
/// asks `service.tool` for sanitized mementos and status DTOs that are already
/// safe for operator UI, logs, and future SSE/EventLog projections.
pub async fn get_tool_operations(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-tool-operations-{}", app_id.0));
    let command = ToolGenericTraceCommand::new(trace.clone())
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route aggregating tool diagnostics through Tool service"
    );

    let snapshot = state
        .tool_client
        .catalog_snapshot(command.clone())
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let provider_status = state
        .tool_client
        .provider_status(command.clone())
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let provider_health = state
        .tool_client
        .provider_health(trace.clone())
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let policy = state
        .tool_client
        .policy_explain(command.clone())
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let audit = state
        .tool_client
        .audit_query(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        visible_tools = snapshot.plan.visible.len(),
        hidden_tools = snapshot.plan.hidden.len(),
        conflicts = snapshot.plan.conflicts.len(),
        provider_count = provider_status.provider_count,
        "web route emitted sanitized tool operations snapshot"
    );

    Ok(Json(serde_json::json!({
        "snapshot": snapshot,
        "provider_status": provider_status,
        "provider_health": provider_health,
        "policy": policy,
        "audit": audit,
        "count": {
            "visible_tools": snapshot.plan.visible.len(),
            "hidden_tools": snapshot.plan.hidden.len(),
            "conflicts": snapshot.plan.conflicts.len(),
            "provider_count": provider_status.provider_count,
        },
        "trace_id": trace.trace_id,
    })))
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_routes_remain_thin_sdk_adapters() {
        let source = include_str!("tool_routes.rs");
        assert!(source.contains(".tool_client"));
        assert!(source.contains("catalog_snapshot"));
        assert!(source.contains("provider_health"));
        assert!(source.contains("policy_explain"));
        assert!(source.contains("audit_query"));
        assert!(!source.contains(&format!("{}{}", "Driver", "Registry")));
        assert!(!source.contains(&format!("{}{}", "SkillRuntime", "Facade")));
        assert!(!source.contains(&format!("{}{}", "McpRuntime", "Facade")));
        assert!(!source.contains(&format!("{}{}", "ServiceHealth::", "Unavailable")));
        assert!(!source.contains(&format!("{}{}", "Policy", "Denied")));
    }
}
