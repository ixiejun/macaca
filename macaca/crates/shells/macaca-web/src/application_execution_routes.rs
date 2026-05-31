//! Thin Web Shell adapters for `service.application_execution`.
//!
//! The Web crate owns HTTP parsing and response mapping only.  It does not run
//! provider loops, construct provider strategies, or append authoritative
//! execution facts directly.  Every handler in this module converts a bounded
//! HTTP request into a provider-neutral SDK command, calls the focused
//! application-execution client, logs sanitized routing evidence, and returns
//! the typed service result.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tracing::{info, warn};

use macaca_proto::{
    ApplicationExecutionControlCommand, ApplicationExecutionControlResult,
    ApplicationExecutionCurrentState, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderPreference, ApplicationExecutionReplayRequest,
    ApplicationExecutionReplayResult, ApplicationExecutionScope, ApplicationId, CapabilityId,
    StartApplicationExecutionCommand, StartApplicationExecutionResult, TraceContext,
};

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

/// HTTP body accepted by the start route.
///
/// The route-level shape intentionally mirrors the protocol command while
/// taking `application_id` from the URL.  That keeps application scope explicit
/// and prevents clients from smuggling a different app id in the JSON body.
#[derive(Debug, Deserialize)]
pub struct StartExecutionRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub task_input: ApplicationExecutionPayload,
    #[serde(default)]
    pub workspace_ref: Option<String>,
    #[serde(default)]
    pub requested_capabilities: Vec<CapabilityId>,
    #[serde(default)]
    pub provider_preference: Option<ApplicationExecutionProviderPreference>,
    #[serde(default)]
    pub policy_context: BTreeMap<String, String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub actor: String,
    pub idempotency_key: String,
    pub trace_id: String,
}

impl Default for StartExecutionRequest {
    fn default() -> Self {
        Self {
            session_id: None,
            run_id: None,
            task_input: ApplicationExecutionPayload::summary("empty"),
            workspace_ref: None,
            requested_capabilities: Vec::new(),
            provider_preference: None,
            policy_context: BTreeMap::new(),
            tenant_id: None,
            actor: String::new(),
            idempotency_key: String::new(),
            trace_id: String::new(),
        }
    }
}

/// Query used by the current-state route.
#[derive(Debug, Deserialize)]
pub struct CurrentStateQuery {
    pub session_id: String,
    pub run_id: String,
    pub actor: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub trace_id: String,
}

/// Query used by the replay route.
#[derive(Debug, Deserialize)]
pub struct ReplayQuery {
    pub session_id: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub from_cursor: Option<String>,
    #[serde(default)]
    pub page_size: Option<usize>,
    #[serde(default)]
    pub event_types: Vec<ApplicationExecutionEventType>,
    #[serde(default)]
    pub visibility: Option<String>,
    pub trace_id: String,
}

/// POST /api/apps/{app_id}/execution/start
pub async fn start_execution(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(request): Json<StartExecutionRequest>,
) -> Result<Json<StartApplicationExecutionResult>, (StatusCode, Json<ErrorResponse>)> {
    let command = build_start_command(&app_id, request)?;
    info!(
        app_id = %command.application_id,
        session_id = ?command.session_id,
        run_id = ?command.run_id,
        trace_id = %command.trace.trace_id,
        "web route forwarding application execution start through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .start_execution(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// POST /api/apps/{app_id}/execution/control
pub async fn send_control(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(command): Json<ApplicationExecutionControlCommand>,
) -> Result<Json<ApplicationExecutionControlResult>, (StatusCode, Json<ErrorResponse>)> {
    let route_app_id = parse_application_id(&app_id)?;
    validate_command_scope("control", route_app_id, &command.scope, &command.trace)?;
    info!(
        app_id = %command.scope.application_id,
        session_id = %command.scope.session_id,
        run_id = %command.scope.run_id,
        trace_id = %command.trace.trace_id,
        control_id = %command.control_id,
        "web route forwarding application execution control through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .send_control(command)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// GET /api/apps/{app_id}/execution/current-state
pub async fn query_current_state(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<CurrentStateQuery>,
) -> Result<Json<ApplicationExecutionCurrentState>, (StatusCode, Json<ErrorResponse>)> {
    let application_id = parse_application_id(&app_id)?;
    let trace = required_trace(&query.trace_id, "current-state")?;
    let scope = build_current_state_scope(application_id, query)?;
    info!(
        app_id = %scope.application_id,
        session_id = %scope.session_id,
        run_id = %scope.run_id,
        trace_id = %trace.trace_id,
        "web route forwarding application execution current-state through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .query_current_state(trace, scope)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

/// GET /api/apps/{app_id}/execution/replay
pub async fn replay_events(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<ReplayQuery>,
) -> Result<Json<ApplicationExecutionReplayResult>, (StatusCode, Json<ErrorResponse>)> {
    let application_id = parse_application_id(&app_id)?;
    let request = build_replay_request(application_id, query)?;
    info!(
        app_id = %request.application_id,
        session_id = %request.session_id,
        run_id = ?request.run_id,
        trace_id = %request.trace.trace_id,
        "web route forwarding application execution replay through SDK"
    );
    state
        .system_facade
        .application_execution_client()
        .replay_events(request)
        .await
        .map(Json)
        .map_err(|error| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &error))
}

pub(crate) fn build_start_command(
    app_id: &str,
    request: StartExecutionRequest,
) -> Result<StartApplicationExecutionCommand, (StatusCode, Json<ErrorResponse>)> {
    let application_id = parse_application_id(app_id)?;
    let trace = required_trace(&request.trace_id, "start")?;
    let actor = required_text("actor", &request.actor)?;
    let idempotency_key = required_text("idempotency_key", &request.idempotency_key)?;
    Ok(StartApplicationExecutionCommand {
        application_id,
        session_id: normalize_optional("session_id", request.session_id)?,
        run_id: normalize_optional("run_id", request.run_id)?,
        task_input: request.task_input,
        workspace_ref: normalize_optional("workspace_ref", request.workspace_ref)?,
        requested_capabilities: request.requested_capabilities,
        provider_preference: request.provider_preference,
        trace,
        policy_context: request.policy_context,
        tenant_id: normalize_optional("tenant_id", request.tenant_id)?,
        actor,
        idempotency_key,
    })
}

pub(crate) fn build_current_state_scope(
    application_id: ApplicationId,
    query: CurrentStateQuery,
) -> Result<ApplicationExecutionScope, (StatusCode, Json<ErrorResponse>)> {
    let mut scope = ApplicationExecutionScope::new(
        application_id,
        required_text("session_id", &query.session_id)?,
        required_text("run_id", &query.run_id)?,
        required_text("actor", &query.actor)?,
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    scope.tenant_id = normalize_optional("tenant_id", query.tenant_id)?;
    Ok(scope)
}

pub(crate) fn build_replay_request(
    application_id: ApplicationId,
    query: ReplayQuery,
) -> Result<ApplicationExecutionReplayRequest, (StatusCode, Json<ErrorResponse>)> {
    Ok(ApplicationExecutionReplayRequest {
        application_id,
        session_id: required_text("session_id", &query.session_id)?,
        run_id: normalize_optional("run_id", query.run_id)?,
        from_cursor: normalize_optional("from_cursor", query.from_cursor)?,
        page_size: query.page_size.unwrap_or(100).clamp(1, 500),
        event_types: query.event_types,
        visibility: normalize_optional("visibility", query.visibility)?,
        trace: required_trace(&query.trace_id, "replay")?,
    })
}

pub(crate) fn validate_command_scope(
    route_name: &str,
    route_app_id: ApplicationId,
    scope: &ApplicationExecutionScope,
    trace: &TraceContext,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if scope.application_id != route_app_id {
        warn!(
            route_name,
            route_app_id = %route_app_id,
            body_app_id = %scope.application_id,
            "application execution route rejected mismatched application scope"
        );
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} application scope does not match route app_id"),
        ));
    }
    required_trace(&trace.trace_id, route_name)?;
    required_text("session_id", &scope.session_id)?;
    required_text("run_id", &scope.run_id)?;
    required_text("actor", &scope.actor)?;
    Ok(())
}

pub(crate) fn parse_application_id(
    app_id: &str,
) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    let id = uuid::Uuid::parse_str(app_id.trim()).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "app_id must be a valid application UUID".into(),
        )
    })?;
    Ok(ApplicationId(id))
}

pub(crate) fn required_trace(
    trace_id: &str,
    route_name: &str,
) -> Result<TraceContext, (StatusCode, Json<ErrorResponse>)> {
    let trace_id = required_text("trace_id", trace_id)?;
    info!(
        route_name,
        trace_id = %trace_id,
        "application execution web route accepted explicit trace scope"
    );
    Ok(TraceContext::new(trace_id))
}

pub(crate) fn required_text(
    name: &str,
    value: &str,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("application execution {name} must not be empty"),
        ));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn normalize_optional(
    name: &str,
    value: Option<String>,
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    value.map(|raw| required_text(name, &raw)).transpose()
}
