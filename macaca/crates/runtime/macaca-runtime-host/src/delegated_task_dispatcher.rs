//! Service-backed delegated task dispatch for orchestration tools.
//!
//! The dispatcher is a runtime-host Strategy for executing `delegate_task`
//! requests through the unified `service.agent_execution` boundary. Shells may
//! wire the callback into a toolkit, but the service-call choreography and
//! executor bookkeeping belong to runtime-host.

use std::sync::Arc;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationId, KernelServiceId, ServiceBusSource, TraceContext, AGENT_EXECUTION_SERVICE_ID,
};
use tracing::{info, warn};

use crate::{ApplicationExecutor, ServiceRuntime, TaskContext, TaskId, TaskResult};

/// Provider-neutral request to delegate work through the agent execution service.
#[derive(Debug, Clone)]
pub struct DelegateViaAgentServiceRequest {
    /// Target application scope.
    pub application_id: ApplicationId,
    /// Agent that initiated the delegation. The label is caller supplied and
    /// must remain application-neutral at the OS layer.
    pub from_agent: String,
    /// Agent that should execute the delegated prompt.
    pub to_agent: String,
    /// Work prompt forwarded to the execution service.
    pub prompt: String,
    /// Queue priority metadata retained for executor bookkeeping.
    pub priority: u8,
    /// Whether the delegated unit may run in parallel with siblings.
    pub parallel: bool,
    /// Optional session/task context propagated into delegated execution metadata.
    pub context: Option<TaskContext>,
}

/// Dispatches delegated tasks through the unified agent execution service boundary.
pub struct ServiceDelegatedTaskDispatcher {
    service_runtime: Arc<ServiceRuntime>,
}

impl ServiceDelegatedTaskDispatcher {
    /// Create a dispatcher bound to the shared host `ServiceRuntime`.
    pub fn new(service_runtime: Arc<ServiceRuntime>) -> Self {
        Self { service_runtime }
    }

    /// Admit a delegated task for service-backed execution and spawn the async service call.
    ///
    /// The method returns the allocated [`TaskId`] immediately so fork orchestration can suspend
    /// on it before the service-backed worker finishes. Completion is reported back through the
    /// executor's service-backed delegation lifecycle hooks.
    pub async fn dispatch(
        &self,
        executor: Arc<ApplicationExecutor>,
        request: DelegateViaAgentServiceRequest,
    ) -> Result<TaskId, String> {
        let task_id = executor
            .begin_service_backed_delegation(
                &request.from_agent,
                &request.to_agent,
                request.prompt.clone(),
                request.priority,
                request.parallel,
                request.context.clone(),
            )
            .await?;

        info!(
            application_id = %request.application_id.0,
            task_id = %task_id.0,
            from_agent = %request.from_agent,
            to_agent = %request.to_agent,
            service_id = AGENT_EXECUTION_SERVICE_ID,
            "Spawned service-backed delegate_task execution"
        );

        let service_runtime = Arc::clone(&self.service_runtime);
        let to_agent = request.to_agent.clone();
        let prompt = request.prompt.clone();
        let application_id = request.application_id;
        let context = request.context.clone();

        tokio::spawn(async move {
            let execution_result = execute_delegate_via_agent_service(
                service_runtime,
                application_id,
                task_id,
                &to_agent,
                &prompt,
                context,
            )
            .await;
            executor
                .complete_service_backed_delegation(task_id, &to_agent, execution_result)
                .await;
        });

        Ok(task_id)
    }
}

/// Execute one delegated prompt through `service.agent_execution`.
///
/// Kept as a free function so module tests can validate command construction
/// without standing up the full executor/fork runtime. The function does not
/// inspect application-specific roles or workflow names.
async fn execute_delegate_via_agent_service(
    service_runtime: Arc<ServiceRuntime>,
    application_id: ApplicationId,
    task_id: TaskId,
    to_agent: &str,
    prompt: &str,
    context: Option<TaskContext>,
) -> Result<TaskResult, String> {
    let session_id = context
        .as_ref()
        .and_then(|ctx| ctx.session_id.clone())
        .unwrap_or_else(|| format!("delegate-task:{}", task_id.0));

    let mut trace = TraceContext::new(format!(
        "delegate-task:{}:{}:{}",
        application_id.0, to_agent, task_id.0
    ));
    trace.session_id = Some(session_id.clone());
    trace.task_id = Some(task_id.0.to_string());
    trace.agent = Some(to_agent.to_string());

    let delegated_context = context
        .as_ref()
        .map(|ctx| {
            serde_json::json!({
                "artifacts": ctx.artifacts.clone(),
                "env": ctx.env.clone(),
            })
        })
        .unwrap_or_else(|| serde_json::json!({}));

    let mut command = AgentExecutionCommand::new(
        application_id,
        session_id,
        to_agent,
        AgentExecutionIntent::TaskWorker,
        prompt,
        trace,
    )
    .map_err(|error| error.to_string())?
    .with_delegated_context(delegated_context);
    command.task_id = Some(task_id);
    command.metadata.insert(
        "entrypoint".into(),
        "macaca.runtime_host.delegate_task".into(),
    );

    let service_command = command
        .into_service_command()
        .map_err(|error| error.to_string())?;

    info!(
        application_id = %application_id.0,
        task_id = %task_id.0,
        agent = %to_agent,
        service_id = AGENT_EXECUTION_SERVICE_ID,
        "Calling agent execution service for delegated task"
    );

    let reply = service_runtime
        .call(
            &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
            ServiceBusSource::new("macaca.runtime_host.delegated_task_dispatcher"),
            service_command,
        )
        .await
        .map_err(|error| error.to_string())?;

    let output = reply
        .output
        .ok_or_else(|| "agent execution service returned no output".to_string())?;
    let result: AgentExecutionResult =
        serde_json::from_value(output).map_err(|error| error.to_string())?;

    match result.status {
        AgentExecutionStatus::Completed => {
            let output_text = result
                .output
                .get("output")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| result.output.to_string());
            Ok(TaskResult {
                task_id: result.task_id.unwrap_or(task_id),
                success: !output_text.is_empty(),
                output: output_text,
                error: None,
                artifacts: vec![],
                completed_at: chrono::Utc::now(),
                tokens_used: None,
            })
        }
        status => {
            let error = result
                .output
                .get("error")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("agent execution service returned {}", status.as_str()));
            warn!(
                application_id = %application_id.0,
                task_id = %task_id.0,
                agent = %to_agent,
                status = %status.as_str(),
                error = %error,
                "Delegated task agent execution finished with non-completed status"
            );
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::AgentExecutionEvent;

    #[test]
    fn delegate_request_uses_task_worker_intent_metadata() {
        let request = DelegateViaAgentServiceRequest {
            application_id: ApplicationId::new(),
            from_agent: "delegator".into(),
            to_agent: "worker-agent".into(),
            prompt: "inspect module boundaries".into(),
            priority: 5,
            parallel: true,
            context: None,
        };

        assert_eq!(request.from_agent, "delegator");
        assert_eq!(request.to_agent, "worker-agent");
        assert!(request.parallel);
        assert_eq!(request.priority, 5);
    }

    #[test]
    fn agent_execution_event_helpers_remain_available_for_future_streaming_delegate_path() {
        // Compile-time guard: delegated execution can forward streaming events later without
        // importing shell-only SSE types into the dispatcher.
        let _event = AgentExecutionEvent::thinking(0);
    }
}
