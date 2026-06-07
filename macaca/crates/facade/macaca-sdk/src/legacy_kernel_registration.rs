//! Legacy kernel registration helper for manifest + runtime agent wiring.
//!
//! The kernel registry is manifest-only; runtime [`Agent`] instances are held in
//! [`LegacyAgentSideRegistry`] and resolved by the configured [`AgentExecutionPort`].

use std::sync::Arc;

use macaca_agent::{Agent, LegacyAgentSideRegistry};
use macaca_kernel::Kernel;
use macaca_proto::{AgentId, AgentManifest, MacacaResult};

/// Register a runtime agent in the side registry and its manifest in the kernel.
pub async fn register_legacy_kernel_agent(
    kernel: &Kernel,
    side_registry: &LegacyAgentSideRegistry,
    agent: Box<dyn Agent>,
    manifest: AgentManifest,
) -> MacacaResult<AgentId> {
    let id = manifest.id;
    side_registry.register_runtime_agent(Arc::from(agent))?;
    kernel.register_agent(manifest).await?;
    tracing::info!(
        agent_id = %id.0,
        "legacy kernel agent registered (manifest + runtime side registry)"
    );
    Ok(id)
}
