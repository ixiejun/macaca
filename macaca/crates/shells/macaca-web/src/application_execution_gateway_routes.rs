//! Thin Web Shell gateway ingress adapters for application execution.
//!
//! External providers and remote agents report facts through these HTTP routes,
//! but the routes do not become authoritative storage.  They validate only
//! transport scope that is visible at the shell boundary, then forward typed
//! gateway commands to `service.application_execution` through the SDK facade.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tracing::{info, warn};

use macaca_proto::{
    AppendExecutionEventCommand, ApplicationExecutionEventEnvelope, ApplicationExecutionSnapshot,
    ApplicationId, ReportExecutionCompletionCommand, ReportExecutionFailureCommand,
    ReportExecutionHeartbeatCommand, ReportExecutionSnapshotCommand,
    RequestExecutionApprovalCommand,
};

use crate::application_execution_routes::{
    parse_application_id, required_text, required_trace, validate_command_scope,
};
use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

/// Lightweight acknowledgement for gateway snapshot routes.
///
/// Snapshot reporting returns the typed provider snapshot through the service
/// path.  This wrapper lets the HTTP response remain self-describing without
/// altering the underlying protocol DTO.
#[derive(Debug, Serialize)]
pub struct GatewaySnapshotResponse {
    pub accepted: bool,
    pub snapshot: ApplicationExecutionSnapshot,
}

/// POST /api/apps/{app_id}/execution/gateway/events
pub async fn gateway_append_event(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<AppendExecutionEventCommand>,
) -> Result<Json<ApplicationExecutionEventEnvelope>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.append_event", &command.callback_identity_ref)?;
    validate_event_scope("gateway.append_event", route_app_id, &command.event)?;
    info!(
        app_id = %command.event.application_id,
        session_id = %command.event.session_id,
        run_id = %command.event.run_id,
        trace_id = %command.event.trace.trace_id,
        "web route forwarding application execution gateway event through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .append_gateway_event(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/gateway/heartbeat
pub async fn gateway_heartbeat(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<ReportExecutionHeartbeatCommand>,
) -> Result<Json<ApplicationExecutionEventEnvelope>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.heartbeat", &command.callback_identity_ref)?;
    validate_command_scope(
        "gateway.heartbeat",
        route_app_id,
        &command.scope,
        &command.trace,
    )?;
    info!(
        app_id = %command.scope.application_id,
        session_id = %command.scope.session_id,
        run_id = %command.scope.run_id,
        trace_id = %command.trace.trace_id,
        "web route forwarding application execution gateway heartbeat through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .report_gateway_heartbeat(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/gateway/snapshot
pub async fn gateway_snapshot(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<ReportExecutionSnapshotCommand>,
) -> Result<Json<GatewaySnapshotResponse>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.snapshot", &command.callback_identity_ref)?;
    validate_command_scope(
        "gateway.snapshot",
        route_app_id,
        &command.snapshot.scope,
        &command.trace,
    )?;
    info!(
        app_id = %command.snapshot.scope.application_id,
        session_id = %command.snapshot.scope.session_id,
        run_id = %command.snapshot.scope.run_id,
        trace_id = %command.trace.trace_id,
        "web route forwarding application execution gateway snapshot through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .report_gateway_snapshot(command)
        .await
        .map(|snapshot| {
            Json(GatewaySnapshotResponse {
                accepted: true,
                snapshot,
            })
        })
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/gateway/approval
pub async fn gateway_approval(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<RequestExecutionApprovalCommand>,
) -> Result<Json<ApplicationExecutionEventEnvelope>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.approval", &command.callback_identity_ref)?;
    validate_command_scope(
        "gateway.approval",
        route_app_id,
        &command.scope,
        &command.trace,
    )?;
    state
        .system_facade
        .application_execution_client()
        .request_gateway_approval(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/gateway/completion
pub async fn gateway_completion(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<ReportExecutionCompletionCommand>,
) -> Result<Json<ApplicationExecutionEventEnvelope>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.completion", &command.callback_identity_ref)?;
    validate_command_scope(
        "gateway.completion",
        route_app_id,
        &command.scope,
        &command.trace,
    )?;
    state
        .system_facade
        .application_execution_client()
        .report_gateway_completion(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/gateway/failure
pub async fn gateway_failure(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<ReportExecutionFailureCommand>,
) -> Result<Json<ApplicationExecutionEventEnvelope>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_callback_identity("gateway.failure", &command.callback_identity_ref)?;
    validate_command_scope(
        "gateway.failure",
        route_app_id,
        &command.scope,
        &command.trace,
    )?;
    state
        .system_facade
        .application_execution_client()
        .report_gateway_failure(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

pub(crate) fn validate_event_scope(
    route_name: &str,
    route_app_id: ApplicationId,
    event: &ApplicationExecutionEventEnvelope,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if event.application_id != route_app_id {
        warn!(
            route_name,
            route_app_id = %route_app_id,
            body_app_id = %event.application_id,
            "application execution gateway rejected mismatched application scope"
        );
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} application scope does not match route app_id"),
        ));
    }
    required_trace(&event.trace.trace_id, route_name)?;
    required_text("session_id", &event.session_id)?;
    required_text("run_id", &event.run_id)?;
    required_text("idempotency_key", &event.idempotency_key)?;
    Ok(())
}

pub(crate) fn validate_callback_identity(
    route_name: &str,
    callback_identity_ref: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    required_text("callback_identity_ref", callback_identity_ref)
        .map(|_| ())
        .map_err(|error| {
            warn!(
                route_name,
                "application execution gateway rejected missing callback identity"
            );
            error
        })
}
