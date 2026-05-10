//! Agent trait.

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{AgentId, AgentOutput, AgentState, Capability, MacacaResult};
use macaca_tools::ToolCatalog;

use crate::services::AgentServices;

// ── Agent trait ───────────────────────────────────────────────────────────────

/// The core trait every Agent OS agent must implement.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent instance.
    fn id(&self) -> AgentId;

    /// The capabilities this agent declares.
    fn capabilities(&self) -> &[Capability];

    /// Current lifecycle state of the agent.
    fn state(&self) -> AgentState;

    /// Execute the agent's main logic.
    async fn run(
        &self,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        services: &AgentServices,
    ) -> MacacaResult<AgentOutput>;
}
