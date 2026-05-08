//! SDK facade and registry adapters.

use async_trait::async_trait;

use macaca_kernel::{DefaultKernelFacade, Kernel, KernelFacade};
use macaca_proto::{AgentId, AgentManifest, MacacaResult};

use crate::builder::DeclarativeAgent;
use crate::config::AgentConfig;
use crate::spec::AgentSpec;

/// Registry boundary used by the SDK facade.
#[async_trait]
pub trait AgentRegistryApi: Send + Sync {
    /// Register a runtime agent and manifest.
    async fn register_agent(
        &self,
        agent: DeclarativeAgent,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId>;
}

/// Kernel-backed registry adapter.
pub struct KernelAgentRegistry<'a> {
    kernel: &'a Kernel,
}

impl<'a> KernelAgentRegistry<'a> {
    /// Create a kernel registry adapter.
    pub fn new(kernel: &'a Kernel) -> Self {
        Self { kernel }
    }
}

#[async_trait]
impl AgentRegistryApi for KernelAgentRegistry<'_> {
    async fn register_agent(
        &self,
        agent: DeclarativeAgent,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId> {
        self.kernel.register_agent(Box::new(agent), manifest).await
    }
}

/// Facade for SDK agent declaration and registration.
pub struct MacacaSdk<R> {
    registry: R,
}

impl<R> MacacaSdk<R>
where
    R: AgentRegistryApi,
{
    /// Create a facade over a registry adapter.
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Register an already-built agent spec.
    pub async fn register_spec(&self, spec: AgentSpec) -> MacacaResult<AgentId> {
        let manifest = spec.manifest();
        let agent = spec.into_agent();
        self.registry.register_agent(agent, manifest).await
    }

    /// Build and register an agent from config.
    pub async fn register_config(&self, config: AgentConfig) -> MacacaResult<AgentId> {
        self.register_spec(AgentSpec::from_config(config)?).await
    }
}

impl<'a> MacacaSdk<KernelAgentRegistry<'a>> {
    /// Create a facade backed by a kernel registry.
    pub fn for_kernel(kernel: &'a Kernel) -> Self {
        Self::new(KernelAgentRegistry::new(kernel))
    }
}

/// SDK-facing access point for microkernel primitive discovery.
///
/// This wrapper is intentionally small: it gives applications and future
/// tooling a stable place to receive a `KernelFacade` without importing
/// `macaca-web` or provider-specific crates.  Runtime behavior stays unchanged
/// in Phase 01; later phases can inject a service-backed facade here.
pub struct KernelPrimitiveSdk<F = DefaultKernelFacade> {
    facade: F,
}

impl<F> KernelPrimitiveSdk<F>
where
    F: KernelFacade,
{
    /// Create SDK primitive access around a facade implementation.
    pub fn new(facade: F) -> Self {
        Self { facade }
    }

    /// Borrow the underlying microkernel facade.
    ///
    /// Keeping this as a borrow preserves ownership for callers that need to
    /// share one facade across multiple application or tooling components.
    pub fn facade(&self) -> &F {
        &self.facade
    }

    /// Consume the SDK wrapper and return the underlying facade.
    pub fn into_inner(self) -> F {
        self.facade
    }
}

impl Default for KernelPrimitiveSdk<DefaultKernelFacade> {
    fn default() -> Self {
        Self::new(DefaultKernelFacade::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use macaca_kernel::{CapabilityRegistry, KernelBuilder, KernelProviderCompat};
    use macaca_llm::LlmProvider;
    use macaca_proto::config::KernelConfig;
    use macaca_proto::{
        CapabilityDescriptor, CapabilityId, KernelServiceId, LlmMessage, LlmOptions, LlmResponse,
        ServiceScope, TokenUsage,
    };
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
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    fn make_kernel() -> Kernel {
        let config = KernelConfig {
            max_agents: 16,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        KernelBuilder::from_compat(
            config,
            KernelProviderCompat::new(llm, Box::new(DefaultToolSet::new())),
        )
        .build()
    }

    #[tokio::test]
    async fn facade_registers_config_with_kernel() {
        let kernel = make_kernel();
        let sdk = MacacaSdk::for_kernel(&kernel);
        let config = AgentConfig::from_yaml(
            r#"
name: facade-agent
capabilities:
  - name: test
prompt_template: "Hello"
"#,
        )
        .unwrap();

        let id = sdk.register_config(config).await.unwrap();
        let agents = kernel.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, id);
        assert_eq!(agents[0].name, "facade-agent");
    }

    #[test]
    fn primitive_sdk_exposes_kernel_facade_without_web_dependency() {
        let sdk = KernelPrimitiveSdk::default();
        let descriptor = CapabilityDescriptor::new(
            CapabilityId::new("capability.example"),
            KernelServiceId::new("service.example"),
            ServiceScope::Global,
            "Example descriptor for SDK primitive access.",
        );

        sdk.facade().register_capability(descriptor).unwrap();

        let found = sdk
            .facade()
            .get_capability(&CapabilityId::new("capability.example"))
            .unwrap();
        assert!(found.is_some());
    }
}
