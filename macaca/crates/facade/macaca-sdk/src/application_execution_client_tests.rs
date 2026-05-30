use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionPayload, ApplicationExecutionProviderKind,
    ApplicationExecutionScope, ApplicationId, StartApplicationExecutionCommand, TraceContext,
    APPLICATION_EXECUTION_SERVICE_ID,
};

use crate::{SystemApplicationExecutionClient, UnavailableSystemApplicationExecutionClient};

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
