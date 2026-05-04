//! Additive kernel construction facade.

use std::sync::Arc;

use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_tools::ToolCatalog;

use crate::{Kernel, SchedulerFactory, SchedulerKind};

/// Builder for constructing a [`Kernel`] from explicit runtime dependencies.
pub struct KernelBuilder {
    config: KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn ToolCatalog>,
    scheduler_kind: SchedulerKind,
}

impl KernelBuilder {
    /// Create a builder with default scheduler behavior.
    pub fn new(
        config: KernelConfig,
        llm: Arc<dyn LlmProvider>,
        tools: Box<dyn ToolCatalog>,
    ) -> Self {
        Self {
            config,
            llm,
            tools,
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
        Kernel::from_parts(
            self.config,
            self.llm,
            self.tools,
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
    async fn builder_matches_kernel_new_empty_registry() {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let kernel = KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build();
        assert_eq!(kernel.agent_count().await, 0);
    }
}
