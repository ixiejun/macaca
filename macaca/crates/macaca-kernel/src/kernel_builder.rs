//! Additive kernel construction facade.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::config::KernelConfig;
use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaError, MacacaResult};

use crate::{
    Kernel, KernelProviderCompat, LegacyLlmProvider, LegacyToolCatalog, SchedulerFactory,
    SchedulerKind,
};

/// Builder for constructing a [`Kernel`] from explicit runtime dependencies.
pub struct KernelBuilder {
    config: KernelConfig,
    providers: KernelProviderCompat,
    scheduler_kind: SchedulerKind,
}

/// Provider-neutral kernel construction bundle for service-client-era callers.
///
/// The kernel execution path still accepts the legacy `Agent::run` provider
/// shape, so this bundle uses an internal unavailable adapter until agent
/// execution itself is serviceized. New callers can depend on this struct
/// instead of constructing `KernelProviderCompat` directly.
pub struct KernelServiceClientCompat {
    llm: Arc<dyn LegacyLlmProvider>,
    tools: Arc<dyn LegacyToolCatalog>,
}

impl KernelServiceClientCompat {
    /// Create a bundle from shared tool catalog handles.
    pub fn new(tools: Arc<dyn LegacyToolCatalog>) -> Self {
        Self {
            llm: Arc::new(UnavailableKernelLlmProvider),
            tools,
        }
    }

    /// Create a bundle from boxed tool catalog handles.
    pub fn from_boxed_tools(tools: Box<dyn LegacyToolCatalog>) -> Self {
        Self {
            llm: Arc::new(UnavailableKernelLlmProvider),
            tools: Arc::from(tools),
        }
    }

    /// Create a bundle with an agent-level LLM bridge for legacy `Agent::run` execution.
    ///
    /// Route C S5 removes direct provider construction from upper layers, but
    /// the historical `Agent::run` ABI still receives an `LlmProvider`
    /// reference.  This method concentrates that temporary bridge in the
    /// construction seam instead of spreading deprecated `KernelProviderCompat`
    /// calls through Web, SDK fixtures, and integration tests.
    pub fn from_agent_provider(
        llm: Arc<dyn LegacyLlmProvider>,
        tools: Arc<dyn LegacyToolCatalog>,
    ) -> Self {
        Self { llm, tools }
    }

    /// Create a bundle with an agent-level LLM bridge and boxed tool catalog.
    pub fn from_agent_provider_boxed_tools(
        llm: Arc<dyn LegacyLlmProvider>,
        tools: Box<dyn LegacyToolCatalog>,
    ) -> Self {
        Self::from_agent_provider(llm, Arc::from(tools))
    }

    /// Convert to the deprecated compatibility bundle in one isolated place.
    #[allow(deprecated)]
    fn into_provider_compat(self) -> KernelProviderCompat {
        KernelProviderCompat::from_shared(self.llm, self.tools)
    }
}

/// Internal null-object provider used only by the service-client construction seam.
///
/// It preserves the previous CLI/status behavior: if legacy execution is
/// accidentally invoked without a real LLM service bridge, callers receive a
/// structured unavailable error instead of a fake model response.
struct UnavailableKernelLlmProvider;

#[async_trait]
impl LegacyLlmProvider for UnavailableKernelLlmProvider {
    fn name(&self) -> &str {
        "kernel-service-client-unavailable-llm"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Err(MacacaError::Llm(
            "Kernel service-client construction has no legacy LLM provider".into(),
        ))
    }
}

impl KernelBuilder {
    /// Create a builder with default scheduler behavior.
    ///
    /// This is a deprecated migration entry that preserves the legacy
    /// provider-shaped constructor while routing the actual composition through
    /// the explicit compatibility bundle.
    #[deprecated(note = "use KernelBuilder::from_compat for new kernel construction")]
    pub fn new(
        config: KernelConfig,
        llm: Arc<dyn LegacyLlmProvider>,
        tools: Box<dyn LegacyToolCatalog>,
    ) -> Self {
        Self::from_compat(config, KernelProviderCompat::new(llm, tools))
    }

    /// Create a builder from the provider compatibility bundle.
    ///
    /// This is the preferred kernel construction path for new internal code
    /// because it makes the migration boundary explicit without spreading
    /// provider handles across the kernel core.
    pub fn from_compat(config: KernelConfig, providers: KernelProviderCompat) -> Self {
        Self {
            config,
            providers,
            scheduler_kind: SchedulerKind::default(),
        }
    }

    /// Create a builder from the provider-neutral service-client compatibility bundle.
    ///
    /// This is the preferred S5 construction seam for new code because it does
    /// not require callers to import or instantiate `LegacyLlmProvider`. The
    /// old provider bundle stays available as a deprecated migration memento.
    pub fn from_service_clients(config: KernelConfig, compat: KernelServiceClientCompat) -> Self {
        Self::from_compat(config, compat.into_provider_compat())
    }

    /// Override the scheduler strategy.
    pub fn scheduler_kind(mut self, scheduler_kind: SchedulerKind) -> Self {
        self.scheduler_kind = scheduler_kind;
        self
    }

    /// Build the kernel.
    pub fn build(self) -> Kernel {
        let config = self.config;
        tracing::info!(
            max_agents = config.max_agents,
            scheduler_kind = ?self.scheduler_kind,
            "building kernel through provider compatibility bundle"
        );
        Kernel::from_parts(
            config,
            self.providers,
            SchedulerFactory::build(self.scheduler_kind),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LegacyLlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    #[tokio::test]
    async fn builder_matches_kernel_new_empty_registry() {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LegacyLlmProvider> = Arc::new(MockLlm);
        let kernel = KernelBuilder::from_compat(
            config,
            KernelProviderCompat::new(llm, Box::new(DefaultToolSet::new())),
        )
        .build();
        assert_eq!(kernel.agent_count().await, 0);
    }

    #[tokio::test]
    async fn deprecated_builder_constructor_remains_callable() {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LegacyLlmProvider> = Arc::new(MockLlm);
        #[allow(deprecated)]
        let kernel = KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build();
        assert_eq!(kernel.agent_count().await, 0);
    }
}
