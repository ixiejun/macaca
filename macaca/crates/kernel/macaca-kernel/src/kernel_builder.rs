//! Additive kernel construction facade.

use std::sync::Arc;

use macaca_agent::{
    AgentExecutionPort, LegacyAgentExecutionAdapter, UnavailableAgentExecutionPort,
};
use macaca_proto::config::KernelConfig;

use crate::{
    Kernel, KernelProviderCompat, LegacyLlmProvider, LegacyToolCatalog, SchedulerFactory,
    SchedulerKind,
};

/// Builder for constructing a [`Kernel`] from explicit runtime dependencies.
pub struct KernelBuilder {
    config: KernelConfig,
    execution_port: Arc<dyn AgentExecutionPort>,
    scheduler_kind: SchedulerKind,
}

/// Provider-neutral kernel construction bundle for service-client-era callers.
///
/// The kernel execution path still accepts the legacy `Agent::run` provider
/// shape, so this bundle uses an internal unavailable adapter until agent
/// execution itself is serviceized. New callers can depend on this struct
/// instead of constructing `KernelProviderCompat` directly.
pub struct KernelServiceClientCompat {
    execution_port: Arc<dyn AgentExecutionPort>,
}

impl KernelServiceClientCompat {
    /// Create a bundle from shared tool catalog handles.
    pub fn new(_tools: Arc<dyn LegacyToolCatalog>) -> Self {
        Self::unavailable("service-client construction has no legacy execution bridge")
    }

    /// Create a bundle from boxed tool catalog handles.
    pub fn from_boxed_tools(_tools: Box<dyn LegacyToolCatalog>) -> Self {
        Self::unavailable("service-client construction has no legacy execution bridge")
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
        Self {
            execution_port: Arc::new(LegacyAgentExecutionAdapter::new(llm, tools)),
        }
    }

    /// Create a bundle with an agent-level LLM bridge and boxed tool catalog.
    pub fn from_agent_provider_boxed_tools(
        llm: Arc<dyn LegacyLlmProvider>,
        tools: Box<dyn LegacyToolCatalog>,
    ) -> Self {
        Self::from_agent_provider(llm, Arc::from(tools))
    }

    /// Create a bundle from an already provider-neutral execution port.
    ///
    /// This is the preferred service-era entry point. Runtime-host or service
    /// clients can provide their own command-dispatch implementation while the
    /// kernel builder remains unchanged.
    pub fn from_execution_port(execution_port: Arc<dyn AgentExecutionPort>) -> Self {
        tracing::info!("kernel service-client compatibility created from execution port");
        Self { execution_port }
    }

    /// Create a bundle that fails explicitly until an execution bridge exists.
    fn unavailable(reason: &'static str) -> Self {
        Self {
            execution_port: Arc::new(UnavailableAgentExecutionPort::new(reason)),
        }
    }

    /// Consume the bundle and return the execution port used by the kernel.
    fn into_execution_port(self) -> Arc<dyn AgentExecutionPort> {
        self.execution_port
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
            execution_port: providers.into_execution_port(),
            scheduler_kind: SchedulerKind::default(),
        }
    }

    /// Create a builder from the provider-neutral service-client compatibility bundle.
    ///
    /// This is the preferred S5 construction seam for new code because it does
    /// not require callers to import or instantiate `LegacyLlmProvider`. The
    /// old provider bundle stays available as a deprecated migration memento.
    pub fn from_service_clients(config: KernelConfig, compat: KernelServiceClientCompat) -> Self {
        Self {
            config,
            execution_port: compat.into_execution_port(),
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
            "building kernel through execution port"
        );
        Kernel::from_parts(
            config,
            self.execution_port,
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
