//! In-process kernel registration helper for manifest + runtime agent wiring.
//!
//! The kernel registry is manifest-only; runtime [`Agent`] instances are held in
//! [`InProcessAgentSideRegistry`] and resolved by the configured [`AgentExecutionPort`]
//! when tests or fixtures use [`InProcessAgentExecutionPort`].

use std::sync::Arc;

use macaca_agent::{Agent, InProcessAgentSideRegistry};
use macaca_kernel::Kernel;
use macaca_proto::{AgentId, AgentManifest, MacacaResult};

/// Register a runtime agent in the side registry and its manifest in the kernel.
///
/// Dual-write pattern: kernel stores the declarative manifest while the side registry
/// holds the executable `Agent` instance for in-process execution ports.
pub async fn register_in_process_kernel_agent(
    kernel: &Kernel,
    side_registry: &InProcessAgentSideRegistry,
    agent: Box<dyn Agent>,
    manifest: AgentManifest,
) -> MacacaResult<AgentId> {
    let id = manifest.id;
    side_registry.register_runtime_agent(Arc::from(agent))?;
    kernel.register_agent(manifest).await?;
    tracing::info!(
        agent_id = %id.0,
        "in-process kernel agent registered (manifest + runtime side registry)"
    );
    Ok(id)
}
