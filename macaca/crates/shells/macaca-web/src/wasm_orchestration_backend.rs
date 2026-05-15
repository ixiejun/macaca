//! Web composition adapter for WASM application orchestration.
//!
//! The Application Service owns the provider-neutral
//! `application.agent.delegate` command.  Web owns the concrete
//! `ApplicationExecutorRegistry` that is already scoped per application and is
//! therefore the right composition root for connecting that command to worker
//! execution.  This file keeps the adapter small and explicit so runtime-host
//! does not import Web state and Web does not define new orchestration
//! semantics.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::ApplicationExecutorRegistry;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, ApplicationAgentDelegateCommand,
    ApplicationAgentDelegateResult, KernelServiceId, ServiceBusSource, ServiceError, ServiceResult,
    TaskId, AGENT_EXECUTION_SERVICE_ID,
};
use macaca_runtime_host::{ApplicationOrchestrationBackend, ServiceRuntime};
use tokio::sync::{oneshot, RwLock};
use tokio::time::{timeout, Duration};

const DEFAULT_AGENT_DELEGATE_WAIT_MS: u64 = 30_000;

/// Web-owned adapter from Application Service delegation commands to the
/// app-scoped executor registry.
pub(crate) struct WebApplicationOrchestrationBackend {
    executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
    service_runtime: Arc<ServiceRuntime>,
}

impl WebApplicationOrchestrationBackend {
    /// Create the adapter with the shared application executor registry.
    pub(crate) fn new(
        executor_registry: Arc<RwLock<Option<Arc<ApplicationExecutorRegistry>>>>,
        service_runtime: Arc<ServiceRuntime>,
    ) -> Self {
        Self {
            executor_registry,
            service_runtime,
        }
    }
}

#[async_trait]
impl ApplicationOrchestrationBackend for WebApplicationOrchestrationBackend {
    async fn delegate_agent(
        &self,
        command: ApplicationAgentDelegateCommand,
    ) -> ServiceResult<ApplicationAgentDelegateResult> {
        let app_id = command.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "application.agent.delegate requires application_id".into(),
            )
        })?;
        let session_id = command
            .scope
            .session_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                ServiceError::AdapterFailure(
                    "application.agent.delegate requires session_id".into(),
                )
            })?;
        let registry = self.executor_registry.read().await.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                "application executor registry is not configured".into(),
            )
        })?;
        if registry.get(&app_id).await.is_none() {
            return Err(ServiceError::ServiceUnavailable(format!(
                "application executor is not configured for application {app_id}"
            )));
        }
        let task_id = TaskId::new();
        let mut execution_command = AgentExecutionCommand::new(
            app_id,
            session_id.clone(),
            command.target_agent.clone(),
            AgentExecutionIntent::WasmDelegate,
            command.prompt.clone(),
            command.trace.clone(),
        )
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?
        .with_delegated_context(command.context.clone());
        execution_command.task_id = Some(task_id);
        execution_command.source_agent = command.scope.agent_name.clone();
        execution_command.metadata = command.metadata.clone();

        let service_runtime = Arc::clone(&self.service_runtime);
        let service_command = execution_command
            .into_service_command()
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tokio::spawn(async move {
            let reply = service_runtime
                .call(
                    &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                    ServiceBusSource::new("macaca.web.wasm_orchestration"),
                    service_command,
                )
                .await;
            let _ = reply_tx.send(reply);
        });
        let wait_ms = command
            .metadata
            .get("wait_timeout_ms")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AGENT_DELEGATE_WAIT_MS);
        let reply = match timeout(Duration::from_millis(wait_ms), reply_rx).await {
            Ok(Ok(Ok(reply))) => reply,
            Ok(Ok(Err(error))) => {
                return Err(ServiceError::AdapterFailure(error.to_string()));
            }
            Ok(Err(_closed)) => {
                return Err(ServiceError::AdapterFailure(
                    "agent execution service reply channel closed".into(),
                ));
            }
            Err(_elapsed) => {
                return Ok(ApplicationAgentDelegateResult {
                    application_id: app_id,
                    session_id,
                    target_agent: command.target_agent,
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
                });
            }
        };
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("agent execution service returned no output".into())
        })?;
        let result: macaca_proto::AgentExecutionResult = serde_json::from_value(output)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        Ok(ApplicationAgentDelegateResult {
            application_id: app_id,
            session_id,
            target_agent: command.target_agent,
            task_id: result.task_id.map(|id| id.0.to_string()),
            success: matches!(result.status, macaca_proto::AgentExecutionStatus::Completed),
            output: result.output,
            status: result.status.as_str().into(),
            metadata: result.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wasm_delegate_uses_agent_execution_service_not_executor_fast_path() {
        let source = include_str!("wasm_orchestration_backend.rs");
        let executor_fast_path = [".delegate", "_task("].concat();

        assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(source.contains("AgentExecutionCommand::new"));
        assert!(source.contains("ServiceBusSource::new(\"macaca.web.wasm_orchestration\")"));
        assert!(!source.contains(&executor_fast_path));
    }
}
