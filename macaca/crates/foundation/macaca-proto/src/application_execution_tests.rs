use std::collections::BTreeMap;

use chrono::Utc;

use crate::{
    application_execution_service_descriptor, ApplicationExecutionCommandStatus,
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationId, MacacaError,
    StartApplicationExecutionResult, TraceContext, APPLICATION_EXECUTION_SERVICE_ID,
    APPLICATION_EXECUTION_START_COMMAND,
};

fn trace() -> TraceContext {
    TraceContext::new("trace-application-execution-test")
}

fn scope() -> ApplicationExecutionScope {
    ApplicationExecutionScope::new(
        ApplicationId::from_name("application-execution-protocol-test"),
        "session-1",
        "run-1",
        "tester",
    )
    .unwrap()
}

#[test]
fn provider_kind_round_trips_through_json() {
    let encoded = serde_json::to_string(&ApplicationExecutionProviderKind::RemoteAgent).unwrap();
    let decoded: ApplicationExecutionProviderKind = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, ApplicationExecutionProviderKind::RemoteAgent);
}

#[test]
fn lifecycle_terminal_helper_is_explicit() {
    assert!(!ApplicationExecutionLifecycleState::Running.is_terminal());
    assert!(ApplicationExecutionLifecycleState::Completed.is_terminal());
    assert!(ApplicationExecutionLifecycleState::Failed.is_terminal());
    assert!(ApplicationExecutionLifecycleState::Cancelled.is_terminal());
}

#[test]
fn scope_rejects_blank_session_identity() {
    let err = ApplicationExecutionScope::new(
        ApplicationId::from_name("application-execution-protocol-test"),
        " ",
        "run-1",
        "tester",
    )
    .unwrap_err();
    assert!(matches!(err, MacacaError::Config(_)));
}

#[test]
fn start_result_unavailable_uses_null_object_provider_kind() {
    let result = StartApplicationExecutionResult::unavailable(
        APPLICATION_EXECUTION_START_COMMAND,
        "provider stack disabled",
    );
    assert_eq!(
        result.status,
        ApplicationExecutionCommandStatus::Unavailable
    );
    assert_eq!(
        result.provider_kind,
        ApplicationExecutionProviderKind::Unavailable
    );
    assert_eq!(
        result.error.unwrap().code,
        ApplicationExecutionCommandStatus::Unavailable
    );
}

#[test]
fn event_envelope_preserves_schema_and_idempotency() {
    let envelope = ApplicationExecutionEventEnvelope {
        application_id: ApplicationId::from_name("application-execution-protocol-test"),
        session_id: "session-1".into(),
        run_id: "run-1".into(),
        seq: Some(7),
        timestamp: Utc::now(),
        event_type: ApplicationExecutionEventType::ExecutionAccepted,
        trace: trace(),
        actor: "tester".into(),
        provider_id: "provider-1".into(),
        provider_kind: ApplicationExecutionProviderKind::MacacaHosted,
        visibility: "session".into(),
        causality: vec!["start-command".into()],
        sanitized_payload: ApplicationExecutionPayload::summary("accepted"),
        payload_ref: None,
        schema_version: "application.execution.event.v1".into(),
        idempotency_key: "idem-1".into(),
    };

    let decoded: ApplicationExecutionEventEnvelope =
        serde_json::from_str(&serde_json::to_string(&envelope).unwrap()).unwrap();
    assert_eq!(decoded.schema_version, "application.execution.event.v1");
    assert_eq!(decoded.idempotency_key, "idem-1");
    assert_eq!(decoded.seq, Some(7));
}

#[test]
fn control_command_preserves_scope_and_kind() {
    let command = ApplicationExecutionControlCommand {
        scope: scope(),
        command: ApplicationExecutionControlKind::Approve,
        control_id: "control-1".into(),
        reason_code: "operator_approved".into(),
        trace: trace(),
        policy_context: BTreeMap::new(),
        payload: Some(ApplicationExecutionPayload::summary("approved")),
        idempotency_key: "control-idem-1".into(),
    };

    let decoded: ApplicationExecutionControlCommand =
        serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();
    assert_eq!(decoded.command, ApplicationExecutionControlKind::Approve);
    assert_eq!(decoded.scope.session_id, "session-1");
    assert_eq!(decoded.idempotency_key, "control-idem-1");
}

#[test]
fn descriptor_is_registered_but_unavailable_until_provider_bootstrap() {
    let descriptor = application_execution_service_descriptor();
    assert_eq!(descriptor.id.as_str(), APPLICATION_EXECUTION_SERVICE_ID);
    assert_eq!(
        descriptor.lifecycle_state,
        crate::ServiceLifecycleState::Registered
    );
    assert!(matches!(
        descriptor.health,
        crate::ServiceHealth::Unavailable { .. }
    ));
}
