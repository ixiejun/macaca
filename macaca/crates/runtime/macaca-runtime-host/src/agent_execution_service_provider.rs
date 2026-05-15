//! Agent Execution system service provider.
//!
//! This provider is the single ServiceRuntime entrypoint for starting
//! context-aware agent work. Concrete execution is supplied by host-specific
//! backends so runtime-host owns the service boundary without importing Web
//! state or application-specific logic.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, CleanupPolicy, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope,
    ServiceType, TraceContext, TraceSchemaRef, AGENT_EXECUTE_COMMAND, AGENT_EXECUTION_SERVICE_ID,
};

/// Replaceable backend that executes one already-admitted agent command.
#[async_trait]
pub trait AgentExecutionBackend: Send + Sync {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult>;
}

/// ServiceRuntime-facing provider for `service.agent_execution`.
pub struct AgentExecutionSystemServiceProvider {
    descriptor: ServiceDescriptor,
    backend: Option<Arc<dyn AgentExecutionBackend>>,
}

impl AgentExecutionSystemServiceProvider {
    /// Create a provider with a concrete execution backend.
    pub fn new(backend: Arc<dyn AgentExecutionBackend>) -> Self {
        Self {
            descriptor: agent_execution_service_descriptor(),
            backend: Some(backend),
        }
    }

    /// Create an explicit unavailable provider for hosts without agent runtime
    /// execution support.
    pub fn unavailable() -> Self {
        let mut descriptor = agent_execution_service_descriptor();
        descriptor.health = ServiceHealth::Unavailable {
            reason: "agent execution backend is not configured".into(),
        };
        Self {
            descriptor,
            backend: None,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        execution: AgentExecutionResult,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            status: execution.status.as_str().into(),
            output: serde_json::to_value(execution)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            metadata: Default::default(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for AgentExecutionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        match command.name.as_str() {
            AGENT_EXECUTE_COMMAND => {
                let backend = self.backend.as_ref().ok_or_else(|| {
                    ServiceError::ServiceUnavailable(
                        "agent execution backend is not configured".into(),
                    )
                })?;
                let typed: AgentExecutionCommand = serde_json::from_value(command.payload)
                    .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))?;
                let execution = backend.execute(typed).await?;
                Self::result(execution, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported agent execution command {other}"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.descriptor.health.clone())
    }
}

/// Descriptor advertised by the Agent Execution service.
pub fn agent_execution_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
        ServiceType::new("agent.execution"),
        TraceSchemaRef::new("trace.system_service.agent_execution.v1"),
    );
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor
        .metadata
        .insert("command.agent.execute".into(), AGENT_EXECUTE_COMMAND.into());
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentExecutionIntent, ApplicationId};

    #[derive(Default)]
    struct FakeExecutionBackend;

    #[async_trait]
    impl AgentExecutionBackend for FakeExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            Ok(AgentExecutionResult::completed(
                &command,
                serde_json::json!({"output": "done"}),
            ))
        }
    }

    #[tokio::test]
    async fn execution_provider_returns_result_from_backend() {
        let provider = AgentExecutionSystemServiceProvider::new(Arc::new(FakeExecutionBackend));
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("demo"),
            "session-a",
            "worker",
            AgentExecutionIntent::WasmDelegate,
            "Analyze BTC",
            TraceContext::new("trace-execution-provider"),
        )
        .unwrap();

        let reply = provider
            .call(command.into_service_command().unwrap())
            .await
            .unwrap();
        let result: AgentExecutionResult = serde_json::from_value(reply.output).unwrap();
        assert_eq!(result.output["output"], "done");
        assert_eq!(reply.status, "completed");
    }

    #[tokio::test]
    async fn unavailable_execution_provider_fails_structurally() {
        let provider = AgentExecutionSystemServiceProvider::unavailable();
        let err = provider
            .call(ServiceCommand::with_trace(
                macaca_proto::ServiceCommandName::new(AGENT_EXECUTE_COMMAND),
                serde_json::json!({}),
                TraceContext::new("trace-execution-unavailable"),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::ServiceUnavailable(_)));
    }
}
