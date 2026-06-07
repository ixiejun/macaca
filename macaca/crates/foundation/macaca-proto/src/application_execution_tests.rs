use std::collections::BTreeMap;

use chrono::Utc;

use crate::{
    application_execution_service_descriptor, ApplicationExecutionCommandStatus,
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationId, CapabilityId,
    MacacaError, PackageRuntimeKind, ReportExecutionHeartbeatCommand,
    StartApplicationExecutionCommand, StartApplicationExecutionResult, TraceContext,
    APPLICATION_EXECUTION_SERVICE_ID, APPLICATION_EXECUTION_START_COMMAND,
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

/// Build a provider-neutral start command for one runtime shape.
///
/// The helper intentionally keeps the command structure identical for YAML,
/// WASM, and future application runtime adapters.  Runtime differences are
/// carried as policy metadata so `service.application_execution` can select a
/// provider strategy without hardcoding application names, workflow names, or
/// business-domain behavior in the protocol DTO.
fn start_command_for_runtime(runtime_kind: PackageRuntimeKind) -> StartApplicationExecutionCommand {
    let mut policy_context = BTreeMap::new();
    policy_context.insert(
        "application_execution.profile".into(),
        runtime_kind.to_string(),
    );
    policy_context.insert(
        "application_execution.terminal_projection_owner".into(),
        APPLICATION_EXECUTION_SERVICE_ID.into(),
    );
    policy_context.insert(
        "application_execution.task_graph_owner".into(),
        "application_execution".into(),
    );

    StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application-execution-protocol-test"),
        session_id: Some("session-equivalence".into()),
        run_id: Some("run-equivalence".into()),
        task_input: ApplicationExecutionPayload::summary("provider-neutral task input"),
        workspace_ref: Some("workspace-ref".into()),
        requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
        provider_preference: None,
        trace: TraceContext::new("trace-equivalence"),
        policy_context,
        tenant_id: None,
        actor: "adapter".into(),
        idempotency_key: format!("start-{}", runtime_kind),
    }
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
fn start_command_envelope_is_equivalent_for_yaml_and_wasm_runtime_shapes() {
    let yaml = start_command_for_runtime(PackageRuntimeKind::Yaml);
    let wasm = start_command_for_runtime(PackageRuntimeKind::WasmComponent);

    assert_eq!(yaml.application_id, wasm.application_id);
    assert_eq!(yaml.session_id, wasm.session_id);
    assert_eq!(yaml.run_id, wasm.run_id);
    assert_eq!(yaml.workspace_ref, wasm.workspace_ref);
    assert_eq!(yaml.requested_capabilities, wasm.requested_capabilities);
    assert_eq!(yaml.provider_preference, wasm.provider_preference);
    assert_eq!(yaml.trace.trace_id, wasm.trace.trace_id);
    assert_eq!(yaml.actor, wasm.actor);
    assert_eq!(
        yaml.policy_context
            .get("application_execution.terminal_projection_owner"),
        Some(&APPLICATION_EXECUTION_SERVICE_ID.to_string())
    );
    assert_eq!(
        yaml.policy_context
            .get("application_execution.terminal_projection_owner"),
        wasm.policy_context
            .get("application_execution.terminal_projection_owner")
    );
    assert_eq!(
        yaml.policy_context
            .get("application_execution.task_graph_owner"),
        wasm.policy_context
            .get("application_execution.task_graph_owner")
    );
    assert_eq!(
        yaml.policy_context.get("application_execution.profile"),
        Some(&PackageRuntimeKind::Yaml.to_string())
    );
    assert_eq!(
        wasm.policy_context.get("application_execution.profile"),
        Some(&PackageRuntimeKind::WasmComponent.to_string())
    );
}

#[test]
fn start_command_policy_context_stays_provider_neutral_after_json_roundtrip() {
    let yaml = start_command_for_runtime(PackageRuntimeKind::Yaml);
    let decoded: StartApplicationExecutionCommand =
        serde_json::from_str(&serde_json::to_string(&yaml).unwrap()).unwrap();
    let encoded = serde_json::to_string(&decoded).unwrap();

    assert_eq!(
        decoded.policy_context.get("application_execution.profile"),
        Some(&PackageRuntimeKind::Yaml.to_string())
    );
    assert_eq!(
        decoded
            .policy_context
            .get("application_execution.terminal_projection_owner"),
        Some(&APPLICATION_EXECUTION_SERVICE_ID.to_string())
    );
    assert!(
        !encoded.contains("codex-wasm-workbench"),
        "application-execution envelopes must not hardcode application names"
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

#[test]
fn gateway_heartbeat_preserves_schema_provider_and_idempotency_metadata() {
    let command = ReportExecutionHeartbeatCommand {
        lease_id: Some("lease-protocol-1".into()),
        callback_identity_ref: "callback.identity.ref".into(),
        scope: scope(),
        provider_id: "provider-protocol".into(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        reported_at: Utc::now(),
        trace: trace(),
        event_schema_version: "application-execution.v1".into(),
        idempotency_key: "heartbeat-protocol-1".into(),
    };

    let decoded: ReportExecutionHeartbeatCommand =
        serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();

    assert_eq!(decoded.provider_id, "provider-protocol");
    assert_eq!(
        decoded.provider_kind,
        ApplicationExecutionProviderKind::ExternalAppBackend
    );
    assert_eq!(decoded.event_schema_version, "application-execution.v1");
    assert_eq!(decoded.idempotency_key, "heartbeat-protocol-1");
}
