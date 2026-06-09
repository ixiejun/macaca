//! Shared bridge from `application.agent.delegate` to `service.agent_execution`.
//!
//! Both WASM host imports and YAML workflow steps enter through the Application
//! Service command `application.agent.delegate`.  This module centralizes the
//! second hop so audit replay sees one orchestration adapter regardless of
//! runtime package kind.  The Bridge pattern keeps `WebApplicationOrchestrationBackend`
//! small while preserving provider-neutral intent selection via metadata.

use std::sync::Arc;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult, ApplicationId,
    KernelServiceId, ServiceBusSource, ServiceError, ServiceReply, ServiceResult, TaskId,
    AGENT_EXECUTION_SERVICE_ID,
};
use macaca_sdk::runtime_host::ServiceRuntime;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use tracing::info;

/// Default wait budget for WASM-style delegation that may return `queued` quickly.
pub(crate) const DEFAULT_AGENT_DELEGATE_WAIT_MS: u64 = 30_000;

/// Service-bus source recorded when the orchestration adapter forwards work.
pub(crate) const ORCHESTRATION_AGENT_EXECUTION_SOURCE: &str =
    "macaca.web.application_agent_delegate_bridge";

/// Build the typed execution command from one Application Service delegation.
///
/// Intent selection uses the Strategy pattern: delegate metadata carries a
/// provider-neutral `execution_intent` label that maps to `AgentExecutionIntent`
/// without referencing any application name, workflow id, or package kind.
pub(crate) fn build_execution_command_from_delegate(
    command: &ApplicationAgentDelegateCommand,
    app_id: ApplicationId,
    session_id: &str,
    delegated_context: serde_json::Value,
    task_id: TaskId,
) -> ServiceResult<AgentExecutionCommand> {
    let execution_intent = AgentExecutionIntent::from_delegate_metadata(&command.metadata);
    info!(
        trace_id = %command.trace.trace_id,
        application_id = %app_id,
        session_id,
        target_agent = %command.target_agent,
        execution_intent = execution_intent.metadata_value(),
        task_id = %task_id.0,
        "application agent delegation resolved execution intent"
    );

    let mut execution_command = AgentExecutionCommand::new(
        app_id,
        session_id.to_string(),
        command.target_agent.clone(),
        execution_intent,
        command.prompt.clone(),
        command.trace.clone(),
    )
    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?
    .with_delegated_context(delegated_context);
    execution_command.task_id = Some(task_id);
    execution_command.source_agent = command.scope.agent_name.clone();
    execution_command.metadata = command.metadata.clone();
    Ok(execution_command)
}

/// Forward one execution command through `service.agent_execution`.
///
/// YAML workflow steps require a blocking wait because the kernel executor
/// thread is waiting on `AgentRunner` completion.  WASM delegation keeps the
/// prior spawn+timeout behavior so long-running guest work can return `queued`
/// without wedging the Application Service provider thread.
pub(crate) async fn dispatch_agent_execution_via_service(
    service_runtime: Arc<ServiceRuntime>,
    execution_command: AgentExecutionCommand,
    wait_ms: u64,
    require_completed: bool,
) -> ServiceResult<ServiceReply> {
    let execution_intent = execution_command.execution_intent.clone();
    let service_command = execution_command
        .into_service_command()
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;

    if require_completed {
        info!(
            service_id = AGENT_EXECUTION_SERVICE_ID,
            execution_intent = ?execution_intent,
            "application agent delegation dispatching synchronous agent execution"
        );
        return service_runtime
            .call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new(ORCHESTRATION_AGENT_EXECUTION_SOURCE),
                service_command,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()));
    }

    let (reply_tx, reply_rx) = oneshot::channel();
    tokio::spawn(async move {
        let reply = service_runtime
            .call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new(ORCHESTRATION_AGENT_EXECUTION_SOURCE),
                service_command,
            )
            .await;
        let _ = reply_tx.send(reply);
    });

    match timeout(Duration::from_millis(wait_ms), reply_rx).await {
        Ok(Ok(Ok(reply))) => Ok(reply),
        Ok(Ok(Err(error))) => Err(ServiceError::AdapterFailure(error.to_string())),
        Ok(Err(_closed)) => Err(ServiceError::AdapterFailure(
            "agent execution service reply channel closed".into(),
        )),
        Err(_elapsed) => Err(ServiceError::AdapterFailure(format!(
            "agent execution service timed out after {wait_ms}ms"
        ))),
    }
}

/// Convert one agent execution service reply into an Application delegate result.
pub(crate) fn delegate_result_from_execution_reply(
    command: &ApplicationAgentDelegateCommand,
    app_id: ApplicationId,
    session_id: String,
    task_id: TaskId,
    reply: ServiceReply,
) -> ServiceResult<ApplicationAgentDelegateResult> {
    let output = reply.output.ok_or_else(|| {
        ServiceError::AdapterFailure("agent execution service returned no output".into())
    })?;
    let result: AgentExecutionResult =
        serde_json::from_value(output).map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
    Ok(ApplicationAgentDelegateResult {
        application_id: app_id,
        session_id,
        target_agent: command.target_agent.clone(),
        task_id: result.task_id.map(|id| id.0.to_string()).or_else(|| Some(task_id.0.to_string())),
        success: matches!(result.status, AgentExecutionStatus::Completed),
        output: result.output,
        status: result.status.as_str().into(),
        metadata: result.metadata,
    })
}

/// Build a queued delegate result when WASM-style async delegation times out.
pub(crate) fn queued_delegate_result(
    command: &ApplicationAgentDelegateCommand,
    app_id: ApplicationId,
    session_id: String,
    task_id: TaskId,
) -> ApplicationAgentDelegateResult {
    ApplicationAgentDelegateResult {
        application_id: app_id,
        session_id,
        target_agent: command.target_agent.clone(),
        task_id: Some(task_id.0.to_string()),
        success: true,
        output: serde_json::json!({
            "status": "queued",
            "task_id": task_id.0.to_string()
        }),
        status: "queued".into(),
        metadata: std::collections::BTreeMap::from([(
            "reason_code".into(),
            "delegate_queued".into(),
        )]),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use macaca_proto::{ApplicationServiceScope, TraceContext};

    use super::*;

    #[test]
    fn delegate_metadata_selects_yaml_workflow_intent() {
        let command = ApplicationAgentDelegateCommand {
            trace: TraceContext::new("test-yaml-intent"),
            scope: ApplicationServiceScope::default(),
            target_agent: "worker".into(),
            prompt: "step".into(),
            context: serde_json::json!({}),
            metadata: BTreeMap::from([(
                macaca_proto::AGENT_EXECUTION_INTENT_METADATA_KEY.into(),
                AgentExecutionIntent::YamlWorkflowStep.metadata_value().into(),
            )]),
        };

        let built = build_execution_command_from_delegate(
            &command,
            ApplicationId::new(),
            "session-1",
            serde_json::json!({}),
            TaskId::new(),
        )
        .expect("yaml intent should build");

        assert_eq!(
            built.execution_intent,
            AgentExecutionIntent::YamlWorkflowStep
        );
    }

    #[test]
    fn missing_delegate_metadata_defaults_to_wasm_delegate_intent() {
        let command = ApplicationAgentDelegateCommand {
            trace: TraceContext::new("test-wasm-default"),
            scope: ApplicationServiceScope::default(),
            target_agent: "worker".into(),
            prompt: "step".into(),
            context: serde_json::json!({}),
            metadata: BTreeMap::new(),
        };

        let built = build_execution_command_from_delegate(
            &command,
            ApplicationId::new(),
            "session-1",
            serde_json::json!({}),
            TaskId::new(),
        )
        .expect("default wasm intent should build");

        assert_eq!(built.execution_intent, AgentExecutionIntent::WasmDelegate);
    }
}
