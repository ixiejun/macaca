use std::sync::Arc;

use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    AppendExecutionEventCommand, ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionPayload, ApplicationExecutionProviderKind,
    ApplicationExecutionProviderLease, ApplicationExecutionReplayRequest,
    ApplicationExecutionScope, ApplicationId, ServiceError, TraceContext,
};
use tempfile::tempdir;

use crate::ApplicationExecutionEventStore;

#[tokio::test]
async fn append_redacts_sensitive_payload_before_replay() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let event = gateway_event(
        "session-redact",
        "run-redact",
        ApplicationExecutionPayload {
            summary: "safe summary".into(),
            data: Some(serde_json::json!({
                "token": "raw-token",
                "nested": {"password": "raw-password", "safe": "visible"}
            })),
            payload_ref: None,
            truncated: false,
        },
        "gateway-redact-1",
    );

    let event = store.append_idempotent(event).await.unwrap();

    let data = event.sanitized_payload.data.unwrap();
    assert_eq!(data["token"], "[redacted]");
    assert_eq!(data["nested"]["password"], "[redacted]");
    assert_eq!(data["nested"]["safe"], "visible");
}

#[tokio::test]
async fn append_rejects_oversized_inline_payload_without_ref() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let event = gateway_event(
        "session-oversized",
        "run-oversized",
        ApplicationExecutionPayload {
            summary: "oversized".into(),
            data: Some(serde_json::json!({"blob": "x".repeat(40 * 1024)})),
            payload_ref: None,
            truncated: false,
        },
        "gateway-oversized-1",
    );

    let err = store.append_idempotent(event).await.unwrap_err();

    assert!(
        matches!(err, ServiceError::InvalidArgument(reason) if reason.contains("inline limit"))
    );
}

#[tokio::test]
async fn duplicate_append_returns_original_event_cursor() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let event = gateway_event(
        "session-duplicate",
        "run-duplicate",
        ApplicationExecutionPayload::summary("completed once"),
        "gateway-duplicate-1",
    );

    let first = store.append_idempotent(event.clone()).await.unwrap();
    let second = store.append_idempotent(event).await.unwrap();

    assert_eq!(first.seq, second.seq);
    assert_eq!(first.idempotency_key, second.idempotency_key);
}

#[tokio::test]
async fn replay_ordering_and_projection_are_stable_after_refresh() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    store
        .append_idempotent(gateway_event(
            "session-replay",
            "run-replay",
            ApplicationExecutionPayload::summary("first"),
            "gateway-replay-1",
        ))
        .await
        .unwrap();
    store
        .append_idempotent(gateway_event(
            "session-replay",
            "run-replay",
            ApplicationExecutionPayload::summary("second"),
            "gateway-replay-2",
        ))
        .await
        .unwrap();

    let request = ApplicationExecutionReplayRequest {
        application_id: ApplicationId::from_name("gateway-neutral-application"),
        session_id: "session-replay".into(),
        run_id: Some("run-replay".into()),
        from_cursor: None,
        page_size: 10,
        event_types: Vec::new(),
        visibility: None,
        trace: TraceContext::new("trace-replay-refresh"),
    };
    let first = store.replay(request.clone()).await.unwrap();
    let second = store.replay(request).await.unwrap();

    assert_eq!(first.events, second.events);
    assert_eq!(first.current_state, second.current_state);
    assert_eq!(
        first
            .events
            .iter()
            .filter_map(|event| event.seq)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[tokio::test]
async fn append_rejects_missing_trace_and_unsupported_schema() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let mut missing_trace = gateway_event(
        "session-invalid",
        "run-invalid",
        ApplicationExecutionPayload::summary("missing trace"),
        "gateway-invalid-1",
    );
    missing_trace.trace.trace_id.clear();
    let mut unsupported_schema = gateway_event(
        "session-invalid",
        "run-invalid",
        ApplicationExecutionPayload::summary("bad schema"),
        "gateway-invalid-2",
    );
    unsupported_schema.schema_version = "application-execution.v0".into();

    assert!(matches!(
        store.append_idempotent(missing_trace).await.unwrap_err(),
        ServiceError::MissingTraceContext
    ));
    assert!(matches!(
        store.append_idempotent(unsupported_schema).await.unwrap_err(),
        ServiceError::InvalidArgument(reason) if reason.contains("unsupported")
    ));
}

#[tokio::test]
async fn gateway_append_rejects_stale_lease_before_eventlog_side_effects() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone()).with_lease(provider_lease(
        "lease-stale",
        chrono::Utc::now() - chrono::Duration::seconds(5),
    ));
    let command = AppendExecutionEventCommand {
        lease_id: Some("lease-stale".into()),
        callback_identity_ref: "callback-ref".into(),
        event: gateway_event(
            "session-lease",
            "run-lease",
            ApplicationExecutionPayload::summary("stale lease event"),
            "gateway-stale-lease-1",
        ),
    };

    let err = store.append_gateway_event(command).await.unwrap_err();

    assert!(matches!(
        err,
        ServiceError::InvalidArgument(reason) if reason.contains("stale lease")
    ));
    assert_eq!(
        event_log.latest_seq("session-lease").await,
        0,
        "stale lease validation must run before durable EventLog append"
    );
}

#[tokio::test]
async fn gateway_append_rejects_unsupported_schema_before_eventlog_side_effects() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let mut event = gateway_event(
        "session-schema",
        "run-schema",
        ApplicationExecutionPayload::summary("unsupported schema"),
        "gateway-schema-1",
    );
    event.schema_version = "application-execution.v0".into();
    let command = AppendExecutionEventCommand {
        lease_id: None,
        callback_identity_ref: "callback-ref".into(),
        event,
    };

    let err = store.append_gateway_event(command).await.unwrap_err();

    assert!(matches!(
        err,
        ServiceError::InvalidArgument(reason) if reason.contains("unsupported")
    ));
    assert_eq!(
        event_log.latest_seq("session-schema").await,
        0,
        "schema validation must run before durable EventLog append"
    );
}

#[tokio::test]
async fn gateway_append_rejects_lease_scope_identity_and_event_type_mismatch() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log).with_lease(provider_lease(
        "lease-valid",
        chrono::Utc::now() + chrono::Duration::minutes(5),
    ));
    let mut event = gateway_event(
        "session-other",
        "run-lease",
        ApplicationExecutionPayload::summary("wrong session"),
        "gateway-lease-mismatch-1",
    );
    event.event_type = ApplicationExecutionEventType::ProviderHeartbeat;
    let command = AppendExecutionEventCommand {
        lease_id: Some("lease-valid".into()),
        callback_identity_ref: "wrong-callback".into(),
        event,
    };

    let err = store.append_gateway_event(command).await.unwrap_err();

    assert!(matches!(
        err,
        ServiceError::InvalidArgument(reason)
            if reason.contains("callback identity")
                || reason.contains("session/run binding")
                || reason.contains("event type")
    ));
}

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("application-execution.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

fn gateway_event(
    session_id: &str,
    run_id: &str,
    sanitized_payload: ApplicationExecutionPayload,
    idempotency_key: &str,
) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id: ApplicationId::from_name("gateway-neutral-application"),
        session_id: session_id.into(),
        run_id: run_id.into(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type: ApplicationExecutionEventType::ExecutionCompleted,
        trace: TraceContext::new(format!("trace-{idempotency_key}")),
        actor: "external-provider".into(),
        provider_id: "provider-external".into(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload,
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key: idempotency_key.into(),
    }
}

fn provider_lease(
    lease_id: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> ApplicationExecutionProviderLease {
    ApplicationExecutionProviderLease {
        lease_id: lease_id.into(),
        provider_id: "provider-external".into(),
        scope: ApplicationExecutionScope {
            application_id: ApplicationId::from_name("gateway-neutral-application"),
            session_id: "session-lease".into(),
            run_id: "run-lease".into(),
            tenant_id: None,
            actor: "external-provider".into(),
        },
        expires_at,
        heartbeat_deadline: expires_at,
        callback_identity_ref: "callback-ref".into(),
        allowed_event_types: vec![ApplicationExecutionEventType::ExecutionCompleted],
        allowed_controls: Vec::new(),
    }
}
