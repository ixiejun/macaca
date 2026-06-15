//! Runtime-host dispatcher and kernel wiring helper for `service.agent_execution`.
//!
//! `ServiceClientAgentExecutionAdapter` lives in `macaca-agent` and only needs an
//! `AgentExecutionDispatch` implementation. This module provides the runtime-host
//! implementation that sends typed commands through `ServiceRuntime` to the
//! already-registered `service.agent_execution` provider.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_agent::{AgentExecutionDispatch, ServiceClientAgentExecutionAdapter};
use macaca_kernel::Kernel;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, ApplicationId, KernelServiceId, MacacaError,
    MacacaResult, ServiceBusSource, AGENT_EXECUTION_SERVICE_ID,
};

use crate::ServiceRuntime;

/// Runtime-host implementation of `AgentExecutionDispatch`.
///
/// The dispatcher keeps kernel execution inside the service boundary:
/// `ServiceClientAgentExecutionAdapter -> AgentExecutionDispatch -> ServiceRuntime ->
/// service.agent_execution`.
#[derive(Clone)]
pub struct ServiceRuntimeAgentExecutionDispatch {
    runtime: Arc<ServiceRuntime>,
    source: ServiceBusSource,
}

impl ServiceRuntimeAgentExecutionDispatch {
    /// Build a dispatcher with an explicit service-bus source label.
    pub fn new(runtime: Arc<ServiceRuntime>, source: impl Into<String>) -> Self {
        Self {
            runtime,
            source: ServiceBusSource::new(source),
        }
    }

    /// Build a dispatcher using the default runtime-host source label.
    pub fn with_default_source(runtime: Arc<ServiceRuntime>) -> Self {
        Self::new(runtime, "runtime.host.kernel")
    }
}

#[async_trait]
impl AgentExecutionDispatch for ServiceRuntimeAgentExecutionDispatch {
    async fn dispatch_agent_execution(
        &self,
        command: AgentExecutionCommand,
    ) -> MacacaResult<AgentExecutionResult> {
        tracing::info!(
            service_id = AGENT_EXECUTION_SERVICE_ID,
            dispatch_source = %self.source,
            target_agent = %command.target_agent,
            trace_id = %command.trace.trace_id,
            command = "agent.execute",
            "runtime-host agent execution dispatch started"
        );
        let service_command = command.into_service_command()?;
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                self.source.clone(),
                service_command,
            )
            .await
            .map_err(|error| {
                tracing::warn!(
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    error = %error,
                    "runtime-host agent execution dispatch failed"
                );
                MacacaError::Agent(format!("agent execution service dispatch failed: {error}"))
            })?;
        if !reply.success {
            tracing::warn!(
                service_id = AGENT_EXECUTION_SERVICE_ID,
                status = %reply.status,
                "runtime-host agent execution dispatch returned unsuccessful reply"
            );
            return Err(MacacaError::Agent(format!(
                "agent execution service returned unsuccessful reply: {}",
                reply.status
            )));
        }
        let output = reply.output.ok_or_else(|| {
            MacacaError::Agent("agent execution service returned empty output".into())
        })?;
        let result: AgentExecutionResult = serde_json::from_value(output).map_err(|error| {
            MacacaError::Agent(format!(
                "agent execution service reply decode failed: {error}"
            ))
        })?;
        tracing::info!(
            service_id = AGENT_EXECUTION_SERVICE_ID,
            target_agent = %result.target_agent,
            trace_id = %result.trace.trace_id,
            status = %result.status.as_str(),
            "runtime-host agent execution dispatch completed"
        );
        Ok(result)
    }
}

/// Build a kernel execution port backed by `service.agent_execution`.
///
/// Composition roots pass the returned port to `KernelBuilder::from_execution_port`
/// so kernel agent dispatch routes through the runtime-host service boundary.
pub fn kernel_execution_port_from_agent_execution_service(
    runtime: Arc<ServiceRuntime>,
    application_id: ApplicationId,
) -> Arc<dyn macaca_agent::AgentExecutionPort> {
    tracing::info!(
        service_id = AGENT_EXECUTION_SERVICE_ID,
        application_id = %application_id,
        "kernel execution port created from agent execution service"
    );
    service_client_execution_port(runtime, application_id)
}

/// Build the production `ServiceClientAgentExecutionAdapter` for one runtime.
pub fn service_client_execution_port(
    runtime: Arc<ServiceRuntime>,
    application_id: ApplicationId,
) -> Arc<dyn macaca_agent::AgentExecutionPort> {
    let dispatch = Arc::new(ServiceRuntimeAgentExecutionDispatch::with_default_source(
        runtime,
    ));
    Arc::new(ServiceClientAgentExecutionAdapter::new(
        dispatch,
        application_id,
    ))
}

/// Hot-swap an already-bootstrapped kernel onto `service.agent_execution`.
///
/// Web seeds the kernel with a temporary bootstrap port for agent registration, then
/// calls this helper immediately after the Agent Execution service provider is
/// started so `Kernel::execute_agent` converges on the single service path.
pub async fn wire_kernel_to_agent_execution_service(
    kernel: &Kernel,
    runtime: Arc<ServiceRuntime>,
    application_id: ApplicationId,
) {
    tracing::info!(
        service_id = AGENT_EXECUTION_SERVICE_ID,
        application_id = %application_id.0,
        "wiring kernel execution port to agent execution service"
    );
    let port = service_client_execution_port(runtime, application_id);
    kernel.replace_execution_port(port).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_kernel::{KernelBuilder, SystemService, UnavailableAgentExecutionPort};
    use macaca_proto::{
        config::KernelConfig, AgentExecutionIntent, AgentExecutionStatus, AgentId, AgentManifest,
        AgentState, Permission, PermissionLevel, ServiceResult, TokenUsage, TraceContext,
    };

    use crate::{
        agent_execution_service_descriptor, AgentExecutionBackend,
        AgentExecutionSystemServiceProvider, ServiceProviderFactoryContext,
        ServiceProviderInstance, ServiceRuntimeConfig, StaticServiceProviderFactory,
    };

    /// Recording backend proving typed commands cross the provider boundary.
    #[derive(Default)]
    struct RecordingExecutionBackend {
        commands: Mutex<Vec<AgentExecutionCommand>>,
    }

    #[async_trait]
    impl AgentExecutionBackend for RecordingExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            self.commands.lock().unwrap().push(command.clone());
            Ok(AgentExecutionResult::completed(
                &command,
                serde_json::json!({
                    "result": "service-dispatched",
                    "tokens_used": TokenUsage::default(),
                }),
            ))
        }
    }

    async fn register_agent_execution_provider(
        runtime: &Arc<ServiceRuntime>,
        backend: Arc<dyn AgentExecutionBackend>,
    ) {
        let provider = Arc::new(AgentExecutionSystemServiceProvider::new(backend));
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    agent_execution_service_descriptor(),
                    provider as Arc<dyn SystemService>,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .expect("register provider");
        runtime
            .start(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                TraceContext::new("test-agent-execution-dispatch-provider"),
            )
            .await
            .expect("start provider");
    }

    #[tokio::test]
    async fn runtime_dispatch_routes_command_to_agent_execution_provider() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let backend = Arc::new(RecordingExecutionBackend::default());
        register_agent_execution_provider(&runtime, backend.clone()).await;

        let dispatch = ServiceRuntimeAgentExecutionDispatch::with_default_source(runtime);
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("dispatch-test"),
            "session-dispatch",
            "worker",
            AgentExecutionIntent::SdkInvocation,
            "dispatch test command",
            TraceContext::new("trace-runtime-dispatch"),
        )
        .expect("command");
        let result = dispatch
            .dispatch_agent_execution(command.clone())
            .await
            .expect("dispatch result");

        assert_eq!(result.status, AgentExecutionStatus::Completed);
        assert_eq!(result.output["result"], "service-dispatched");
        let commands = backend.commands.lock().expect("lock");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].target_agent, command.target_agent);
        assert_eq!(commands[0].trace.trace_id, command.trace.trace_id);
    }

    #[tokio::test]
    async fn wire_kernel_hot_swap_routes_execute_agent_through_service() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let backend = Arc::new(RecordingExecutionBackend::default());
        register_agent_execution_provider(&runtime, backend.clone()).await;

        let kernel = KernelBuilder::from_execution_port(
            KernelConfig {
                max_agents: 4,
                heartbeat_interval_ms: 1_000,
                agent_timeout_ms: 5_000,
            },
            Arc::new(UnavailableAgentExecutionPort::new("bootstrap placeholder")),
        )
        .build();

        wire_kernel_to_agent_execution_service(
            &kernel,
            Arc::clone(&runtime),
            ApplicationId::from_name("wire-test"),
        )
        .await;

        let agent_id = AgentId::new();
        let manifest = AgentManifest {
            id: agent_id,
            name: "wired-agent".into(),
            capabilities: Vec::new(),
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: Vec::new(),
                allowed_paths: Vec::new(),
                network_access: false,
            },
            state: AgentState::Running,
            created_at: Utc::now(),
            model: String::new(),
        };
        // Manifest-only registration: execution is delegated to the wired service port.
        kernel
            .register_agent(manifest)
            .await
            .expect("register agent manifest");

        let output = kernel
            .execute_agent(&agent_id)
            .await
            .expect("execute agent");
        assert_eq!(output.result, "service-dispatched");
        assert_eq!(backend.commands.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn kernel_execution_port_helper_uses_service_client_adapter() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let backend = Arc::new(RecordingExecutionBackend::default());
        register_agent_execution_provider(&runtime, backend.clone()).await;

        let kernel = KernelBuilder::from_execution_port(
            KernelConfig {
                max_agents: 4,
                heartbeat_interval_ms: 1_000,
                agent_timeout_ms: 5_000,
            },
            kernel_execution_port_from_agent_execution_service(
                Arc::clone(&runtime),
                ApplicationId::from_name("kernel-execution-port"),
            ),
        )
        .build();

        let agent_id = AgentId::new();
        let manifest = AgentManifest {
            id: agent_id,
            name: "service-client-adapter-agent".into(),
            capabilities: Vec::new(),
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: Vec::new(),
                allowed_paths: Vec::new(),
                network_access: false,
            },
            state: AgentState::Running,
            created_at: Utc::now(),
            model: String::new(),
        };
        kernel
            .register_agent(manifest)
            .await
            .expect("register agent manifest");

        let output = kernel
            .execute_agent(&agent_id)
            .await
            .expect("execute agent");
        assert_eq!(output.result, "service-dispatched");
        let commands = backend.commands.lock().expect("lock");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].target_agent, agent_id.0.to_string());
    }
}
