//! Temporary compatibility boundary for legacy kernel provider construction.
//!
//! Route C wants the kernel to own invariants and facades, not provider
//! construction. This module isolates the remaining migration-era provider
//! handles behind a narrow bundle so kernel composition can gradually move
//! toward provider-neutral and service-runtime-based entry points.

use std::sync::Arc;

pub use macaca_agent::LlmProvider as LegacyLlmProvider;
pub use macaca_tools::ToolCatalog as LegacyToolCatalog;

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

    /// Return the legacy LLM provider handle for the existing agent execution path.
    pub fn llm(&self) -> &dyn LegacyLlmProvider {
        self.llm.as_ref()
    }

    /// Return the legacy tool catalog handle for the existing agent execution path.
    pub fn tools(&self) -> &dyn LegacyToolCatalog {
        self.tools.as_ref()
    }
}
