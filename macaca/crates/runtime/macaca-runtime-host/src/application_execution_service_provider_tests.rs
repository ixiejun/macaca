use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionControlResult,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionPayload,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionReplayRequest,
    ApplicationExecutionReplayResult, ApplicationId, CapabilityId, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ServiceError, StartApplicationExecutionCommand,
    StartApplicationExecutionResult, TraceContext,
    APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND, APPLICATION_EXECUTION_REPLAY_COMMAND,
    APPLICATION_EXECUTION_SERVICE_ID, APPLICATION_EXECUTION_START_COMMAND,
};
use tempfile::tempdir;

use crate::{
    bootstrap_unavailable_application_execution_service, ApplicationExecutionProvider,
    ApplicationExecutionProviderRegistry, ApplicationExecutionSystemServiceProvider,
    ServiceRuntime, ServiceRuntimeConfig,
};

#[tokio::test]
async fn unavailable_provider_returns_structured_start_result() {
    let provider = ApplicationExecutionSystemServiceProvider::unavailable("disabled for test");
    let trace = TraceContext::new("trace-application-execution-provider-test");
    let command = StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application-execution-provider-test"),
        session_id: Some("session-1".into()),
        run_id: Some("run-1".into()),
        task_input: ApplicationExecutionPayload::summary("test"),
        workspace_ref: None,
        requested_capabilities: Vec::new(),
        provider_preference: None,
        trace: trace.clone(),
        policy_context: Default::default(),
        tenant_id: None,
        actor: "runtime-test".into(),
        idempotency_key: "start-idem-1".into(),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_START_COMMAND),
            serde_json::to_value(command).unwrap(),
            trace,
        ))
        .await
        .unwrap();
    let typed: StartApplicationExecutionResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(typed.status, ApplicationExecutionCommandStatus::Unavailable);
    assert!(typed.error.unwrap().reason.contains("disabled for test"));
}

#[tokio::test]
async fn service_runtime_registers_unavailable_provider() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id =
        bootstrap_unavailable_application_execution_service(runtime.clone(), "trace-register")
            .await
            .unwrap();

    assert_eq!(service_id.as_str(), APPLICATION_EXECUTION_SERVICE_ID);

    let trace = TraceContext::new("trace-runtime-start");
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new(APPLICATION_EXECUTION_START_COMMAND),
        serde_json::to_value(StartApplicationExecutionCommand {
            application_id: ApplicationId::from_name("application-execution-runtime-test"),
            session_id: Some("session-1".into()),
            run_id: None,
            task_input: ApplicationExecutionPayload::summary("test"),
            workspace_ref: None,
            requested_capabilities: Vec::new(),
            provider_preference: None,
            trace: trace.clone(),
            policy_context: Default::default(),
            tenant_id: None,
            actor: "runtime-test".into(),
            idempotency_key: "start-idem-1".into(),
        })
        .unwrap(),
        trace,
    );
    let reply = runtime
        .call(
            &service_id,
            ServiceBusSource::new("application-execution-test"),
            command,
        )
        .await
        .unwrap();
    let typed: StartApplicationExecutionResult =
        serde_json::from_value(reply.output.expect("service reply should include output")).unwrap();

    assert_eq!(typed.status, ApplicationExecutionCommandStatus::Unavailable);
}

#[tokio::test]
async fn configured_provider_persists_start_event_and_replays_state() {
    let (_dir, event_log) = event_log();
    let registry = ApplicationExecutionProviderRegistry::new()
        .register(Arc::new(FakeApplicationExecutionProvider::new(
            "provider-hosted",
            ApplicationExecutionProviderKind::MacacaHosted,
        )))
        .unwrap();
    let provider =
        ApplicationExecutionSystemServiceProvider::with_event_log(event_log.clone(), registry);
    let trace = TraceContext::new("trace-configured-application-execution");
    let command = StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("provider-neutral-application"),
        session_id: Some("session-start".into()),
        run_id: Some("run-start".into()),
        task_input: ApplicationExecutionPayload::summary("generic task"),
        workspace_ref: Some("workspace://app/session-start".into()),
        requested_capabilities: Vec::new(),
        provider_preference: None,
        trace: trace.clone(),
        policy_context: Default::default(),
        tenant_id: None,
        actor: "runtime-test".into(),
        idempotency_key: "start-configured-1".into(),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_START_COMMAND),
            serde_json::to_value(command.clone()).unwrap(),
            trace.clone(),
        ))
        .await
        .unwrap();
    let typed: StartApplicationExecutionResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(typed.status, ApplicationExecutionCommandStatus::Accepted);
    assert_eq!(
        typed.provider_kind,
        ApplicationExecutionProviderKind::MacacaHosted
    );

    let replay = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_EXECUTION_REPLAY_COMMAND),
            serde_json::to_value(ApplicationExecutionReplayRequest {
                application_id: command.application_id,
                session_id: "session-start".into(),
                run_id: Some("run-start".into()),
                from_cursor: None,
                page_size: 20,
                event_types: Vec::new(),
                visibility: None,
                trace,
            })
            .unwrap(),
            TraceContext::new("trace-replay-start"),
        ))
        .await
        .unwrap();
    let replay: ApplicationExecutionReplayResult = serde_json::from_value(replay.output).unwrap();

    assert_eq!(replay.events.len(), 1);
    assert!(replay.current_state.is_some());
}

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

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("application-execution.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

struct FakeApplicationExecutionProvider {
    provider_id: String,
    provider_kind: ApplicationExecutionProviderKind,
}

impl FakeApplicationExecutionProvider {
    fn new(provider_id: &str, provider_kind: ApplicationExecutionProviderKind) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_kind,
        }
    }
}

#[async_trait]
impl ApplicationExecutionProvider for FakeApplicationExecutionProvider {
    fn describe(&self) -> ApplicationExecutionProviderDescriptor {
        ApplicationExecutionProviderDescriptor {
            provider_id: self.provider_id.clone(),
            provider_kind: self.provider_kind,
            protocol_version: "application-execution.v1".into(),
            supported_commands: vec![ApplicationExecutionControlKind::Cancel],
            supported_events: vec![ApplicationExecutionEventType::ExecutionCompleted],
            checkpoint_support: false,
            heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
                interval_ms: 1000,
                timeout_ms: 5000,
                required: false,
            },
            control_delivery: "local-test".into(),
            capability_declarations: vec![CapabilityId::new("capability.test")],
            resource_profile: Default::default(),
            transport_kind: "local-test".into(),
            health_state: ApplicationExecutionProviderHealth::Healthy,
        }
    }

    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        Ok(StartApplicationExecutionResult {
            status: ApplicationExecutionCommandStatus::Accepted,
            session_id: command.session_id,
            run_id: command.run_id,
            provider_id: Some(self.provider_id.clone()),
            provider_kind: self.provider_kind,
            event_cursor: None,
            control_ref: None,
            workspace_ref: command.workspace_ref,
            error: None,
        })
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        Ok(ApplicationExecutionControlResult {
            status: ApplicationExecutionCommandStatus::Delivered,
            scope: command.scope,
            provider_id: Some(self.provider_id.clone()),
            provider_kind: self.provider_kind,
            event_cursor: None,
            error: None,
        })
    }

    async fn snapshot(
        &self,
    ) -> Result<Option<macaca_proto::ApplicationExecutionSnapshot>, ServiceError> {
        Ok(None)
    }
}
