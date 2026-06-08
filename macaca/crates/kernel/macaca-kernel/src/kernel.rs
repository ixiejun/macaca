//! Kernel — the central orchestrator for agent identity, status, and execution dispatch.

use std::sync::Arc;

use macaca_proto::config::KernelConfig;
use macaca_proto::AgentExecutionPort;

use crate::execution_port::SwappableAgentExecutionPort;
use macaca_proto::{
    AgentActivity, AgentId, AgentManifest, AgentOutput, AgentState, MacacaError, MacacaResult,
};

use crate::registry::AgentRegistry;
use crate::scheduler::Scheduler;
use crate::status::AgentStatusTracker;

/// The core kernel that manages agents and orchestrates task execution.
pub struct Kernel {
    registry: AgentRegistry,
    scheduler: Box<dyn Scheduler>,
    status_tracker: AgentStatusTracker,
    /// Stable swappable port identity used by composition roots during bootstrap.
    execution_port: Arc<SwappableAgentExecutionPort>,
}

impl Kernel {
    pub(crate) fn from_parts(
        config: KernelConfig,
        execution_port: Arc<SwappableAgentExecutionPort>,
        scheduler: Box<dyn Scheduler>,
    ) -> Self {
        tracing::info!(
            max_agents = config.max_agents,
            "kernel created with swappable provider-neutral execution port"
        );
        Self {
            registry: AgentRegistry::new(config.max_agents),
            scheduler,
            status_tracker: AgentStatusTracker::new(),
            execution_port,
        }
    }

    /// Replace the active execution port after `service.agent_execution` is wired.
    ///
    /// Bootstrap hosts may seed the kernel with a legacy adapter for agent
    /// registration, then hot-swap to `ServiceClientAgentExecutionAdapter` once
    /// the runtime-host service provider is registered and started.
    pub async fn replace_execution_port(&self, port: Arc<dyn AgentExecutionPort>) {
        tracing::info!(
            service_id = "service.agent_execution",
            "kernel execution port hot-swap requested"
        );
        self.execution_port.replace(port).await;
    }

    /// Register a new agent manifest with the kernel (identity-only registry).
    pub async fn register_agent(&self, manifest: AgentManifest) -> MacacaResult<AgentId> {
        let id = manifest.id;
        let name = manifest.name.clone();
        self.registry.register(manifest).await?;
        // Register status tracking
        self.status_tracker.register(id, name).await;
        self.status_tracker
            .update_state(&id, AgentState::Running)
            .await;
        Ok(id)
    }

    /// Unregister an agent.
    pub async fn unregister_agent(&self, id: &AgentId) -> MacacaResult<()> {
        self.registry.unregister(id).await?;
        self.status_tracker.unregister(id).await;
        Ok(())
    }

    /// Execute a registered agent by ID.
    ///
    /// The kernel owns lookup, status transitions, and audit-friendly logs. The
    /// replaceable execution mechanics are delegated to `AgentExecutionPort`,
    /// which allows service-client execution, legacy adapters, or test doubles
    /// to be swapped without moving provider logic into the kernel.
    pub async fn execute_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        tracing::info!(agent_id = %agent_id.0, "kernel agent execution started");

        // Status tracking remains a kernel invariant because shells and service
        // clients depend on these lifecycle observations for diagnostics.
        self.status_tracker
            .set_thinking(agent_id, "executing agent")
            .await;

        if !self.registry.contains(agent_id).await {
            return Err(MacacaError::NotFound(format!(
                "Agent {} not found",
                agent_id.0
            )));
        }
        let output = self
            .execution_port
            .execute_registered_agent(agent_id)
            .await;

        // The current compatibility behavior marks agents idle after the
        // delegate returns, even when the delegate reports an error. Preserve
        // that observable transition while the execution service is introduced.
        self.status_tracker.set_idle(agent_id).await;
        match &output {
            Ok(result) => tracing::info!(
                agent_id = %agent_id.0,
                artifacts = result.artifacts.len(),
                total_tokens = result.tokens_used.total_tokens,
                "kernel agent execution finished"
            ),
            Err(error) => tracing::warn!(
                agent_id = %agent_id.0,
                error = %error,
                "kernel agent execution failed"
            ),
        }

        output
    }

    /// List all registered agents.
    pub async fn list_agents(&self) -> Vec<AgentManifest> {
        self.registry.list().await
    }

    /// Get a specific agent's manifest by name.
    pub async fn get_agent_by_name(&self, name: &str) -> Option<AgentManifest> {
        let agents = self.registry.list().await;
        agents.into_iter().find(|m| m.name == name)
    }

    /// Number of registered agents.
    pub async fn agent_count(&self) -> usize {
        self.registry.count().await
    }

    /// Get access to the scheduler.
    pub fn scheduler(&self) -> &dyn Scheduler {
        self.scheduler.as_ref()
    }

    /// Get access to the registry (for scheduler use).
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Get access to the status tracker.
    pub fn status_tracker(&self) -> &AgentStatusTracker {
        &self.status_tracker
    }

    /// Update agent activity status.
    pub async fn update_agent_activity(&self, agent_id: &AgentId, activity: AgentActivity) {
        self.status_tracker
            .update_activity(agent_id, activity)
            .await;
    }

    /// Get agent runtime status.
    pub async fn get_agent_status(
        &self,
        agent_id: &AgentId,
    ) -> Option<macaca_proto::AgentRuntimeStatus> {
        self.status_tracker.get(agent_id).await
    }

    /// Get all agent runtime statuses.
    pub async fn list_agent_statuses(&self) -> Vec<macaca_proto::AgentRuntimeStatus> {
        self.status_tracker.list().await
    }

    /// Get statuses for specific agents (e.g., agents of an app).
    pub async fn list_agent_statuses_for(
        &self,
        agent_ids: &[AgentId],
    ) -> Vec<macaca_proto::AgentRuntimeStatus> {
        self.status_tracker.list_for_agents(agent_ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_agent::{
        Agent, AgentServices, InProcessAgentExecutionPort, LlmProvider, ToolCatalog,
    };
    use macaca_proto::{
        AgentState, Capability, LlmMessage, LlmOptions, LlmResponse, Permission, PermissionLevel,
        TokenUsage,
    };

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "kernel test output".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 5,
                    completion_tokens: 3,
                    total_tokens: 8,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    struct TestAgent {
        id: AgentId,
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn id(&self) -> AgentId {
            self.id
        }
        fn capabilities(&self) -> &[Capability] {
            &[]
        }
        fn state(&self) -> AgentState {
            AgentState::Running
        }
        async fn run(
            &self,
            llm: &dyn LlmProvider,
            _tools: &dyn ToolCatalog,
            _services: &AgentServices,
        ) -> MacacaResult<AgentOutput> {
            let msgs = vec![LlmMessage::user("test")];
            let resp = llm.chat(msgs, &LlmOptions::default()).await?;
            Ok(AgentOutput {
                result: resp.content,
                artifacts: vec![],
                tokens_used: resp.usage,
            })
        }
    }

    fn make_kernel() -> (Kernel, Arc<macaca_agent::InProcessAgentSideRegistry>) {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let adapter = InProcessAgentExecutionPort::new(
            llm,
            Arc::from(Box::new(macaca_tools::DefaultToolSet::new()) as Box<dyn ToolCatalog>),
        );
        let side_registry = adapter.side_registry();
        let execution_port: Arc<dyn AgentExecutionPort> = Arc::new(adapter);
        let kernel = crate::KernelBuilder::from_execution_port(config, execution_port).build();
        (kernel, side_registry)
    }

    fn make_test_agent() -> (AgentId, Box<dyn Agent>, AgentManifest) {
        let id = AgentId::new();
        let agent: Box<dyn Agent> = Box::new(TestAgent { id });
        let manifest = AgentManifest {
            id,
            name: "test-agent".into(),
            capabilities: vec![],
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: vec![],
                allowed_paths: vec![],
                network_access: false,
            },
            state: AgentState::Running,
            created_at: Utc::now(),
            model: String::new(),
        };
        (id, agent, manifest)
    }

    #[tokio::test]
    async fn register_and_list() {
        let (kernel, side_registry) = make_kernel();
        let (id, agent, manifest) = make_test_agent();
        side_registry
            .register_runtime_agent(Arc::from(agent))
            .unwrap();
        kernel.register_agent(manifest).await.unwrap();
        assert_eq!(kernel.agent_count().await, 1);
        let agents = kernel.list_agents().await;
        assert_eq!(agents[0].id, id);
    }

    #[tokio::test]
    async fn execute_agent_calls_llm() {
        let (kernel, side_registry) = make_kernel();
        let (id, agent, manifest) = make_test_agent();
        side_registry
            .register_runtime_agent(Arc::from(agent))
            .unwrap();
        kernel.register_agent(manifest).await.unwrap();
        let output = kernel.execute_agent(&id).await.unwrap();
        assert_eq!(output.result, "kernel test output");
        assert_eq!(output.tokens_used.total_tokens, 8);
    }

    #[tokio::test]
    async fn execute_missing_agent_returns_error() {
        let (kernel, _side_registry) = make_kernel();
        let err = kernel.execute_agent(&AgentId::new()).await.unwrap_err();
        assert!(matches!(err, MacacaError::NotFound(_)));
    }
}
