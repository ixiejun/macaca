use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationAbiError, ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionReplayRequest, ApplicationExecutionScope,
    ApplicationExecutionSnapshot, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationId, ApplicationImport, CapabilityId,
    PackageRuntimeKind, StartApplicationExecutionCommand, TaskGraphOwner, TraceContext,
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
    assert_eq!(command.payload["chat"]["input"], "generic task");
    assert_eq!(command.payload["chat"]["session_id"], "session-wasm-export");
    assert_eq!(command.payload["chat"]["run_id"], "run-wasm-export");
    let trace = command.trace.as_ref().unwrap();
    assert_eq!(trace.session_id.as_deref(), Some("session-wasm-export"));
    assert_eq!(trace.task_id.as_deref(), Some("run-wasm-export"));
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

#[tokio::test]
async fn abi_hosted_declared_host_command_results_become_durable_execution_events() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let provider = MacacaHostedApplicationExecutionProvider::new(
        store,
        Arc::new(ApplicationAbiHostedExecutionAdapter::new(Arc::new(
            CapturingHostRuntime::with_host_command_results(serde_json::json!([
                {
                    "index": 0,
                    "status": "Completed",
                    "output": {},
                    "metadata": {
                        "service_id": "service.git",
                        "reason_code": "import_completed"
                    }
                },
                {
                    "index": 1,
                    "status": "Ok",
                    "output": {
                        "output": {
                            "status": "completed",
                            "task_id": "task-hosted-delegate-1"
                        }
                    },
                    "metadata": {
                        "service_id": "service.application",
                        "reason_code": "import_completed"
                    }
                }
            ])),
        ))),
        vec![CapabilityId::new("capability.application_execution")],
    );
    let start = start_command(
        "session-wasm-host-results",
        "run-wasm-host-results",
        "hosted-wasm-host-results-1",
    );

    provider.start(start.clone()).await.unwrap();

    let replay = ApplicationExecutionEventStore::new(event_log)
        .replay(ApplicationExecutionReplayRequest {
            application_id: start.application_id,
            session_id: "session-wasm-host-results".into(),
            run_id: Some("run-wasm-host-results".into()),
            from_cursor: None,
            page_size: 50,
            event_types: Vec::new(),
            visibility: None,
            trace: TraceContext::new("trace-hosted-host-results-replay"),
        })
        .await
        .unwrap();
    let event_types = replay
        .events
        .iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();

    assert!(event_types.contains(&ApplicationExecutionEventType::ToolCallCompleted));
    assert!(event_types.contains(&ApplicationExecutionEventType::ExecutionCompleted));
}

#[tokio::test]
async fn abi_hosted_terminal_state_fails_when_any_host_command_fails() {
    let (_dir, event_log) = event_log();
    let store = ApplicationExecutionEventStore::new(event_log.clone());
    let provider = MacacaHostedApplicationExecutionProvider::new(
        store,
        Arc::new(ApplicationAbiHostedExecutionAdapter::new(Arc::new(
            CapturingHostRuntime::with_host_command_results(serde_json::json!([
                {
                    "index": 0,
                    "status": "Ok",
                    "output": {
                        "output": {
                            "status": "completed",
                            "task_id": "task-authoritative-1"
                        }
                    },
                    "metadata": {
                        "service_id": "service.application",
                        "reason_code": "import_completed",
                        "graph_owner": TaskGraphOwner::ApplicationExecution.as_str()
                    }
                },
                {
                    "index": 1,
                    "status": "Ok",
                    "output": {
                        "output": {
                            "status": "failed",
                            "task_id": "task-compatibility-1"
                        }
                    },
                    "metadata": {
                        "service_id": "service.task",
                        "reason_code": "compatibility_fallback_failed",
                        "graph_owner": TaskGraphOwner::TaskServiceCompatibility.as_str()
                    }
                },
                {
                    "index": 2,
                    "status": "Pending",
                    "output": {},
                    "metadata": {
                        "service_id": "service.diagnostics",
                        "reason_code": "diagnostic_pending",
                        "graph_owner": TaskGraphOwner::DiagnosticOnly.as_str()
                    }
                }
            ])),
        ))),
        vec![CapabilityId::new("capability.application_execution")],
    );
    let start = start_command(
        "session-wasm-authoritative-results",
        "run-wasm-authoritative-results",
        "hosted-wasm-authoritative-results-1",
    );

    provider.start(start.clone()).await.unwrap();

    let replay = ApplicationExecutionEventStore::new(event_log)
        .replay(ApplicationExecutionReplayRequest {
            application_id: start.application_id,
            session_id: "session-wasm-authoritative-results".into(),
            run_id: Some("run-wasm-authoritative-results".into()),
            from_cursor: None,
            page_size: 50,
            event_types: Vec::new(),
            visibility: None,
            trace: TraceContext::new("trace-hosted-authoritative-results-replay"),
        })
        .await
        .unwrap();
    let event_types = replay
        .events
        .iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();
    let terminal = replay
        .events
        .iter()
        .find(|event| event.event_type == ApplicationExecutionEventType::ExecutionFailed)
        .expect("any host-command failure should emit a failed terminal event");

    assert!(event_types.contains(&ApplicationExecutionEventType::ToolCallCompleted));
    assert!(event_types.contains(&ApplicationExecutionEventType::ExecutionFailed));
    assert!(!event_types.contains(&ApplicationExecutionEventType::ExecutionCompleted));
    let terminal_data = terminal
        .sanitized_payload
        .data
        .as_ref()
        .expect("terminal failure should retain aggregate host-command audit data");
    assert_eq!(terminal_data["failed"], serde_json::json!(1));
    assert_eq!(terminal_data["completed"], serde_json::json!(1));
}

struct FakeHostedAdapter {
    outcome: Result<HostedApplicationExecutionOutcome, ServiceErrorProxy>,
    controls: Mutex<Vec<ApplicationExecutionControlKind>>,
}

#[derive(Default)]
struct CapturingHostRuntime {
    last_command: Mutex<Option<ApplicationHostCommand>>,
    host_command_results: Option<serde_json::Value>,
}

impl CapturingHostRuntime {
    fn with_host_command_results(host_command_results: serde_json::Value) -> Self {
        Self {
            last_command: Mutex::new(None),
            host_command_results: Some(host_command_results),
        }
    }
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
            output: serde_json::json!({
                "captured": true,
                "host_command_results": self.host_command_results.clone().unwrap_or_else(|| serde_json::json!([])),
            }),
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
            signals: Vec::new(),
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
