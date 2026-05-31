use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationExecutionSnapshot,
    ApplicationId, ReportExecutionSnapshotCommand, ServiceCommand, ServiceCommandName,
    TraceContext, APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND,
};
use tempfile::tempdir;

use crate::{ApplicationExecutionProviderRegistry, ApplicationExecutionSystemServiceProvider};

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

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("application-execution.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}
