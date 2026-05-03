//! Command implementations for the Agent OS CLI.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use macaca_app::AppRuntime;
use macaca_gateway::GatewayBuilder;
use macaca_kernel::Kernel;
use macaca_llm::LlmProvider;
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::error::MacacaResult;
use macaca_proto::types::{LlmMessage, LlmOptions, LlmResponse, TokenUsage};
use macaca_tools::DefaultToolSet;

/// A no-op LLM provider used when no real provider is configured.
///
/// Returns a canned response for every request. This allows the CLI
/// to boot the kernel without requiring API keys.
struct StubLlmProvider;

#[async_trait]
impl LlmProvider for StubLlmProvider {
    fn name(&self) -> &str {
        "stub"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "(stub LLM — no provider configured)".into(),
            reasoning_content: None,
            model: "stub".into(),
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

/// Initialize and run the kernel with optional gateway adapters.
///
/// Loads configuration from `config/default.toml` (with env overrides),
/// creates a kernel with the stub LLM, and optionally starts gateway
/// adapters if configured.
pub async fn run_kernel() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let app_runtime = AppRuntime::default();

    info!(
        max_agents = config.kernel.max_agents,
        "Initializing Agent OS kernel"
    );

    let llm: Arc<dyn LlmProvider> = Arc::new(StubLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = Kernel::new(&config.kernel, llm, tools);

    info!(
        agents = kernel.agent_count().await,
        loaded_apps = app_runtime.app_count().await,
        "Kernel started successfully"
    );

    // Set up gateway if enabled
    if config.gateway.enabled {
        let gateway = GatewayBuilder::new(config.gateway.clone()).start().await?;
        info!(
            adapters = gateway.adapter_count(),
            "Gateway adapters running"
        );
    }

    println!("Agent OS is running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| macaca_proto::error::MacacaError::Io(e))?;

    info!("Shutting down Agent OS");
    Ok(())
}

/// List all agents currently registered with the kernel.
pub async fn list_agents() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let llm: Arc<dyn LlmProvider> = Arc::new(StubLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = Kernel::new(&config.kernel, llm, tools);

    let agents = kernel.list_agents().await;
    if agents.is_empty() {
        println!("No agents registered.");
    } else {
        println!("{:<38} {:<20} {:<12}", "ID", "NAME", "STATE");
        println!("{}", "-".repeat(70));
        for manifest in &agents {
            println!(
                "{:<38} {:<20} {:?}",
                manifest.id.0, manifest.name, manifest.state
            );
        }
    }
    println!("\nTotal: {} agent(s)", agents.len());
    Ok(())
}

/// Display system status information.
pub async fn show_status() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let app_runtime = AppRuntime::default();
    let llm: Arc<dyn LlmProvider> = Arc::new(StubLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = Kernel::new(&config.kernel, llm, tools);

    let agent_count = kernel.agent_count().await;

    println!("Agent OS Status");
    println!("{}", "=".repeat(40));
    println!("Version:         {}", env!("CARGO_PKG_VERSION"));
    println!("Agents:          {}", agent_count);
    println!("Loaded apps:     {}", app_runtime.app_count().await);
    println!("Max agents:      {}", config.kernel.max_agents);
    println!("LLM provider:    {}", config.llm.default_provider);
    println!("App runtime:     macaca-app/AppRuntime");
    println!("Gateway enabled: {}", config.gateway.enabled);

    if config.gateway.enabled {
        if let Some(ref tg) = config.gateway.telegram {
            println!(
                "  Telegram:      {}",
                if tg.enabled { "enabled" } else { "disabled" }
            );
        }
        if let Some(ref dc) = config.gateway.discord {
            println!(
                "  Discord:       {}",
                if dc.enabled { "enabled" } else { "disabled" }
            );
        }
    }

    Ok(())
}

/// Create a kernel instance from config (for testing and composition).
pub fn create_kernel(config: &KernelConfig) -> Kernel {
    let llm: Arc<dyn LlmProvider> = Arc::new(StubLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    Kernel::new(config, llm, tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::config::KernelConfig;

    fn test_kernel_config() -> KernelConfig {
        KernelConfig {
            max_agents: 8,
            heartbeat_interval_ms: 1000,
            agent_timeout_ms: 5000,
        }
    }

    #[test]
    fn create_kernel_returns_valid_kernel() {
        let config = test_kernel_config();
        let _kernel = create_kernel(&config);
    }

    #[tokio::test]
    async fn list_agents_empty() {
        // Should succeed and print "No agents registered."
        list_agents().await.unwrap();
    }

    #[tokio::test]
    async fn show_status_succeeds() {
        show_status().await.unwrap();
    }

    #[tokio::test]
    async fn kernel_starts_with_stub_llm() {
        let config = test_kernel_config();
        let kernel = create_kernel(&config);
        assert_eq!(kernel.agent_count().await, 0);
    }

    #[test]
    fn stub_llm_provider_name() {
        let provider = StubLlmProvider;
        assert_eq!(provider.name(), "stub");
    }
}
