use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionCurrentState,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationId, MacacaResult,
    StartApplicationExecutionCommand, TraceContext, APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
    APPLICATION_EXECUTION_SERVICE_ID,
};
use serde_json::Value;

use crate::{
    ServiceBackedApplicationExecutionClient, ServiceCallCommand, ServiceCallResult,
    ServiceInspectionCommand, ServiceInspectionResult, SystemApplicationExecutionClient,
    SystemApplicationExecutionFacadeExt, SystemFacade, SystemServiceClient, SystemTaskClient,
    TaskBoardQueryCommand, TaskBoardQueryResult, UnavailableSystemApplicationExecutionClient,
};

#[tokio::test]
async fn unavailable_client_returns_structured_unavailable_start() {
    let client = UnavailableSystemApplicationExecutionClient::new();
    let command = StartApplicationExecutionCommand {
        application_id: ApplicationId::from_name("application-execution-sdk-test"),
        session_id: Some("session-1".into()),
        run_id: None,
        task_input: ApplicationExecutionPayload::summary("run test"),
        workspace_ref: None,
        requested_capabilities: Vec::new(),
        provider_preference: None,
        trace: TraceContext::new("trace-application-execution-sdk-start"),
        policy_context: Default::default(),
        tenant_id: None,
        actor: "sdk-test".into(),
        idempotency_key: "start-idem-1".into(),
    };

    let result = client.start_execution(command).await.unwrap();

    assert_eq!(
        result.status,
        ApplicationExecutionCommandStatus::Unavailable
    );
    assert_eq!(
        result.provider_kind,
        ApplicationExecutionProviderKind::Unavailable
    );
    assert_eq!(
        result.error.unwrap().code,
        ApplicationExecutionCommandStatus::Unavailable
    );
}

#[tokio::test]
async fn unavailable_client_preserves_control_scope() {
    let client = UnavailableSystemApplicationExecutionClient::new();
    let scope = ApplicationExecutionScope::new(
        ApplicationId::from_name("application-execution-sdk-test"),
        "session-1",
        "run-1",
        "operator",
    )
    .unwrap();
    let command = ApplicationExecutionControlCommand {
        scope: scope.clone(),
        command: ApplicationExecutionControlKind::Cancel,
        control_id: "control-1".into(),
        reason_code: "test_cancel".into(),
        trace: TraceContext::new("trace-application-execution-sdk-control"),
        policy_context: Default::default(),
        payload: None,
        idempotency_key: "control-idem-1".into(),
    };

    let result = client.send_control(command).await.unwrap();

    assert_eq!(result.scope, scope);
    assert_eq!(
        result.status,
        ApplicationExecutionCommandStatus::Unavailable
    );
}

#[test]
fn descriptor_names_the_generic_service() {
    let client = UnavailableSystemApplicationExecutionClient::new();
    assert_eq!(
        client.descriptor().id.as_str(),
        APPLICATION_EXECUTION_SERVICE_ID
    );
}

#[tokio::test]
async fn service_backed_current_state_forwards_complete_scope() {
    let captured = Arc::new(Mutex::new(None));
    let client = ServiceBackedApplicationExecutionClient::new(Arc::new(CapturingServiceClient {
        captured: Arc::clone(&captured),
    }));
    let scope = ApplicationExecutionScope::new(
        ApplicationId::from_name("application-execution-sdk-scope-test"),
        "session-scope",
        "run-scope",
        "operator",
    )
    .unwrap();

    let state = client
        .query_current_state(
            TraceContext::new("trace-application-execution-current-state"),
            scope.clone(),
        )
        .await
        .unwrap();

    assert_eq!(state.scope, scope);
    let captured = captured.lock().unwrap().clone().unwrap();
    assert_eq!(
        captured["command_name"],
        APPLICATION_EXECUTION_CURRENT_STATE_COMMAND
    );
    assert_eq!(captured["payload"]["session_id"], "session-scope");
    assert_eq!(captured["payload"]["run_id"], "run-scope");
    assert_eq!(captured["payload"]["actor"], "operator");
}

#[test]
fn system_facade_extension_builds_application_execution_clients() {
    let facade = SystemFacade::new(
        EmptyTaskBoardClient,
        crate::StaticSystemStatusDataSource::new(crate::SystemStatusSnapshot {
            version: "test".into(),
            agent_count: 0,
            loaded_apps: 0,
            max_agents: 8,
            llm_provider: "stub".into(),
            app_runtime: "macaca-app/AppRuntime".into(),
            gateway_enabled: false,
        }),
    );

    let unavailable = facade.unavailable_application_execution_client();
    assert_eq!(
        unavailable.descriptor().id.as_str(),
        APPLICATION_EXECUTION_SERVICE_ID
    );
}

struct CapturingServiceClient {
    captured: Arc<Mutex<Option<Value>>>,
}

struct EmptyTaskBoardClient;

#[async_trait]
impl SystemTaskClient for EmptyTaskBoardClient {
    async fn query_task_board(
        &self,
        _command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        Ok(TaskBoardQueryResult {
            todos: Vec::new(),
            count: 0,
        })
    }
}

#[async_trait]
impl SystemServiceClient for CapturingServiceClient {
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        Ok(ServiceInspectionResult {
            scope: command.scope.clone(),
            services: vec![APPLICATION_EXECUTION_SERVICE_ID.into()],
        })
    }

    async fn call_service(&self, command: &ServiceCallCommand) -> MacacaResult<ServiceCallResult> {
        self.captured.lock().unwrap().replace(serde_json::json!({
            "service_id": command.service_id,
            "command_name": command.command_name,
            "payload": command.payload,
            "trace_id": command.trace.as_ref().map(|trace| trace.trace_id.clone()),
        }));
        let scope: ApplicationExecutionScope = serde_json::from_value(command.payload.clone())?;
        Ok(ServiceCallResult {
            service_id: command.service_id.clone(),
            output: serde_json::to_value(ApplicationExecutionCurrentState {
                scope,
                lifecycle_state: ApplicationExecutionLifecycleState::Running,
                provider_id: None,
                provider_kind: ApplicationExecutionProviderKind::Unavailable,
                latest_heartbeat_at: None,
                pending_approval_refs: Vec::new(),
                active_control_ids: Vec::new(),
                latest_checkpoint_ref: None,
                step_summaries: Vec::new(),
                terminal_result: None,
                terminal_error: None,
                replay_cursor: None,
            })?,
        })
    }
}
