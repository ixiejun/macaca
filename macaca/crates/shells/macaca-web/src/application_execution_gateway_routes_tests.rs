//! Tests for the application execution gateway Web adapter boundary.
//!
//! The Web gateway is a shell adapter, not the durable protocol owner.  These
//! tests therefore focus on the transport-scope checks that the route can know
//! locally and on static guardrails that prevent the shell from appending to
//! EventLog or enforcing provider leases itself.  Lease, schema, idempotency,
//! and sanitization remain service-owned specifications that are exercised in
//! `macaca-runtime-host` tests.

use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationId, TraceContext,
};

use crate::application_execution_gateway_routes::{
    validate_callback_identity, validate_event_scope,
};

fn app_id() -> ApplicationId {
    ApplicationId(uuid::Uuid::new_v4())
}

fn gateway_event(application_id: ApplicationId) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id,
        session_id: "session-gateway-route".into(),
        run_id: "run-gateway-route".into(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type: ApplicationExecutionEventType::ExecutionCompleted,
        trace: TraceContext::new("trace-gateway-route"),
        actor: "external-provider".into(),
        provider_id: "provider-generic".into(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: ApplicationExecutionPayload::summary("completed"),
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key: "idem-gateway-route".into(),
    }
}

#[test]
fn gateway_route_rejects_missing_callback_identity_before_forwarding() {
    let error = validate_callback_identity("gateway.append_event", " ").unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("callback_identity_ref"));
}

#[test]
fn gateway_route_rejects_application_scope_mismatch_before_forwarding() {
    let route_app_id = app_id();
    let body_app_id = app_id();
    let event = gateway_event(body_app_id);

    let error = validate_event_scope("gateway.append_event", route_app_id, &event).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("application scope"));
}

#[test]
fn gateway_route_rejects_missing_idempotency_before_forwarding() {
    let application_id = app_id();
    let mut event = gateway_event(application_id);
    event.idempotency_key.clear();

    let error = validate_event_scope("gateway.append_event", application_id, &event).unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(error.1 .0.error.contains("idempotency_key"));
}

#[test]
fn gateway_route_does_not_own_eventlog_or_lease_semantics() {
    let source = include_str!("application_execution_gateway_routes.rs");

    assert!(source.contains(".append_gateway_event(command)"));
    assert!(!source.contains("append_idempotent("));
    assert!(!source.contains("EventLog"));
    assert!(!source.contains("with_lease("));
}
