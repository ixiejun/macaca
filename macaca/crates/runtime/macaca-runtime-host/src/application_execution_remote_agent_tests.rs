use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    AppendExecutionEventCommand, ApplicationExecutionCommandStatus,
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionPayload,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionScope, ApplicationId, CapabilityId, ServiceError,
    StartApplicationExecutionCommand, TraceContext,
};
use tempfile::tempdir;
use tokio::sync::Mutex;

use crate::{
    ApplicationExecutionEventStore, ApplicationExecutionProvider,
    RemoteAgentApplicationExecutionProvider, RemoteAgentExecutionTransport,
    RemoteAgentRegistration, RemoteAgentRegistry, RemoteAgentTransportMetadata,
};

#[tokio::test]
async fn remote_agent_provider_registers_selects_leases_and_routes_control() {
    let (_dir, event_log) = event_log();
    let registry = Arc::new(RemoteAgentRegistry::new());
    registry.register(remote_agent()).await.unwrap();
    let transport = Arc::new(FakeRemoteTransport::default());
    let provider = RemoteAgentApplicationExecutionProvider::new(
        ApplicationExecutionEventStore::new(event_log),
        Arc::clone(&registry),
        transport.clone(),
    );
    let start = start_command("session-remote", "run-remote", "remote-start-1");

    let result = provider.start(start.clone()).await.unwrap();

    assert_eq!(result.status, ApplicationExecutionCommandStatus::Accepted);
    assert_eq!(
        result.provider_kind,
        ApplicationExecutionProviderKind::RemoteAgent
    );
    assert!(result.control_ref.unwrap().starts_with("remote-agent://"));
    assert_eq!(transport.starts.lock().await.len(), 1);

    let control = ApplicationExecutionControlCommand {
        scope: ApplicationExecutionScope {
            application_id: start.application_id,
            session_id: "session-remote".into(),
            run_id: "run-remote".into(),
            tenant_id: None,
            actor: "tester".into(),
        },
        command: ApplicationExecutionControlKind::Cancel,
        control_id: "control-cancel-1".into(),
        reason_code: "test_cancel".into(),
        trace: TraceContext::new("trace-remote-cancel"),
        policy_context: BTreeMap::new(),
        payload: None,
        idempotency_key: "remote-control-1".into(),
    };
    let control_result = provider.control(control).await.unwrap();

    assert_eq!(
        control_result.status,
        ApplicationExecutionCommandStatus::Delivered
    );
    assert_eq!(transport.controls.lock().await.len(), 1);
}

#[tokio::test]
async fn remote_agent_gateway_rejects_stale_lease_and_accepts_registered_lease() {
    let (_dir, event_log) = event_log();
    let registry = Arc::new(RemoteAgentRegistry::new());
    registry.register(remote_agent()).await.unwrap();
    let provider = RemoteAgentApplicationExecutionProvider::new(
        ApplicationExecutionEventStore::new(event_log),
        Arc::clone(&registry),
        Arc::new(FakeRemoteTransport::default()),
    );
    let start = start_command("session-gateway", "run-gateway", "remote-gateway-start");
    let start_result = provider.start(start.clone()).await.unwrap();
    let lease_id = start_result
        .control_ref
        .unwrap()
        .trim_start_matches("remote-agent://")
        .to_string();

    let stale = provider
        .append_gateway_event(AppendExecutionEventCommand {
            lease_id: Some("missing-lease".into()),
            callback_identity_ref: "remote-callback".into(),
            event: remote_event("session-gateway", "run-gateway", "remote-gateway-stale"),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(stale, ServiceError::InvalidArgument(reason) if reason.contains("stale lease"))
    );

    let accepted = provider
        .append_gateway_event(AppendExecutionEventCommand {
            lease_id: Some(lease_id),
            callback_identity_ref: "remote-callback".into(),
            event: remote_event("session-gateway", "run-gateway", "remote-gateway-ok"),
        })
        .await
        .unwrap();
    assert_eq!(
        accepted.event_type,
        ApplicationExecutionEventType::ProviderHeartbeat
    );
}

#[derive(Default)]
struct FakeRemoteTransport {
    starts: Mutex<Vec<String>>,
    controls: Mutex<Vec<ApplicationExecutionControlKind>>,
}

#[async_trait]
impl RemoteAgentExecutionTransport for FakeRemoteTransport {
    async fn start(
        &self,
        agent: RemoteAgentRegistration,
        _lease: macaca_proto::ApplicationExecutionProviderLease,
        _command: StartApplicationExecutionCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError> {
        self.starts.lock().await.push(agent.agent_id);
        Ok(ApplicationExecutionCommandStatus::Accepted)
    }

    async fn control(
        &self,
        _agent: RemoteAgentRegistration,
        _lease: macaca_proto::ApplicationExecutionProviderLease,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError> {
        self.controls.lock().await.push(command.command);
        Ok(ApplicationExecutionCommandStatus::Delivered)
    }
}

fn remote_agent() -> RemoteAgentRegistration {
    RemoteAgentRegistration {
        agent_id: "remote-agent-1".into(),
        protocol_version: "application-execution.v1".into(),
        transport: RemoteAgentTransportMetadata {
            transport_kind: "test-transport".into(),
            endpoint_ref: "remote://agent-1".into(),
            callback_identity_ref: "remote-callback".into(),
            region: Some("local".into()),
        },
        supported_controls: vec![ApplicationExecutionControlKind::Cancel],
        supported_events: vec![ApplicationExecutionEventType::ProviderHeartbeat],
        heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
            interval_ms: 1_000,
            timeout_ms: 30_000,
            required: true,
        },
        checkpoint_support: true,
        capability_declarations: vec![CapabilityId::new("capability.application_execution")],
        resource_profile: BTreeMap::new(),
        health_state: ApplicationExecutionProviderHealth::Healthy,
        max_leases: 4,
        tenant_id: None,
    }
}

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("remote-agent.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

fn start_command(
    session_id: &str,
    run_id: &str,
    idempotency_key: &str,
) -> StartApplicationExecutionCommand {
    StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("remote-neutral-application"),
        session_id: Some(session_id.into()),
        run_id: Some(run_id.into()),
        task_input: ApplicationExecutionPayload::summary("generic remote task"),
        workspace_ref: Some("workspace://session".into()),
        requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
        provider_preference: None,
        trace: TraceContext::new(format!("trace-{idempotency_key}")),
        policy_context: BTreeMap::new(),
        tenant_id: None,
        actor: "tester".into(),
        idempotency_key: idempotency_key.into(),
    }
}

fn remote_event(
    session_id: &str,
    run_id: &str,
    idempotency_key: &str,
) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id: ApplicationId::from_name("remote-neutral-application"),
        session_id: session_id.into(),
        run_id: run_id.into(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type: ApplicationExecutionEventType::ProviderHeartbeat,
        trace: TraceContext::new(format!("trace-{idempotency_key}")),
        actor: "remote-agent".into(),
        provider_id: "remote-agent-1".into(),
        provider_kind: ApplicationExecutionProviderKind::RemoteAgent,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: ApplicationExecutionPayload::summary("remote heartbeat"),
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key: idempotency_key.into(),
    }
}
