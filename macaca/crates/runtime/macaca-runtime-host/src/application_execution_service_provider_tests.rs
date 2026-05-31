use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionControlResult,
    ApplicationExecutionEventType, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionPayload, ApplicationExecutionProviderDescriptor,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionReplayRequest, ApplicationExecutionReplayResult, ApplicationId,
    CapabilityId, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError,
    StartApplicationExecutionCommand, StartApplicationExecutionResult, TraceContext,
    APPLICATION_EXECUTION_REPLAY_COMMAND, APPLICATION_EXECUTION_SERVICE_ID,
    APPLICATION_EXECUTION_START_COMMAND,
};
use tempfile::tempdir;

use crate::{
    bootstrap_application_execution_service, bootstrap_unavailable_application_execution_service,
    ApplicationExecutionProvider, ApplicationExecutionProviderRegistry,
    ApplicationExecutionSystemServiceProvider, DenyAllServiceRuntimePolicy, ServiceRuntime,
    ServiceRuntimeConfig, ServiceRuntimeError,
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
async fn service_runtime_rejects_missing_trace_before_application_execution_dispatch() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id =
        bootstrap_unavailable_application_execution_service(runtime.clone(), "trace-register")
            .await
            .unwrap();

    let err = runtime
        .call(
            &service_id,
            ServiceBusSource::new("application-execution-test"),
            ServiceCommand::without_trace(
                ServiceCommandName::new(APPLICATION_EXECUTION_START_COMMAND),
                serde_json::json!({}),
            ),
        )
        .await
        .unwrap_err();

    assert_eq!(err, ServiceRuntimeError::MissingTraceContext);
}

#[tokio::test]
async fn service_runtime_policy_denial_stops_application_execution_side_effects() {
    let (_dir, event_log) = event_log();
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        policy: Arc::new(DenyAllServiceRuntimePolicy::new(
            "application execution blocked by test policy",
        )),
        ..Default::default()
    }));
    let registry = ApplicationExecutionProviderRegistry::new()
        .register(Arc::new(FakeApplicationExecutionProvider::new(
            "provider-policy-test",
            ApplicationExecutionProviderKind::MacacaHosted,
        )))
        .unwrap();
    let service_id = bootstrap_application_execution_service(
        runtime.clone(),
        event_log.clone(),
        registry,
        "trace-policy-bootstrap",
    )
    .await
    .unwrap();
    let trace = TraceContext::new("trace-application-execution-policy");
    let command = StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application-execution-policy-test"),
        session_id: Some("session-policy".into()),
        run_id: Some("run-policy".into()),
        task_input: ApplicationExecutionPayload::summary("test"),
        workspace_ref: None,
        requested_capabilities: Vec::new(),
        provider_preference: None,
        trace: trace.clone(),
        policy_context: Default::default(),
        tenant_id: None,
        actor: "runtime-test".into(),
        idempotency_key: "start-policy-1".into(),
    };

    let err = runtime
        .call(
            &service_id,
            ServiceBusSource::new("application-execution-test"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(APPLICATION_EXECUTION_START_COMMAND),
                serde_json::to_value(command).unwrap(),
                trace,
            ),
        )
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ServiceRuntimeError::PolicyDenied("application execution blocked by test policy".into())
    );
    assert_eq!(
        event_log.latest_seq("session-policy").await,
        0,
        "policy denial must happen before provider assignment or EventLog append"
    );
}

#[tokio::test]
async fn unknown_application_execution_command_returns_structured_unsupported() {
    let provider = ApplicationExecutionSystemServiceProvider::unavailable("disabled for test");
    let err = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("application_execution.unknown"),
            serde_json::json!({}),
            TraceContext::new("trace-application-execution-unknown"),
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(err, ServiceError::UnsupportedCommand(command) if command == "application_execution.unknown")
    );
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
async fn duplicate_control_command_reuses_cursor_without_duplicate_provider_delivery() {
    let (_dir, event_log) = event_log();
    let fake_provider = Arc::new(FakeApplicationExecutionProvider::new(
        "provider-control-idempotent",
        ApplicationExecutionProviderKind::MacacaHosted,
    ));
    let registry = ApplicationExecutionProviderRegistry::new()
        .register(fake_provider.clone())
        .unwrap();
    let provider =
        ApplicationExecutionSystemServiceProvider::with_event_log(event_log.clone(), registry);
    let trace = TraceContext::new("trace-control-idempotent");
    let command = ApplicationExecutionControlCommand {
        control_id: "control-1".into(),
        command: ApplicationExecutionControlKind::Cancel,
        scope: macaca_proto::ApplicationExecutionScope::new(
            ApplicationId::from_name("control-neutral-application"),
            "session-control",
            "run-control",
            "runtime-test",
        )
        .unwrap(),
        payload: None,
        idempotency_key: "control-idempotency-1".into(),
        reason_code: "control.idempotency.test".into(),
        trace: trace.clone(),
        policy_context: Default::default(),
    };

    let first = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(macaca_proto::APPLICATION_EXECUTION_CONTROL_COMMAND),
            serde_json::to_value(command.clone()).unwrap(),
            trace.clone(),
        ))
        .await
        .unwrap();
    let second = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(macaca_proto::APPLICATION_EXECUTION_CONTROL_COMMAND),
            serde_json::to_value(command).unwrap(),
            trace,
        ))
        .await
        .unwrap();
    let first: ApplicationExecutionControlResult = serde_json::from_value(first.output).unwrap();
    let second: ApplicationExecutionControlResult = serde_json::from_value(second.output).unwrap();

    assert_eq!(first.event_cursor, second.event_cursor);
    assert_eq!(first.status, ApplicationExecutionCommandStatus::Delivered);
    assert_eq!(second.status, ApplicationExecutionCommandStatus::Duplicate);
    assert_eq!(
        fake_provider.control_calls(),
        1,
        "duplicate control commands must not be delivered twice to the selected provider"
    );
    let replay = event_log.query("session-control", 0, 10).await;
    assert_eq!(
        replay.len(),
        1,
        "duplicate control commands must not create duplicate durable EventLog rows"
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
    control_calls: Arc<AtomicUsize>,
}

impl FakeApplicationExecutionProvider {
    fn new(provider_id: &str, provider_kind: ApplicationExecutionProviderKind) -> Self {
        Self {
            provider_id: provider_id.into(),
            provider_kind,
            control_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn control_calls(&self) -> usize {
        self.control_calls.load(Ordering::SeqCst)
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
        self.control_calls.fetch_add(1, Ordering::SeqCst);
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
