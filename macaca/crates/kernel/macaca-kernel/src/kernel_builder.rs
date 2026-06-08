//! Kernel construction facade (Builder pattern).
//!
//! The microkernel accepts only a provider-neutral [`AgentExecutionPort`]. Composition
//! roots (web shell, runtime-host, SDK fixtures) supply the port implementation while
//! the kernel keeps orchestration, registry, and scheduling invariants stable.

use std::sync::Arc;

use macaca_proto::config::KernelConfig;
use macaca_proto::AgentExecutionPort;

use crate::execution_port::SwappableAgentExecutionPort;
use crate::{Kernel, SchedulerFactory, SchedulerKind};

/// Builder for constructing a [`Kernel`] from explicit runtime dependencies.
///
/// This type is the sole supported construction entry for new kernel wiring. Callers
/// inject an [`AgentExecutionPort`] produced by runtime-host service clients, legacy
/// test adapters, or bootstrap placeholders that are hot-swapped after services start.
pub struct KernelBuilder {
    config: KernelConfig,
    execution_port: Arc<dyn AgentExecutionPort>,
    scheduler_kind: SchedulerKind,
}

impl KernelBuilder {
    /// Create a builder bound to one swappable execution port.
    ///
    /// The port may be a service-client adapter in production hosts, a legacy adapter in
    /// integration fixtures, or an unavailable placeholder that is replaced once
    /// `service.agent_execution` is registered.
    pub fn from_execution_port(
        config: KernelConfig,
        execution_port: Arc<dyn AgentExecutionPort>,
    ) -> Self {
        tracing::info!(
            max_agents = config.max_agents,
            "kernel builder created from execution port"
        );
        Self {
            config,
            execution_port,
            scheduler_kind: SchedulerKind::default(),
        }
    }

    /// Override the scheduler strategy before materializing the kernel.
    pub fn scheduler_kind(mut self, scheduler_kind: SchedulerKind) -> Self {
        tracing::debug!(?scheduler_kind, "kernel builder scheduler strategy overridden");
        self.scheduler_kind = scheduler_kind;
        self
    }

    /// Materialize the kernel with a swappable execution port wrapper.
    ///
    /// The wrapper preserves stable port identity for bootstrap hot-swap while allowing
    /// composition roots to replace the active implementation after services start.
    pub fn build(self) -> Kernel {
        let config = self.config;
        tracing::info!(
            max_agents = config.max_agents,
            scheduler_kind = ?self.scheduler_kind,
            "building kernel through swappable execution port"
        );
        let swappable = SwappableAgentExecutionPort::new(self.execution_port);
        Kernel::from_parts(
            config,
            swappable,
            SchedulerFactory::build(self.scheduler_kind),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_agent::{InProcessAgentExecutionPort, LlmProvider, ToolCatalog};
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
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
    async fn builder_from_execution_port_produces_empty_registry() {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let execution_port: Arc<dyn AgentExecutionPort> = Arc::new(InProcessAgentExecutionPort::new(
            llm,
            Arc::from(Box::new(DefaultToolSet::new()) as Box<dyn ToolCatalog>),
        ));
        let kernel = KernelBuilder::from_execution_port(config, execution_port).build();
        assert_eq!(kernel.agent_count().await, 0);
    }
}
