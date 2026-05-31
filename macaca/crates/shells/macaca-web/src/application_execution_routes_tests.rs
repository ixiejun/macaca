//! Tests for the application execution Web adapter boundary.
//!
//! These tests exercise request-shaping helpers instead of running provider
//! code.  That is intentional: Web must reject malformed HTTP scope before it
//! calls the SDK, while provider assignment, EventLog append, and lease
//! validation stay owned by `service.application_execution`.

use macaca_proto::{
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationId, TraceContext,
};

use crate::application_execution_gateway_routes::validate_callback_identity;
use crate::application_execution_routes::{
    build_current_state_scope, build_start_command, validate_command_scope, CurrentStateQuery,
    StartExecutionRequest,
};

fn app_id() -> ApplicationId {
    ApplicationId(uuid::Uuid::new_v4())
}

#[test]
fn start_request_requires_explicit_trace_scope() {
    let request = StartExecutionRequest {
        task_input: macaca_proto::ApplicationExecutionPayload::summary("safe summary"),
        actor: "operator".into(),
        idempotency_key: "idem-1".into(),
        trace_id: " ".into(),
        ..StartExecutionRequest::default()
    };

    let error = build_start_command(&app_id().to_string(), request).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("trace_id"));
}

#[test]
fn current_state_request_requires_session_scope() {
    let query = CurrentStateQuery {
        trace_id: "trace-a".into(),
        actor: "operator".into(),
        run_id: "run-a".into(),
        session_id: " ".into(),
        tenant_id: None,
    };

    let error = build_current_state_scope(app_id(), query).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("session_id"));
}

#[test]
fn control_route_rejects_path_scope_mismatch() {
    let route_app_id = app_id();
    let body_app_id = app_id();
    let command = ApplicationExecutionControlCommand {
        scope: ApplicationExecutionScope::new(body_app_id, "session-a", "run-a", "operator")
            .unwrap(),
        command: ApplicationExecutionControlKind::Cancel,
        control_id: "control-a".into(),
        reason_code: "operator_requested".into(),
        trace: TraceContext::new("trace-a"),
        policy_context: Default::default(),
        payload: None,
        idempotency_key: "idem-a".into(),
    };

    let error = validate_command_scope("control", route_app_id, &command.scope, &command.trace)
        .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("application scope"));
}

#[test]
fn gateway_append_requires_callback_identity() {
    let envelope = macaca_proto::ApplicationExecutionEventEnvelope {
        application_id: app_id(),
        session_id: "session-a".into(),
        run_id: "run-a".into(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type: macaca_proto::ApplicationExecutionEventType::ProviderHeartbeat,
        trace: TraceContext::new("trace-a"),
        actor: "provider".into(),
        provider_id: "provider-a".into(),
        provider_kind: ApplicationExecutionProviderKind::RemoteAgent,
        visibility: "public".into(),
        causality: Vec::new(),
        sanitized_payload: macaca_proto::ApplicationExecutionPayload::summary("heartbeat"),
        payload_ref: None,
        schema_version: "application.execution.v1".into(),
        idempotency_key: "idem-a".into(),
    };
    let command = macaca_proto::AppendExecutionEventCommand {
        lease_id: Some("lease-a".into()),
        callback_identity_ref: " ".into(),
        event: envelope,
    };

    let error = validate_callback_identity("gateway.append_event", &command.callback_identity_ref)
        .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("callback_identity_ref"));
}
