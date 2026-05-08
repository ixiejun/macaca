//! Additive kernel construction facade.

use std::sync::Arc;

use macaca_proto::config::KernelConfig;

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
