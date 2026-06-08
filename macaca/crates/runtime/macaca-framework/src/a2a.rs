//! A2A (Agent-to-Agent) protocol support (**Facade** module tree).
//!
//! Vendor-neutral inter-agent discovery and message exchange:
//! - [`AgentCard`] for service discovery
//! - [`A2AMessage`], [`A2ATask`], [`A2AArtifact`] for agent communication
//! - [`A2AFormatter`] for bidirectional conversion between internal [`Msg`] and A2A types
//! - [`AgentCardResolver`] trait and [`FileCardResolver`] implementation
//!
//! # Module tree (P4 iteration 106)
//! - `types` — wire-protocol value objects (discovery, messages, tasks, RPC envelopes)
//! - `error` — [`A2AError`] taxonomy by failure domain
//! - `formatter` — [`A2AFormatter`] Adapter (`Msg` ↔ A2A)
//! - `resolver` — [`AgentCardResolver`] Strategy + [`FileCardResolver`] filesystem backend
//! - `tests` — serde round-trips, formatter boundary policy, resolver contract tests

mod error;
mod formatter;
mod resolver;
mod types;

#[cfg(test)]
mod tests;

pub use error::A2AError;
pub use formatter::A2AFormatter;
pub use resolver::{AgentCardResolver, FileCardResolver};
pub use types::{
    A2AArtifact, A2AFile, A2AMessage, A2APart, A2ARole, A2ATask, A2ATaskState, A2ATaskStatus,
    AgentCapabilities, AgentCard, AgentSkillInfo, SendMessageRequest, SendMessageResponse,
};
