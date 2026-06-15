//! Provider-neutral kernel execution port contract.
//!
//! The microkernel dispatches registered agents through this Port abstraction.
//! Concrete execution (service-client adapter, in-process test adapter, unavailable
//! placeholder) lives in upper layers; the trait itself stays in foundation proto
//! so `macaca-kernel` does not depend on application-agent crates.

use async_trait::async_trait;

use crate::{AgentId, AgentOutput, MacacaResult};

/// Provider-neutral port used by kernel code to execute one registered agent.
///
/// This is the inbound Port in a Hexagonal layout: the kernel owns identity,
/// registry lookup, and status transitions, while replaceable implementations
/// decide how execution actually happens (typically `service.agent_execution`).
/// The contract intentionally references only `AgentId` so the kernel registry
/// stores manifests/identity without holding application `Agent` trait objects.
#[async_trait]
pub trait AgentExecutionPort: Send + Sync {
    /// Execute the registered agent identified by `agent_id`.
    ///
    /// Implementations must emit trace-friendly logs at key execution points and
    /// return structured errors when execution is unavailable. The port must not
    /// branch on application names, provider names, model names, or business-domain
    /// identifiers.
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput>;
}
