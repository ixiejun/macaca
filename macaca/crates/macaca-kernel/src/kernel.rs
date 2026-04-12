//! Kernel — the central orchestrator that ties agents, LLM, tools, and services together.

use std::sync::Arc;

use macaca_agent::AgentServices;
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{
    AgentActivity, AgentId, AgentManifest, AgentOutput, AgentState, MacacaError, MacacaResult,
};
use macaca_tools::ToolSet;

use crate::registry::AgentRegistry;
use crate::scheduler::{Scheduler, SimpleScheduler};
use crate::status::AgentStatusTracker;

/// The core kernel that manages agents and orchestrates task execution.
pub struct Kernel {
    registry: AgentRegistry,
    scheduler: Box<dyn Scheduler>,
    status_tracker: AgentStatusTracker,
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolSet>,
}

impl Kernel {
    /// Create a new kernel with the given configuration.
    pub fn new(config: &KernelConfig, llm: Arc<dyn LlmProvider>, tools: Box<dyn ToolSet>) -> Self {
        Self {
            registry: AgentRegistry::new(config.max_agents),
            scheduler: Box::new(SimpleScheduler),
            status_tracker: AgentStatusTracker::new(),
            llm,
            tools: Arc::from(tools),
        }
    }

    /// Register a new agent with the kernel.
    pub async fn register_agent(
        &self,
        agent: Box<dyn macaca_agent::Agent>,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId> {
        let id = manifest.id;
        let name = manifest.name.clone();
        self.registry.register(agent, manifest).await?;
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
    /// Builds an `AgentServices` bundle (empty for now — real injection in Phase 4+)
    /// and invokes `agent.run()`.
    pub async fn execute_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        let services = AgentServices::empty();
        let llm = Arc::clone(&self.llm);
        let tools = Arc::clone(&self.tools);

        // Mark as thinking
        self.status_tracker
            .set_thinking(agent_id, "executing agent")
            .await;

        let map = self.registry.agents_read().await;
        let entry = map
            .get(agent_id)
            .ok_or_else(|| MacacaError::NotFound(format!("Agent {} not found", agent_id.0)))?;
        let output = entry
            .agent
            .run(llm.as_ref(), tools.as_ref(), &services)
            .await;

        // Mark as idle after execution
        self.status_tracker.set_idle(agent_id).await;

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
    use macaca_agent::Agent;
    use macaca_proto::{
        AgentState, Capability, LlmMessage, LlmOptions, LlmResponse, Permission, PermissionLevel,
        TokenUsage,
    };
    use macaca_tools::DefaultToolSet;

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
            _tools: &dyn ToolSet,
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

    fn make_kernel() -> Kernel {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        Kernel::new(&config, llm, Box::new(DefaultToolSet::new()))
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
        let kernel = make_kernel();
        let (id, agent, manifest) = make_test_agent();
        kernel.register_agent(agent, manifest).await.unwrap();
        assert_eq!(kernel.agent_count().await, 1);
        let agents = kernel.list_agents().await;
        assert_eq!(agents[0].id, id);
    }

    #[tokio::test]
    async fn execute_agent_calls_llm() {
        let kernel = make_kernel();
        let (id, agent, manifest) = make_test_agent();
        kernel.register_agent(agent, manifest).await.unwrap();
        let output = kernel.execute_agent(&id).await.unwrap();
        assert_eq!(output.result, "kernel test output");
        assert_eq!(output.tokens_used.total_tokens, 8);
    }

    #[tokio::test]
    async fn execute_missing_agent_returns_error() {
        let kernel = make_kernel();
        let err = kernel.execute_agent(&AgentId::new()).await.unwrap_err();
        assert!(matches!(err, MacacaError::NotFound(_)));
    }
}
