use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::{EventLog, RedbStore};
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionPayload, ApplicationExecutionProviderKind,
    ApplicationExecutionReplayRequest, ApplicationExecutionScope, ApplicationExecutionSnapshot,
    ApplicationId, CapabilityId, StartApplicationExecutionCommand, TraceContext,
};
use tempfile::tempdir;
use tokio::sync::Mutex;

use crate::{
    ApplicationExecutionEventStore, ApplicationExecutionProvider,
    HostedApplicationExecutionAdapter, HostedApplicationExecutionOutcome,
    MacacaHostedApplicationExecutionProvider,
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

struct FakeHostedAdapter {
    outcome: Result<HostedApplicationExecutionOutcome, ServiceErrorProxy>,
    controls: Mutex<Vec<ApplicationExecutionControlKind>>,
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
