//! Temporary compatibility boundary for legacy provider construction.
//!
//! Route C wants the production kernel to own invariants and facades, not
//! provider construction. This module now converts the remaining migration-era
//! provider handles into an `AgentExecutionPort`, keeping the provider-shaped
//! bridge outside `Kernel` storage while callers migrate toward service-runtime
//! execution clients.

use std::sync::Arc;

pub use macaca_agent::AgentExecutionPort;
use macaca_agent::LegacyAgentExecutionAdapter;
pub use macaca_agent::LlmProvider as LegacyLlmProvider;
pub use macaca_agent::ToolCatalog as LegacyToolCatalog;
pub use macaca_agent::UnavailableAgentExecutionPort;

/// Migration bundle that groups the legacy provider handles still required by
/// the kernel compatibility path.
///
/// The bundle exists to make provider ownership explicit and searchable while
/// keeping the core kernel implementation focused on orchestration and agent
/// lifecycle behavior.
#[derive(Clone)]
#[deprecated(note = "Use service-client/SystemFacade construction for new kernel wiring")]
pub struct KernelProviderCompat {
    llm: Arc<dyn LegacyLlmProvider>,
    tools: Arc<dyn LegacyToolCatalog>,
}

#[allow(deprecated)]
impl KernelProviderCompat {
    /// Build the compatibility bundle from the legacy direct constructor inputs.
    ///
    /// The method keeps the old construction shape available for migration while
    /// making the new internal representation explicit and reusable.
    pub fn new(llm: Arc<dyn LegacyLlmProvider>, tools: Box<dyn LegacyToolCatalog>) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "kernel provider compatibility bundle created from legacy inputs"
        );
        Self {
            llm,
            tools: Arc::from(tools),
        }
    }

    /// Build the compatibility bundle from already shared provider handles.
    ///
    /// This is the preferred path for new internal wiring because it keeps the
    /// kernel composition layer provider-neutral and prevents repeated boxing.
    pub fn from_shared(llm: Arc<dyn LegacyLlmProvider>, tools: Arc<dyn LegacyToolCatalog>) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "kernel provider compatibility bundle created from shared handles"
        );
        Self { llm, tools }
    }

    /// Convert legacy handles into the provider-neutral execution port.
    ///
    /// The kernel consumes only the resulting port. This method is intentionally
    /// the only bridge that knows about both legacy provider traits, which makes
    /// the remaining compatibility debt searchable and easy to delete when the
    /// service execution client becomes the sole implementation.
    pub fn into_execution_port(self) -> Arc<dyn AgentExecutionPort> {
        tracing::info!(
            llm_provider = %self.llm.name(),
            "kernel provider compatibility converted into execution port"
        );
        Arc::new(LegacyAgentExecutionAdapter::new(self.llm, self.tools))
    }
}
