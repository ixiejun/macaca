use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionError,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderLease, ApplicationExecutionScope,
    ApplicationExecutionSnapshot, ApplicationId, ReportExecutionCompletionCommand,
    ReportExecutionFailureCommand, ReportExecutionHeartbeatCommand, ReportExecutionSnapshotCommand,
    RequestExecutionApprovalCommand, ServiceCommand, ServiceCommandName, ServiceError,
    TraceContext, APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND,
};
use tempfile::tempdir;

use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_gateway_events::{
    append_gateway_approval_request, append_gateway_completion, append_gateway_failure,
    append_gateway_heartbeat,
};
use crate::{ApplicationExecutionProviderRegistry, ApplicationExecutionSystemServiceProvider};

const GATEWAY_SCHEMA_VERSION: &str = "application-execution.v1";

#[tokio::test]
async fn gateway_append_is_idempotent_and_replayable() {
    let (_dir, event_log) = event_log();
    let provider = ApplicationExecutionSystemServiceProvider::with_event_log(
        event_log,
        ApplicationExecutionProviderRegistry::new(),
    );
    let trace = TraceContext::new("trace-gateway-append");
    let event = ApplicationExecutionEventEnvelope {
        application_id: ApplicationId::from_name("gateway-neutral-application"),
        session_id: "session-gateway".into(),
        run_id: "run-gateway".into(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type: ApplicationExecutionEventType::ExecutionCompleted,
        trace: trace.clone(),
        actor: "external-provider".into(),
        provider_id: "provider-external".into(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: ApplicationExecutionPayload::summary("completed"),
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key: "gateway-complete-1".into(),
    };

    let command_payload = serde_json::json!({
        "lease_id": null,
        "callback_identity_ref": "test-callback",
        "event": event,
    });
    let first = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND),
            command_payload.clone(),
            trace.clone(),
        ))
        .await
        .unwrap();
    let second = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND),
            command_payload,
            trace,
        ))
        .await
        .unwrap();
    let first_event: ApplicationExecutionEventEnvelope =
        serde_json::from_value(first.output).unwrap();
    let second_event: ApplicationExecutionEventEnvelope =
        serde_json::from_value(second.output).unwrap();

    assert_eq!(first_event.seq, second_event.seq);
    assert_eq!(
        provider.health().await.unwrap(),
        macaca_proto::ServiceHealth::Healthy
    );
}

#[tokio::test]
async fn gateway_snapshot_command_appends_provider_snapshot_event() {
    let (_dir, event_log) = event_log();
    let provider = ApplicationExecutionSystemServiceProvider::with_event_log(
        event_log.clone(),
        ApplicationExecutionProviderRegistry::new(),
    );
    let trace = TraceContext::new("trace-gateway-snapshot");
    let scope = ApplicationExecutionScope::new(
        ApplicationId::from_name("snapshot-neutral-application"),
        "session-snapshot",
        "run-snapshot",
        "external-provider",
    )
    .unwrap();
    let snapshot = ApplicationExecutionSnapshot {
        scope: scope.clone(),
        lifecycle_state: ApplicationExecutionLifecycleState::Running,
        provider_id: Some("provider-external".into()),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        latest_event_cursor: Some("event/7".into()),
        latest_checkpoint_ref: Some("checkpoint://snapshot/7".into()),
        metadata: BTreeMap::from([("diagnostic".into(), "bounded".into())]),
    };

    let reply = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND),
            serde_json::to_value(ReportExecutionSnapshotCommand {
                lease_id: None,
                callback_identity_ref: "test-callback".into(),
                snapshot: snapshot.clone(),
                trace: trace.clone(),
                event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
                idempotency_key: "snapshot-idem-1".into(),
            })
            .unwrap(),
            trace,
        ))
        .await
        .unwrap();
    let typed: ApplicationExecutionSnapshot = serde_json::from_value(reply.output).unwrap();

    assert_eq!(typed, snapshot);
    let replay = event_log.query("session-snapshot", 0, 10).await;
    assert_eq!(replay.len(), 1);
    let event: ApplicationExecutionEventEnvelope =
        serde_json::from_value(replay[0].payload.clone()).unwrap();
    assert_eq!(
        event.event_type,
        ApplicationExecutionEventType::ProviderSnapshot
    );
    assert_eq!(event.provider_id, "provider-external");
    assert_eq!(
        event.provider_kind,
        ApplicationExecutionProviderKind::ExternalAppBackend
    );
}

#[tokio::test]
async fn gateway_helper_callbacks_append_expected_protocol_events() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let scope = gateway_scope("session-helper", "run-helper");
    let trace = TraceContext::new("trace-gateway-helper");

    let heartbeat = append_gateway_heartbeat(
        &store,
        serde_json::to_value(ReportExecutionHeartbeatCommand {
            lease_id: None,
            callback_identity_ref: "callback-helper".into(),
            scope: scope.clone(),
            provider_id: "provider-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            reported_at: chrono::Utc::now(),
            trace: trace.clone(),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "heartbeat-helper-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    let approval = append_gateway_approval_request(
        &store,
        serde_json::to_value(RequestExecutionApprovalCommand {
            lease_id: None,
            callback_identity_ref: "callback-helper".into(),
            scope: scope.clone(),
            provider_id: "provider-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            approval_ref: "approval-helper-1".into(),
            prompt: ApplicationExecutionPayload::summary("approval required"),
            trace: trace.clone(),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "approval-helper-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    let completion = append_gateway_completion(
        &store,
        serde_json::to_value(ReportExecutionCompletionCommand {
            lease_id: None,
            callback_identity_ref: "callback-helper".into(),
            scope: scope.clone(),
            provider_id: "provider-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            result: ApplicationExecutionPayload::summary("completed"),
            trace: trace.clone(),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "completion-helper-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    let failure = append_gateway_failure(
        &store,
        serde_json::to_value(ReportExecutionFailureCommand {
            lease_id: None,
            callback_identity_ref: "callback-helper".into(),
            scope,
            provider_id: "provider-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            error: gateway_error("failure-helper"),
            trace,
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "failure-helper-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        [
            heartbeat.event_type,
            approval.event_type,
            completion.event_type,
            failure.event_type
        ],
        [
            ApplicationExecutionEventType::ProviderHeartbeat,
            ApplicationExecutionEventType::ApprovalRequested,
            ApplicationExecutionEventType::ExecutionCompleted,
            ApplicationExecutionEventType::ExecutionFailed
        ]
    );
    assert_eq!(completion.provider_id, "provider-helper");
    assert_eq!(
        completion.provider_kind,
        ApplicationExecutionProviderKind::ExternalAppBackend
    );
}

#[tokio::test]
async fn gateway_helper_callbacks_reject_schema_and_idempotency_before_append() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let scope = gateway_scope("session-reject", "run-reject");

    let schema_err = append_gateway_heartbeat(
        &store,
        serde_json::to_value(ReportExecutionHeartbeatCommand {
            lease_id: None,
            callback_identity_ref: "callback-reject".into(),
            scope: scope.clone(),
            provider_id: "provider-reject".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            reported_at: chrono::Utc::now(),
            trace: TraceContext::new("trace-schema-reject"),
            event_schema_version: "application-execution.v0".into(),
            idempotency_key: "heartbeat-reject-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        schema_err,
        ServiceError::InvalidArgument(reason) if reason.contains("schema version")
    ));

    let idempotency_err = append_gateway_completion(
        &store,
        serde_json::to_value(ReportExecutionCompletionCommand {
            lease_id: None,
            callback_identity_ref: "callback-reject".into(),
            scope,
            provider_id: "provider-reject".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            result: ApplicationExecutionPayload::summary("completed"),
            trace: TraceContext::new("trace-idempotency-reject"),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: " ".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        idempotency_err,
        ServiceError::InvalidArgument(reason) if reason.contains("idempotency key")
    ));
    assert_eq!(event_log.latest_seq("session-reject").await, 0);
}

#[tokio::test]
async fn gateway_helper_callbacks_enforce_lease_scope_identity_and_allowed_type() {
    let (_dir, event_log) = event_log();
    let scope = gateway_scope("session-lease-helper", "run-lease-helper");
    let store = ApplicationExecutionEventStore::new(event_log.clone()).with_lease(gateway_lease(
        "lease-helper-valid",
        "provider-lease-helper",
        scope.clone(),
        vec![ApplicationExecutionEventType::ExecutionCompleted],
    ));

    let completed = append_gateway_completion(
        &store,
        serde_json::to_value(ReportExecutionCompletionCommand {
            lease_id: Some("lease-helper-valid".into()),
            callback_identity_ref: "callback-lease-helper".into(),
            scope: scope.clone(),
            provider_id: "provider-lease-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            result: ApplicationExecutionPayload::summary("completed"),
            trace: TraceContext::new("trace-lease-complete"),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "lease-complete-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        completed.event_type,
        ApplicationExecutionEventType::ExecutionCompleted
    );

    let rejected = append_gateway_failure(
        &store,
        serde_json::to_value(ReportExecutionFailureCommand {
            lease_id: Some("lease-helper-valid".into()),
            callback_identity_ref: "callback-lease-helper".into(),
            scope,
            provider_id: "provider-lease-helper".into(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            error: gateway_error("failure-not-allowed"),
            trace: TraceContext::new("trace-lease-failure"),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "lease-failure-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        rejected,
        ServiceError::InvalidArgument(reason) if reason.contains("not allowed")
    ));
    assert_eq!(event_log.latest_seq("session-lease-helper").await, 1);
}

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("application-execution.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

fn gateway_scope(session_id: &str, run_id: &str) -> ApplicationExecutionScope {
    ApplicationExecutionScope::new(
        ApplicationId::from_name("gateway-neutral-application"),
        session_id,
        run_id,
        "external-provider",
    )
    .unwrap()
}

fn gateway_error(reason: &str) -> ApplicationExecutionError {
    ApplicationExecutionError {
        code: ApplicationExecutionCommandStatus::ProviderFailed,
        layer: "service.application_execution".into(),
        operation: "gateway.failure".into(),
        application_id: None,
        session_id: None,
        run_id: None,
        provider_id: Some("provider-helper".into()),
        provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
        trace_id: None,
        reason: reason.into(),
        retryable: false,
    }
}

fn gateway_lease(
    lease_id: &str,
    provider_id: &str,
    scope: ApplicationExecutionScope,
    allowed_event_types: Vec<ApplicationExecutionEventType>,
) -> ApplicationExecutionProviderLease {
    ApplicationExecutionProviderLease {
        lease_id: lease_id.into(),
        provider_id: provider_id.into(),
        scope,
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        heartbeat_deadline: chrono::Utc::now() + chrono::Duration::minutes(5),
        callback_identity_ref: "callback-lease-helper".into(),
        allowed_event_types,
        allowed_controls: Vec::new(),
    }
}
