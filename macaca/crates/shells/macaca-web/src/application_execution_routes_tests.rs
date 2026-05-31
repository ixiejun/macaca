//! Tests for the application execution Web adapter boundary.
//!
//! These tests exercise request-shaping helpers instead of running provider
//! code.  That is intentional: Web must reject malformed HTTP scope before it
//! calls the SDK, while provider assignment, EventLog append, and lease
//! validation stay owned by `service.application_execution`.

use macaca_proto::{
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionEventType, ApplicationExecutionProviderKind, ApplicationExecutionScope,
    ApplicationId, EventEntry, TraceContext,
};

use crate::application_execution_gateway_routes::validate_callback_identity;
use crate::application_execution_routes::{
    build_current_state_scope, build_start_command, validate_command_scope, CurrentStateQuery,
    StartExecutionRequest,
};
use crate::application_execution_stream_routes::{
    build_stream_query, event_entry_to_stream_payload, ExecutionEventStreamQuery,
    APPLICATION_EXECUTION_EVENT_SOURCE,
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

#[test]
fn event_stream_requires_explicit_trace_scope() {
    let query = ExecutionEventStreamQuery {
        session_id: "session-a".into(),
        trace_id: " ".into(),
        since: None,
        limit: None,
        event_type: None,
    };

    let error = build_stream_query(&app_id().to_string(), query).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("trace_id"));
}

#[test]
fn event_stream_query_uses_service_owned_source_index() {
    let application_id = app_id();
    let query = ExecutionEventStreamQuery {
        session_id: "session-a".into(),
        trace_id: "trace-a".into(),
        since: Some(41),
        limit: Some(900),
        event_type: Some(ApplicationExecutionEventType::ProviderHeartbeat),
    };

    let (_, trace, event_query) = match build_stream_query(&application_id.to_string(), query) {
        Ok(parts) => parts,
        Err((status, body)) => panic!("unexpected stream query error: {status} {}", body.0.error),
    };

    assert_eq!(trace.trace_id, "trace-a");
    assert_eq!(event_query.session_id, "session-a");
    assert_eq!(event_query.since_seq, 41);
    assert_eq!(event_query.limit, 500);
    assert_eq!(
        event_query.source.as_deref(),
        Some(APPLICATION_EXECUTION_EVENT_SOURCE)
    );
    assert_eq!(event_query.event_type.as_deref(), Some("ProviderHeartbeat"));
}

#[test]
fn event_stream_payload_includes_durable_event_ref() {
    let entry = EventEntry {
        seq: 7,
        timestamp: chrono::Utc::now(),
        session_id: "session-a".into(),
        event_type: "ProviderHeartbeat".into(),
        source: APPLICATION_EXECUTION_EVENT_SOURCE.into(),
        payload: serde_json::json!({
            "application_id": app_id().to_string(),
            "sanitized_payload": {"kind": "summary", "summary": "heartbeat"}
        }),
    };

    let payload = event_entry_to_stream_payload(&entry);

    assert_eq!(payload["seq"], 7);
    assert_eq!(payload["event_ref"]["session_id"], "session-a");
    assert_eq!(payload["event_ref"]["seq"], 7);
    assert_eq!(
        payload["payload"]["sanitized_payload"]["summary"],
        "heartbeat"
    );
}

#[test]
fn event_stream_adapter_has_no_execution_control_side_effects() {
    let source = include_str!("application_execution_stream_routes.rs");

    assert!(!source.contains("send_control("));
    assert!(!source.contains("ApplicationExecutionControlCommand"));
    assert!(!source.contains("ApplicationExecutionProvider"));
    assert!(!source.contains("tokio::spawn"));
}
