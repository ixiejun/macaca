//! End-to-end tests for the external application backend provider strategy.
//!
//! These tests keep the external backend fake at the transport boundary.  The
//! provider still builds real protocol leases, gateway callbacks still pass
//! through the EventLog adapter, and controls still flow back through the same
//! provider Strategy used by production composition.  The fake never encodes an
//! application workflow; it only records generic protocol envelopes.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    AppendExecutionEventCommand, ApplicationExecutionCommandStatus,
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionPayload,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionProviderLease, ApplicationExecutionScope, ApplicationId, CapabilityId,
    ExternalApplicationBackendExecutionProfile, ReportExecutionCompletionCommand,
    RequestExecutionApprovalCommand, ServiceError, TraceContext,
};
use tempfile::tempdir;

use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_gateway_events::{
    append_gateway_approval_request, append_gateway_completion,
};
use crate::{
    ApplicationExecutionProvider, ExternalApplicationBackendControlRequest,
    ExternalApplicationBackendProvider, ExternalApplicationBackendStartRequest,
    ExternalApplicationBackendTransport,
};

const GATEWAY_SCHEMA_VERSION: &str = "application-execution.v1";

#[tokio::test]
async fn fake_external_backend_start_gateway_approval_completion_and_duplicate_are_replayable() {
    let (_dir, event_log) = event_log();
    let transport = Arc::new(ExternalBackendCycleTransport::default());
    let provider =
        ExternalApplicationBackendProvider::with_transport(external_profile(), transport.clone())
            .expect("external backend profile should be admitted");

    let start_result = provider.start(start_command()).await.unwrap();
    let start_request = transport.last_start();
    let scope = external_scope();
    let store = ApplicationExecutionEventStore::new(event_log.clone())
        .with_lease(lease_from_start(&start_request, scope.clone()));

    assert_eq!(
        start_result.status,
        ApplicationExecutionCommandStatus::Accepted
    );
    assert_eq!(start_request.lease_id.len() > "lease-".len(), true);

    let direct_event = store
        .append_gateway_event(AppendExecutionEventCommand {
            lease_id: Some(start_request.lease_id.clone()),
            callback_identity_ref: start_request.callback_identity_ref.clone(),
            event: gateway_event(
                &scope,
                ApplicationExecutionEventType::ProviderSnapshot,
                "direct-gateway-event-1",
            ),
        })
        .await
        .unwrap();
    let approval_event = append_gateway_approval_request(
        &store,
        serde_json::to_value(RequestExecutionApprovalCommand {
            lease_id: Some(start_request.lease_id.clone()),
            callback_identity_ref: start_request.callback_identity_ref.clone(),
            scope: scope.clone(),
            provider_id: start_request.provider_id(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            approval_ref: "approval-cycle-1".into(),
            prompt: ApplicationExecutionPayload::summary("approval required"),
            trace: TraceContext::new("trace-cycle-approval"),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "approval-cycle-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();

    let control_result = provider.control(approval_control()).await.unwrap();
    let delivered_control = transport.last_control();
    assert_eq!(
        control_result.status,
        ApplicationExecutionCommandStatus::Delivered
    );
    assert_eq!(
        delivered_control.lease_id.as_deref(),
        Some(start_request.lease_id.as_str())
    );

    let completion_payload = serde_json::to_value(ReportExecutionCompletionCommand {
        lease_id: Some(start_request.lease_id.clone()),
        callback_identity_ref: start_request.callback_identity_ref.clone(),
        scope: scope.clone(),
        provider_id: start_request.provider_id(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        result: ApplicationExecutionPayload::summary("completed"),
        trace: TraceContext::new("trace-cycle-completion"),
        event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
        idempotency_key: "completion-cycle-1".into(),
    })
    .unwrap();
    let first_completion = append_gateway_completion(&store, completion_payload.clone())
        .await
        .unwrap();
    let duplicate_completion = append_gateway_completion(&store, completion_payload)
        .await
        .unwrap();
    let replayed = event_log.query(&scope.session_id, 0, 10).await;

    assert_eq!(
        [
            direct_event.event_type,
            approval_event.event_type,
            first_completion.event_type,
        ],
        [
            ApplicationExecutionEventType::ProviderSnapshot,
            ApplicationExecutionEventType::ApprovalRequested,
            ApplicationExecutionEventType::ExecutionCompleted,
        ]
    );
    assert_eq!(first_completion.seq, duplicate_completion.seq);
    assert_eq!(
        replayed.len(),
        3,
        "duplicate callback must reuse the durable event instead of appending another row"
    );
}

#[tokio::test]
async fn fake_external_backend_rejects_invalid_callback_identity_before_append() {
    let (_dir, event_log) = event_log();
    let transport = Arc::new(ExternalBackendCycleTransport::default());
    let provider =
        ExternalApplicationBackendProvider::with_transport(external_profile(), transport.clone())
            .expect("external backend profile should be admitted");
    let _ = provider.start(start_command()).await.unwrap();
    let start_request = transport.last_start();
    let scope = external_scope();
    let store = ApplicationExecutionEventStore::new(event_log.clone())
        .with_lease(lease_from_start(&start_request, scope.clone()));

    let rejected = append_gateway_completion(
        &store,
        serde_json::to_value(ReportExecutionCompletionCommand {
            lease_id: Some(start_request.lease_id.clone()),
            callback_identity_ref: "wrong-callback-identity".into(),
            scope: scope.clone(),
            provider_id: start_request.provider_id(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            result: ApplicationExecutionPayload::summary("completed"),
            trace: TraceContext::new("trace-invalid-callback-identity"),
            event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
            idempotency_key: "completion-invalid-identity-1".into(),
        })
        .unwrap(),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        rejected,
        ServiceError::InvalidArgument(reason) if reason.contains("callback identity")
    ));
    assert_eq!(event_log.latest_seq(&scope.session_id).await, 0);
}

#[tokio::test]
async fn fake_external_backend_times_out_when_heartbeat_is_lost() {
    let transport = Arc::new(ExternalBackendCycleTransport::default());
    let mut profile = external_profile();
    profile.heartbeat_policy.timeout_ms = 1;
    profile.request_timeout_ms = 10_000;
    let provider = ExternalApplicationBackendProvider::with_transport(profile, transport)
        .expect("external backend profile should be admitted");
    let _ = provider.start(start_command()).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let health = provider.health().await.unwrap();
    let snapshot = provider
        .snapshot()
        .await
        .unwrap()
        .expect("lost heartbeat should produce stale diagnostics");

    assert!(matches!(
        health,
        ApplicationExecutionProviderHealth::Unavailable { ref reason }
            if reason.contains("heartbeat deadline expired")
    ));
    assert_eq!(
        snapshot
            .metadata
            .get("failure_event_required")
            .map(String::as_str),
        Some("true")
    );
}

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("application-execution.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

fn external_profile() -> ExternalApplicationBackendExecutionProfile {
    ExternalApplicationBackendExecutionProfile {
        provider_id: "provider.external.cycle".into(),
        start_endpoint: "https://backend.example.test/start".into(),
        control_endpoint: Some("https://backend.example.test/control".into()),
        protocol_version: GATEWAY_SCHEMA_VERSION.into(),
        callback_gateway_ref: Some("gateway/application-execution".into()),
        callback_identity_ref: "identity.external.cycle".into(),
        supported_controls: vec![ApplicationExecutionControlKind::Approve],
        heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
            interval_ms: 1_000,
            timeout_ms: 5_000,
            required: true,
        },
        request_timeout_ms: 3_000,
        event_schema_version: GATEWAY_SCHEMA_VERSION.into(),
        capability_declarations: vec![CapabilityId::new("capability.application_execution")],
        resource_profile: Default::default(),
    }
}

fn start_command() -> macaca_proto::StartApplicationExecutionCommand {
    macaca_proto::StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application.external-cycle"),
        session_id: Some("session-external-cycle".into()),
        run_id: Some("run-external-cycle".into()),
        task_input: ApplicationExecutionPayload::summary("start external backend cycle"),
        workspace_ref: Some("workspace://session-external-cycle".into()),
        requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
        provider_preference: None,
        trace: TraceContext::new("trace-external-cycle-start"),
        policy_context: Default::default(),
        tenant_id: Some("tenant-external-cycle".into()),
        actor: "runtime-test".into(),
        idempotency_key: "start-external-cycle-1".into(),
    }
}

fn external_scope() -> ApplicationExecutionScope {
    ApplicationExecutionScope {
        application_id: ApplicationId::from_name("application.external-cycle"),
        session_id: "session-external-cycle".into(),
        run_id: "run-external-cycle".into(),
        tenant_id: Some("tenant-external-cycle".into()),
        actor: "runtime-test".into(),
    }
}

fn approval_control() -> ApplicationExecutionControlCommand {
    ApplicationExecutionControlCommand {
        scope: external_scope(),
        command: ApplicationExecutionControlKind::Approve,
        control_id: "approval-cycle-1".into(),
        reason_code: "approval.accepted".into(),
        trace: TraceContext::new("trace-cycle-control"),
        policy_context: Default::default(),
        payload: Some(ApplicationExecutionPayload::summary("approved")),
        idempotency_key: "control-approval-cycle-1".into(),
    }
}

fn gateway_event(
    scope: &ApplicationExecutionScope,
    event_type: ApplicationExecutionEventType,
    idempotency_key: &str,
) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id: scope.application_id,
        session_id: scope.session_id.clone(),
        run_id: scope.run_id.clone(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type,
        trace: TraceContext::new(format!("trace-{idempotency_key}")),
        actor: scope.actor.clone(),
        provider_id: "provider.external.cycle".into(),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: ApplicationExecutionPayload::summary("direct gateway event"),
        payload_ref: None,
        schema_version: GATEWAY_SCHEMA_VERSION.into(),
        idempotency_key: idempotency_key.into(),
    }
}

fn lease_from_start(
    request: &ExternalApplicationBackendStartRequest,
    scope: ApplicationExecutionScope,
) -> ApplicationExecutionProviderLease {
    ApplicationExecutionProviderLease {
        lease_id: request.lease_id.clone(),
        provider_id: request.provider_id(),
        scope,
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        heartbeat_deadline: chrono::Utc::now() + chrono::Duration::minutes(5),
        callback_identity_ref: request.callback_identity_ref.clone(),
        allowed_event_types: request.allowed_event_types.clone(),
        allowed_controls: request.allowed_controls.clone(),
    }
}

trait StartRequestProviderId {
    fn provider_id(&self) -> String;
}

impl StartRequestProviderId for ExternalApplicationBackendStartRequest {
    fn provider_id(&self) -> String {
        "provider.external.cycle".into()
    }
}

#[derive(Default)]
struct ExternalBackendCycleTransport {
    starts: Mutex<Vec<ExternalApplicationBackendStartRequest>>,
    controls: Mutex<Vec<ExternalApplicationBackendControlRequest>>,
}

impl ExternalBackendCycleTransport {
    fn last_start(&self) -> ExternalApplicationBackendStartRequest {
        self.starts
            .lock()
            .expect("fake transport lock should not be poisoned")
            .last()
            .expect("start request should be recorded")
            .clone()
    }

    fn last_control(&self) -> ExternalApplicationBackendControlRequest {
        self.controls
            .lock()
            .expect("fake transport lock should not be poisoned")
            .last()
            .expect("control request should be recorded")
            .clone()
    }
}

#[async_trait]
impl ExternalApplicationBackendTransport for ExternalBackendCycleTransport {
    async fn start(
        &self,
        request: ExternalApplicationBackendStartRequest,
    ) -> Result<(), ServiceError> {
        self.starts
            .lock()
            .expect("fake transport lock should not be poisoned")
            .push(request);
        Ok(())
    }

    async fn control(
        &self,
        request: ExternalApplicationBackendControlRequest,
    ) -> Result<(), ServiceError> {
        self.controls
            .lock()
            .expect("fake transport lock should not be poisoned")
            .push(request);
        Ok(())
    }
}
