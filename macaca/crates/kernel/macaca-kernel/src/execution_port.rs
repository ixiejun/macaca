//! Kernel-owned execution port decorators and bootstrap placeholders.
//!
//! These types implement the proto [`AgentExecutionPort`] contract while keeping
//! swappable identity and unavailable semantics inside the microkernel crate.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{AgentExecutionPort, AgentId, AgentOutput, MacacaError, MacacaResult};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Hot-swappable wrapper around [`AgentExecutionPort`] for composition-root wiring.
///
/// Web bootstrap must register agents in the kernel before `service.agent_execution`
/// is available. This Decorator keeps one stable `Arc` identity on the kernel while
/// allowing the host to replace the inner port after service providers start,
/// without re-registering agents or rebuilding the kernel registry.
pub struct SwappableAgentExecutionPort {
    inner: RwLock<Arc<dyn AgentExecutionPort>>,
}

impl SwappableAgentExecutionPort {
    /// Create a swappable port seeded with the bootstrap execution implementation.
    pub fn new(initial: Arc<dyn AgentExecutionPort>) -> Arc<Self> {
        info!("swappable agent execution port created");
        Arc::new(Self {
            inner: RwLock::new(initial),
        })
    }

    /// Replace the active execution port after service providers become available.
    ///
    /// Callers must ensure the new port routes through `service.agent_execution`
    /// before relying on kernel `execute_agent` in production paths.
    pub async fn replace(&self, port: Arc<dyn AgentExecutionPort>) {
        info!(
            service_id = "service.agent_execution",
            "swappable agent execution port replacement started"
        );
        let mut guard = self.inner.write().await;
        *guard = port;
        info!(
            service_id = "service.agent_execution",
            "swappable agent execution port replacement completed"
        );
    }
}

#[async_trait]
impl AgentExecutionPort for SwappableAgentExecutionPort {
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        let guard = self.inner.read().await;
        guard.execute_registered_agent(agent_id).await
    }
}

/// Null Object execution port used when no execution service bridge is wired.
///
/// This object preserves service-client construction safety. A kernel can be
/// assembled before the Agent Execution service is registered, but any attempt to
/// run an agent receives a clear unavailable error instead of a fake success.
pub struct UnavailableAgentExecutionPort {
    reason: String,
}

impl UnavailableAgentExecutionPort {
    /// Create a reusable unavailable execution port with an operator-facing reason.
    ///
    /// The reason must stay generic and must not include secrets, prompts,
    /// manifests, package bytes, or application-specific payloads.
    pub fn new(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        info!(
            reason = %reason,
            "unavailable agent execution port created"
        );
        Self { reason }
    }
}

#[async_trait]
impl AgentExecutionPort for UnavailableAgentExecutionPort {
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        warn!(
            agent_id = %agent_id.0,
            reason = %self.reason,
            "agent execution requested without an execution bridge"
        );
        Err(MacacaError::Agent(format!(
            "Agent execution unavailable: {}",
            self.reason
        )))
    }
}
