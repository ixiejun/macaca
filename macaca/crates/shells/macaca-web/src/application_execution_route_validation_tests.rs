//! Route validation tests for the application execution Web shell.
//!
//! These tests prove that malformed HTTP-facing scope is rejected before the
//! shell calls the SDK.  The assertions intentionally stay at the Adapter
//! boundary: Web validates app/session/trace shape and returns structured
//! errors, while execution ownership, provider lifecycle, EventLog append, and
//! cancel semantics remain owned by `service.application_execution`.

use macaca_proto::{
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind, ApplicationExecutionScope,
    ApplicationId, TraceContext,
};

use crate::application_execution_routes::{
    build_replay_request, build_start_command, validate_command_scope, ReplayQuery,
    StartExecutionRequest,
};
use crate::application_execution_stream_routes::{build_stream_query, ExecutionEventStreamQuery};

fn app_id() -> ApplicationId {
    ApplicationId(uuid::Uuid::new_v4())
}

fn valid_start_request() -> StartExecutionRequest {
    StartExecutionRequest {
        task_input: macaca_proto::ApplicationExecutionPayload::summary("safe summary"),
        actor: "operator".into(),
        idempotency_key: "idem-route-validation".into(),
        trace_id: "trace-route-validation".into(),
        ..StartExecutionRequest::default()
    }
}

#[test]
fn start_route_rejects_invalid_application_scope_with_structured_error() {
    let error = build_start_command("not-a-uuid", valid_start_request()).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("app_id"));
}

#[test]
fn start_route_rejects_invalid_body_fields_before_sdk_call() {
    let request = StartExecutionRequest {
        actor: " ".into(),
        ..valid_start_request()
    };

    let error = build_start_command(&app_id().to_string(), request).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("actor"));
}

#[test]
fn replay_route_rejects_missing_trace_scope() {
    let query = ReplayQuery {
        session_id: "session-replay-route".into(),
        run_id: None,
        from_cursor: None,
        page_size: None,
        event_types: Vec::new(),
        visibility: None,
        trace_id: " ".into(),
    };

    let error = build_replay_request(app_id(), query).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("trace_id"));
}

#[test]
fn stream_route_rejects_missing_session_scope() {
    let query = ExecutionEventStreamQuery {
        session_id: " ".into(),
        trace_id: "trace-stream-route".into(),
        since: None,
        limit: None,
        event_type: None,
    };

    let error = build_stream_query(&app_id().to_string(), query).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("session_id"));
}

#[test]
fn control_route_rejects_missing_trace_scope_before_forwarding() {
    let route_app_id = app_id();
    let command = ApplicationExecutionControlCommand {
        scope: ApplicationExecutionScope::new(
            route_app_id,
            "session-control",
            "run-control",
            "operator",
        )
        .unwrap(),
        command: ApplicationExecutionControlKind::Cancel,
        control_id: "control-route-validation".into(),
        reason_code: "operator_requested".into(),
        trace: TraceContext::new(" "),
        policy_context: Default::default(),
        payload: None,
        idempotency_key: "idem-control-route-validation".into(),
    };

    let error = validate_command_scope("control", route_app_id, &command.scope, &command.trace)
        .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("trace_id"));
}

#[test]
fn subscriber_disconnect_path_has_no_cancel_side_effect() {
    let stream_source = include_str!("application_execution_stream_routes.rs");

    assert!(stream_source.contains("RecvError::Closed"));
    assert!(stream_source.contains("break;"));
    assert!(!stream_source.contains("ApplicationExecutionControlKind::Cancel"));
    assert!(!stream_source.contains("send_control("));
    assert!(!stream_source.contains("ApplicationExecutionControlCommand"));
}
