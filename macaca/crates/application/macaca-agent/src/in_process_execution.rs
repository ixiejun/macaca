//! In-process agent execution port for composition roots and integration tests.
//!
//! **Adapter pattern**: bridges the kernel's [`AgentExecutionPort`] contract to the
//! in-process `Agent::run(llm, tools, services)` ABI. Provider handles (LLM, tools)
//! and runtime [`Agent`] instances live outside the microkernel; the kernel registry
//! stores manifests only while [`InProcessAgentSideRegistry`] maps `agent_id` → runtime
//! agent objects.
//!
//! Production composition roots MUST prefer [`ServiceClientAgentExecutionAdapter`]
//! (see `execution.rs`), which dispatches through `service.agent_execution` and
//! produces auditable service-call evidence. This module remains for hermetic unit
//! tests, SDK bootstrap fixtures, and other non-production wiring that intentionally
//! bypasses the service runtime.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use macaca_proto::{
    AgentExecutionPort, AgentId, AgentOutput, MacacaError, MacacaResult,
};

use crate::{Agent, AgentServices, LlmProvider, ToolCatalog};

/// Side registry holding runtime [`Agent`] instances for in-process execution.
///
/// The kernel manifest registry stores identity metadata only. This companion
/// registry resolves `agent_id` → `Arc<dyn Agent>` so [`InProcessAgentExecutionPort`]
/// can invoke `Agent::run` without pulling application-agent types into the kernel.
#[derive(Default)]
pub struct InProcessAgentSideRegistry {
    agents: RwLock<HashMap<AgentId, Arc<dyn Agent>>>,
}

impl InProcessAgentSideRegistry {
    /// Creates an empty side registry ready for runtime agent registration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a runtime agent instance keyed by its stable kernel id.
    ///
    /// Logs the registration for traceability during bootstrap and test setup.
    pub fn register_runtime_agent(&self, agent: Arc<dyn Agent>) -> MacacaResult<()> {
        let agent_id = agent.id();
        let mut agents = self.agents.write().map_err(|_| {
            MacacaError::Agent("in-process agent side registry lock poisoned".into())
        })?;
        agents.insert(agent_id, agent);
        tracing::info!(
            agent_id = %agent_id.0,
            "in-process runtime agent registered in side registry"
        );
        Ok(())
    }

    /// Resolves a runtime agent by kernel id, returning `None` when unregistered.
    pub fn get_runtime_agent(&self, agent_id: &AgentId) -> Option<Arc<dyn Agent>> {
        self.agents
            .read()
            .ok()
            .and_then(|agents| agents.get(agent_id).cloned())
    }
}

/// In-process [`AgentExecutionPort`] that invokes `Agent::run` with injected providers.
///
/// **Hexagonal outbound port**: keeps LLM/tool provider construction in composition
/// roots while the kernel depends only on the typed `AgentExecutionPort` trait.
pub struct InProcessAgentExecutionPort {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolCatalog>,
    side_registry: Arc<InProcessAgentSideRegistry>,
}

impl InProcessAgentExecutionPort {
    /// Shared side registry used by default bootstrap and test wiring.
    pub fn runtime_registry() -> Arc<InProcessAgentSideRegistry> {
        static REGISTRY: std::sync::OnceLock<Arc<InProcessAgentSideRegistry>> =
            std::sync::OnceLock::new();
        REGISTRY
            .get_or_init(|| Arc::new(InProcessAgentSideRegistry::new()))
            .clone()
    }

    /// Creates a port using the process-wide shared side registry.
    pub fn new(llm: Arc<dyn LlmProvider>, tools: Arc<dyn ToolCatalog>) -> Self {
        Self::new_with_registry(llm, tools, Self::runtime_registry())
    }

    /// Creates a port bound to an explicit side registry (tests / composition roots).
    pub fn new_with_registry(
        llm: Arc<dyn LlmProvider>,
        tools: Arc<dyn ToolCatalog>,
        side_registry: Arc<InProcessAgentSideRegistry>,
    ) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "in-process agent execution port created"
        );
        Self {
            llm,
            tools,
            side_registry,
        }
    }

    /// Returns the side registry used by this port for dual manifest + runtime registration.
    pub fn side_registry(&self) -> Arc<InProcessAgentSideRegistry> {
        Arc::clone(&self.side_registry)
    }
}

#[async_trait]
impl AgentExecutionPort for InProcessAgentExecutionPort {
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        tracing::info!(
            agent_id = %agent_id.0,
            llm_provider = %self.llm.name(),
            execution_mode = "in_process",
            "in-process agent execution port dispatch started"
        );
        let agent = self
            .side_registry
            .get_runtime_agent(agent_id)
            .ok_or_else(|| MacacaError::NotFound(format!("runtime agent not found: {agent_id}")))?;
        let services = AgentServices::builder().build();
        let output = agent
            .run(self.llm.as_ref(), self.tools.as_ref(), &services)
            .await;
        match &output {
            Ok(result) => tracing::info!(
                agent_id = %agent_id.0,
                artifacts = result.artifacts.len(),
                total_tokens = result.tokens_used.total_tokens,
                execution_mode = "in_process",
                "in-process agent execution port dispatch completed"
            ),
            Err(error) => tracing::warn!(
                agent_id = %agent_id.0,
                error = %error,
                execution_mode = "in_process",
                "in-process agent execution port dispatch failed"
            ),
        }
        output
    }
}
