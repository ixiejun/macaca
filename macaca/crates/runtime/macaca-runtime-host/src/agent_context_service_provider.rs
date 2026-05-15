//! Agent Context system service provider.
//!
//! The provider owns the ServiceRuntime-facing command surface while concrete
//! context construction remains replaceable behind [`AgentContextBackend`].
//! This keeps Web and future hosts as composition roots, not protocol owners.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, CleanupPolicy, KernelServiceId,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
    AGENT_CONTEXT_BUILD_COMMAND, AGENT_CONTEXT_SERVICE_ID,
};

/// Replaceable backend that builds trusted system context for one agent run.
#[async_trait]
pub trait AgentContextBackend: Send + Sync {
    async fn build_context(
        &self,
        command: AgentContextBuildCommand,
    ) -> ServiceResult<AgentContextSnapshot>;
}

/// ServiceRuntime-facing provider for `service.agent_context`.
pub struct AgentContextSystemServiceProvider {
    descriptor: ServiceDescriptor,
    backend: Option<Arc<dyn AgentContextBackend>>,
}

impl AgentContextSystemServiceProvider {
    /// Create a provider with a concrete backend.
    pub fn new(backend: Arc<dyn AgentContextBackend>) -> Self {
        Self {
            descriptor: agent_context_service_descriptor(),
            backend: Some(backend),
        }
    }

    /// Create an explicit unavailable provider for hosts without context
    /// construction support.
    pub fn unavailable() -> Self {
        let mut descriptor = agent_context_service_descriptor();
        descriptor.health = ServiceHealth::Unavailable {
            reason: "agent context backend is not configured".into(),
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
        snapshot: AgentContextSnapshot,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(snapshot)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: Default::default(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for AgentContextSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        match command.name.as_str() {
            AGENT_CONTEXT_BUILD_COMMAND => {
                let backend = self.backend.as_ref().ok_or_else(|| {
                    ServiceError::ServiceUnavailable(
                        "agent context backend is not configured".into(),
                    )
                })?;
                let typed: AgentContextBuildCommand = serde_json::from_value(command.payload)
                    .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))?;
                let snapshot = backend.build_context(typed).await?;
                Self::result(snapshot, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported agent context command {other}"
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

/// Descriptor advertised by the Agent Context service.
pub fn agent_context_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(AGENT_CONTEXT_SERVICE_ID),
        ServiceType::new("agent.context"),
        TraceSchemaRef::new("trace.system_service.agent_context.v1"),
    );
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.metadata.insert(
        "command.agent.context.build".into(),
        AGENT_CONTEXT_BUILD_COMMAND.into(),
    );
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentExecutionIntent, AgentExecutionPolicyContext, ApplicationId};

    #[derive(Default)]
    struct FakeContextBackend;

    #[async_trait]
    impl AgentContextBackend for FakeContextBackend {
        async fn build_context(
            &self,
            command: AgentContextBuildCommand,
        ) -> ServiceResult<AgentContextSnapshot> {
            Ok(AgentContextSnapshot::minimal(&command, "trusted context"))
        }
    }

    #[tokio::test]
    async fn context_provider_returns_snapshot_from_backend() {
        let provider = AgentContextSystemServiceProvider::new(Arc::new(FakeContextBackend));
        let trace = TraceContext::new("trace-context-provider");
        let command = AgentContextBuildCommand {
            application_id: ApplicationId::from_name("demo"),
            session_id: "session-a".into(),
            task_id: None,
            target_agent: "worker".into(),
            execution_intent: AgentExecutionIntent::WasmDelegate,
            trace: trace.clone(),
            policy: AgentExecutionPolicyContext::default(),
            context_budget_tokens: None,
            metadata: Default::default(),
        };
        let reply = provider
            .call(command.into_service_command().unwrap())
            .await
            .unwrap();
        let snapshot: AgentContextSnapshot = serde_json::from_value(reply.output).unwrap();
        assert_eq!(snapshot.system_prompt, "trusted context");
        assert_eq!(snapshot.target_agent, "worker");
    }

    #[tokio::test]
    async fn unavailable_context_provider_fails_structurally() {
        let provider = AgentContextSystemServiceProvider::unavailable();
        let err = provider
            .call(ServiceCommand::with_trace(
                macaca_proto::ServiceCommandName::new(AGENT_CONTEXT_BUILD_COMMAND),
                serde_json::json!({}),
                TraceContext::new("trace-context-unavailable"),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::ServiceUnavailable(_)));
    }
}
