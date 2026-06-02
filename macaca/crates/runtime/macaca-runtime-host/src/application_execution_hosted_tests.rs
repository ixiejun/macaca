use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationAbiError, ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionReplayRequest, ApplicationExecutionScope,
    ApplicationExecutionSnapshot, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationId, ApplicationImport, CapabilityId,
    PackageRuntimeKind, StartApplicationExecutionCommand, TraceContext,
};
use tempfile::tempdir;
use tokio::sync::Mutex;

use crate::{
    ApplicationAbiHostedExecutionAdapter, ApplicationExecutionEventStore,
    ApplicationExecutionProvider, ApplicationHostRuntime, HostedApplicationExecutionAdapter,
    HostedApplicationExecutionOutcome, MacacaHostedApplicationExecutionProvider,
};

#[tokio::test]
async fn hosted_provider_waits_for_approval_and_completes_after_control() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let adapter = Arc::new(FakeHostedAdapter::waiting_for_approval());
    let provider = MacacaHostedApplicationExecutionProvider::new(
        store,
        adapter.clone(),
        vec![CapabilityId::new("capability.application_execution")],
    );
    let start = start_command("session-hosted", "run-hosted", "hosted-start-1");

    let result = provider.start(start.clone()).await.unwrap();

    assert_eq!(result.status, ApplicationExecutionCommandStatus::Accepted);
    assert_eq!(
        result.provider_kind,
        ApplicationExecutionProviderKind::MacacaHosted
    );

    let scope = ApplicationExecutionScope {
        application_id: start.application_id,
        session_id: "session-hosted".into(),
        run_id: "run-hosted".into(),
        tenant_id: None,
        actor: "tester".into(),
    };
    let control = ApplicationExecutionControlCommand {
        scope: scope.clone(),
        command: ApplicationExecutionControlKind::Approve,
        control_id: "control-approve-1".into(),
        reason_code: "test_approval".into(),
        trace: TraceContext::new("trace-hosted-approve"),
        policy_context: Default::default(),
        payload: Some(ApplicationExecutionPayload::summary("approved")),
        idempotency_key: "hosted-control-1".into(),
    };
    let control_result = provider.control(control).await.unwrap();

    assert_eq!(
        control_result.status,
        ApplicationExecutionCommandStatus::Completed
    );
    assert_eq!(
        adapter.controls.lock().await.as_slice(),
        &[ApplicationExecutionControlKind::Approve]
    );

    let replay = ApplicationExecutionEventStore::new(event_log)
        .replay(ApplicationExecutionReplayRequest {
            application_id: scope.application_id,
            session_id: scope.session_id,
            run_id: Some(scope.run_id),
            from_cursor: None,
            page_size: 50,
            event_types: Vec::new(),
            visibility: None,
            trace: TraceContext::new("trace-hosted-replay"),
        })
        .await
        .unwrap();
    let state = replay.current_state.unwrap();
    assert_eq!(state.lifecycle_state.is_terminal(), true);
}

#[tokio::test]
async fn hosted_provider_returns_structured_unavailable_when_runtime_is_missing() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log);
    let provider = MacacaHostedApplicationExecutionProvider::new(
        store,
        Arc::new(FakeHostedAdapter::unavailable()),
        Vec::new(),
    );

    let result = provider
        .start(start_command(
            "session-hosted-missing",
            "run-hosted-missing",
            "hosted-missing-1",
        ))
        .await
        .unwrap();

    assert_eq!(
        result.status,
        ApplicationExecutionCommandStatus::Unavailable
    );
    assert!(result.error.unwrap().reason.contains("runtime missing"));
}

#[tokio::test]
async fn abi_hosted_start_invokes_generic_wasm_start_export() {
    let host = Arc::new(CapturingHostRuntime::default());
    let adapter = ApplicationAbiHostedExecutionAdapter::new(host.clone());

    let outcome = adapter
        .start(start_command(
            "session-wasm-export",
            "run-wasm-export",
            "hosted-wasm-export-1",
        ))
        .await
        .unwrap();

    assert!(matches!(
        outcome,
        HostedApplicationExecutionOutcome::Running {
            checkpoint_ref: Some(_),
            ..
        }
    ));
    let command = host.last_command.lock().await.clone().unwrap();
    assert_eq!(
        command.import,
        ApplicationImport::Custom("macaca:wasm/invoke".into())
    );
    assert_eq!(
        command.metadata.get("wasm.export").map(String::as_str),
        Some("app:start")
    );
    assert_eq!(
        command
            .metadata
            .get("execution.operation")
            .map(String::as_str),
        Some("start")
    );
}

#[tokio::test]
async fn abi_hosted_start_ack_is_replayable_without_terminal_completion() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let provider = MacacaHostedApplicationExecutionProvider::new(
        store,
        Arc::new(ApplicationAbiHostedExecutionAdapter::new(Arc::new(
            CapturingHostRuntime::default(),
        ))),
        vec![CapabilityId::new("capability.application_execution")],
    );
    let start = start_command(
        "session-wasm-running",
        "run-wasm-running",
        "hosted-wasm-running-1",
    );

    let result = provider.start(start.clone()).await.unwrap();

    assert_eq!(result.status, ApplicationExecutionCommandStatus::Accepted);
    let replay = ApplicationExecutionEventStore::new(event_log)
        .replay(ApplicationExecutionReplayRequest {
            application_id: start.application_id,
            session_id: "session-wasm-running".into(),
            run_id: Some("run-wasm-running".into()),
            from_cursor: None,
            page_size: 50,
            event_types: Vec::new(),
            visibility: None,
            trace: TraceContext::new("trace-hosted-running-replay"),
        })
        .await
        .unwrap();
    let event_types = replay
        .events
        .iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();

    assert!(event_types.contains(&ApplicationExecutionEventType::ExecutionAccepted));
    assert!(event_types.contains(&ApplicationExecutionEventType::ProviderHeartbeat));
    assert!(event_types.contains(&ApplicationExecutionEventType::CheckpointCreated));
    assert!(!event_types.contains(&ApplicationExecutionEventType::ExecutionCompleted));
    assert_eq!(
        replay.current_state.unwrap().lifecycle_state.is_terminal(),
        false
    );
}

struct FakeHostedAdapter {
    outcome: Result<HostedApplicationExecutionOutcome, ServiceErrorProxy>,
    controls: Mutex<Vec<ApplicationExecutionControlKind>>,
}

#[derive(Default)]
struct CapturingHostRuntime {
    last_command: Mutex<Option<ApplicationHostCommand>>,
}

#[async_trait]
impl ApplicationHostRuntime for CapturingHostRuntime {
    fn runtime_kind(&self) -> PackageRuntimeKind {
        PackageRuntimeKind::WasmComponent
    }

    /// Capture the exact ABI command emitted by the hosted execution adapter.
    ///
    /// The fake host deliberately returns only the provider-neutral success
    /// status.  It does not emulate Codex, WASM bytes, service providers, or
    /// business workflow; the test only verifies that hosted execution crosses
    /// the generic WASM export-invoke seam.
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        *self.last_command.lock().await = Some(command.clone());
        Ok(ApplicationHostCommandResult {
            status: ApplicationHostCommandStatus::Ok,
            output: serde_json::json!({ "captured": true }),
            trace: command.trace,
            policy: None,
            metadata: Default::default(),
        })
    }
}

impl FakeHostedAdapter {
    fn waiting_for_approval() -> Self {
        Self {
            outcome: Ok(HostedApplicationExecutionOutcome::WaitingForApproval {
                approval_ref: "approval-1".into(),
                checkpoint_ref: Some("checkpoint-before-approval".into()),
                summary: "waiting for generic approval".into(),
            }),
            controls: Mutex::new(Vec::new()),
        }
    }

    fn unavailable() -> Self {
        Self {
            outcome: Err(ServiceErrorProxy("runtime missing".into())),
            controls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl HostedApplicationExecutionAdapter for FakeHostedAdapter {
    async fn start(
        &self,
        _command: StartApplicationExecutionCommand,
    ) -> Result<HostedApplicationExecutionOutcome, macaca_proto::ServiceError> {
        self.outcome
            .clone()
            .map_err(|error| macaca_proto::ServiceError::ServiceUnavailable(error.0))
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, macaca_proto::ServiceError> {
        self.controls.lock().await.push(command.command);
        Ok(ApplicationExecutionCommandStatus::Completed)
    }

    async fn resume(
        &self,
        _snapshot: ApplicationExecutionSnapshot,
    ) -> Result<HostedApplicationExecutionOutcome, macaca_proto::ServiceError> {
        Ok(HostedApplicationExecutionOutcome::Running {
            checkpoint_ref: None,
            summary: "resumed".into(),
        })
    }
}

#[derive(Debug, Clone)]
struct ServiceErrorProxy(String);

fn event_log() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempdir().unwrap();
    let redb = Arc::new(RedbStore::open(dir.path().join("hosted.redb")).unwrap());
    (dir, Arc::new(EventLog::new(redb)))
}

fn start_command(
    session_id: &str,
    run_id: &str,
    idempotency_key: &str,
) -> StartApplicationExecutionCommand {
    StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("hosted-neutral-application"),
        session_id: Some(session_id.into()),
        run_id: Some(run_id.into()),
        task_input: ApplicationExecutionPayload::summary("generic task"),
        workspace_ref: Some("workspace://session".into()),
        requested_capabilities: vec![CapabilityId::new("capability.application_execution")],
        provider_preference: None,
        trace: TraceContext::new(format!("trace-{idempotency_key}")),
        policy_context: Default::default(),
        tenant_id: None,
        actor: "tester".into(),
        idempotency_key: idempotency_key.into(),
    }
}
