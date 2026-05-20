//! Provider-neutral execution ports for registered agents.
//!
//! This module keeps the legacy `Agent::run(llm, tools, services)` ABI outside
//! the kernel.  The kernel should coordinate identity, registry, status, and
//! scheduling invariants; replaceable provider handles belong behind an
//! execution port owned by the application-agent boundary until the execution
//! service contract fully replaces the legacy ABI.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{AgentId, AgentOutput, MacacaError, MacacaResult};

use crate::{Agent, AgentServices, LlmProvider, ToolCatalog};

/// Provider-neutral port used by kernel code to execute one registered agent.
///
/// This is a small Port pattern boundary: callers pass an already-registered
/// agent and its stable identifier, while the implementation decides how that
/// agent is actually run.  Service-era implementations can dispatch typed
/// commands through a runtime service client; the temporary legacy adapter below
/// still bridges to `Agent::run` without forcing kernel code to store concrete
/// provider compatibility bundles.
#[async_trait]
pub trait AgentExecutionPort: Send + Sync {
    /// Execute the supplied agent and return its output.
    ///
    /// Implementations must emit trace-friendly logs at key execution points
    /// and return structured errors when execution is unavailable.  The port is
    /// intentionally generic and carries no application-specific workflow,
    /// provider, model, driver, gateway, or business-domain branching.
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput>;
}

/// Legacy adapter for the current `Agent::run` execution ABI.
///
/// The adapter is an explicit Adapter pattern implementation. It contains the
/// provider-shaped bridge that existing agents still require, while keeping
/// production kernel state provider-neutral. Once agent execution is fully
/// serviceized, this adapter can be replaced by a service-client implementation
/// without changing the kernel registry or status code.
pub struct LegacyAgentExecutionAdapter {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolCatalog>,
}

impl LegacyAgentExecutionAdapter {
    /// Create the adapter from shared provider handles.
    ///
    /// Handles are shared so composition roots can own provider lifecycles while
    /// this adapter only borrows them during execution. The log records the
    /// provider family name for operational traceability without exposing raw
    /// prompts, credentials, or provider payloads.
    pub fn new(llm: Arc<dyn LlmProvider>, tools: Arc<dyn ToolCatalog>) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "legacy agent execution adapter created"
        );
        Self { llm, tools }
    }
}

#[async_trait]
impl AgentExecutionPort for LegacyAgentExecutionAdapter {
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        tracing::info!(
            agent_id = %agent_id.0,
            llm_provider = %self.llm.name(),
            "legacy agent execution adapter started"
        );
        let services = AgentServices::builder().build();
        let output = agent
            .run(self.llm.as_ref(), self.tools.as_ref(), &services)
            .await;
        match &output {
            Ok(result) => tracing::info!(
                agent_id = %agent_id.0,
                artifacts = result.artifacts.len(),
                total_tokens = result.tokens_used.total_tokens,
                "legacy agent execution adapter finished"
            ),
            Err(error) => tracing::warn!(
                agent_id = %agent_id.0,
                error = %error,
                "legacy agent execution adapter failed"
            ),
        }
        output
    }
}

/// Null Object execution port used when no execution service bridge is wired.
///
/// This object preserves service-client construction safety. A kernel can be
/// assembled before the legacy execution bridge exists, but any attempt to run
/// an agent receives a clear unavailable error instead of a fake success.
pub struct UnavailableAgentExecutionPort {
    reason: String,
}

impl UnavailableAgentExecutionPort {
    /// Create a reusable unavailable execution port with an operator-facing
    /// reason. The reason must stay generic and must not include secrets,
    /// prompts, manifests, package bytes, or application-specific payloads.
    pub fn new(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        tracing::info!(
            reason = %reason,
            "unavailable agent execution port created"
        );
        Self { reason }
    }
}

#[async_trait]
impl AgentExecutionPort for UnavailableAgentExecutionPort {
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        _agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        tracing::warn!(
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
