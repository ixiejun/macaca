//! Core [`Agent`] trait — the framework process abstraction.
//!
//! **Design pattern:** Strategy — concrete agents (ReAct, pipelines, etc.) plug in
//! different `reply` / `observe` behavior behind one stable interface.

use async_trait::async_trait;
use macaca_proto::AgentId;

use crate::message::Msg;

use super::error::AgentResult;

/// The core Agent trait implemented by every framework agent.
///
/// Essential methods:
/// - [`Agent::reply`] — generate a response to an input message
/// - [`Agent::observe`] — receive a message without generating a response
#[async_trait]
pub trait Agent: Send + Sync {
    /// Generate a reply to the given message.
    ///
    /// Implementations may call LLMs, execute tools, consult memory, etc.
    /// Application-specific behavior is configured above this layer (personas).
    async fn reply(&self, msg: Msg) -> AgentResult<Msg>;

    /// Observe a message without generating a reply.
    ///
    /// Used in multi-agent scenarios where an agent must be aware of messages
    /// exchanged between peers. Default is a no-op; memory-backed agents override.
    async fn observe(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    /// Request interruption of the current reply execution.
    ///
    /// Default is a no-op. Agents that support cancellation should check a signal
    /// during their reply loop.
    async fn interrupt(&self, _msg: Msg) -> AgentResult<()> {
        Ok(())
    }

    /// The agent's display name (configured by the hosting application).
    fn name(&self) -> &str;

    /// The agent's unique identifier.
    fn id(&self) -> &AgentId;
}
