//! [`SequentialPipeline`] — chain agents in series (Chain of Responsibility).
//!
//! Each agent's `reply` output becomes the next agent's input. An empty agent
//! list is a no-op passthrough, which makes sequential composition safe to
//! build incrementally at runtime.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::agent::{Agent, AgentResult};
use crate::message::Msg;

use super::core::Pipeline;

/// Execute agents in series: each agent's output becomes the next agent's input.
///
/// **Design pattern:** Chain of Responsibility — agents are visited in FIFO order;
/// the message is forwarded along the chain until the last agent produces the result.
///
/// An empty agent list returns the original message unchanged.
pub struct SequentialPipeline {
    /// Ordered agent chain; each element must be `Send + Sync` for async fan-out safety.
    agents: Vec<Arc<dyn Agent>>,
}

impl SequentialPipeline {
    /// Create a new sequential pipeline from a list of agents.
    pub fn new(agents: Vec<Arc<dyn Agent>>) -> Self {
        debug!(
            target = "macaca_framework::pipeline::sequential",
            agent_count = agents.len(),
            "SequentialPipeline constructed"
        );
        Self { agents }
    }
}

#[async_trait]
impl Pipeline for SequentialPipeline {
    async fn run(&self, msg: Msg) -> AgentResult<Msg> {
        debug!(
            target = "macaca_framework::pipeline::sequential",
            agent_count = self.agents.len(),
            input_sender = %msg.name,
            "SequentialPipeline::run start"
        );

        let mut current = msg;
        for (index, agent) in self.agents.iter().enumerate() {
            debug!(
                target = "macaca_framework::pipeline::sequential",
                step = index,
                agent_name = agent.name(),
                "delegating reply to chained agent"
            );
            current = agent.reply(current).await?;
        }

        debug!(
            target = "macaca_framework::pipeline::sequential",
            output_sender = %current.name,
            "SequentialPipeline::run complete"
        );
        Ok(current)
    }
}
