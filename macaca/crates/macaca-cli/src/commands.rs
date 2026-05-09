//! Command implementations for the Agent OS CLI.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use macaca_agent::LlmProvider;
use macaca_app::AppRuntime;
use macaca_gateway::GatewayBuilder;
use macaca_kernel::{Kernel, KernelBuilder, KernelServiceClientCompat};
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::error::MacacaResult;
use macaca_proto::types::{LlmMessage, LlmOptions, LlmResponse, TokenUsage};
use macaca_sdk::{
    kernel_status_snapshot, ServiceInspectionCommand, StaticSystemStatusDataSource, SystemFacade,
    SystemTaskClient, TaskBoardQueryCommand, TaskBoardQueryResult,
};
use macaca_tools::{DefaultToolSet, ToolCatalog};

/// Compatibility LLM provider used only to satisfy legacy kernel construction.
///
/// Route C S5 moves production LLM calls behind SDK/SystemFacade service
/// clients.  Some CLI commands still need to instantiate the legacy kernel
/// compatibility bundle for read-only status and registry inspection.  This
/// provider makes that compatibility path explicit and fails chat calls with a
/// structured unavailable response instead of pretending to be a real model.
struct CliUnavailableLlmProvider;

#[async_trait]
impl LlmProvider for CliUnavailableLlmProvider {
    fn name(&self) -> &str {
        "cli-unavailable-service-compat"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "(LLM service unavailable through CLI compatibility provider)".into(),
            reasoning_content: None,
            model: "cli-unavailable-service-compat".into(),
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
pub async fn execute_run_kernel() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let app_runtime = AppRuntime::default();

    info!(
        max_agents = config.kernel.max_agents,
        "Initializing Agent OS kernel"
    );

    let llm: Arc<dyn LlmProvider> = Arc::new(CliUnavailableLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = build_kernel(config.kernel.clone(), llm, tools);

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

/// Initialize and run the kernel with optional gateway adapters.
#[deprecated(note = "Use RunCommandHandler through CliCommandHandler dispatch instead")]
pub async fn run_kernel() -> MacacaResult<()> {
    execute_run_kernel().await
}

/// List all agents currently registered with the kernel.
pub async fn execute_list_agents() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let llm: Arc<dyn LlmProvider> = Arc::new(CliUnavailableLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = build_kernel(config.kernel.clone(), llm, tools);

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

/// List all agents currently registered with the kernel.
#[deprecated(note = "Use AgentsCommandHandler through CliCommandHandler dispatch instead")]
pub async fn list_agents() -> MacacaResult<()> {
    execute_list_agents().await
}

/// Display system status information.
pub async fn execute_show_status() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let app_runtime = AppRuntime::default();
    let llm: Arc<dyn LlmProvider> = Arc::new(CliUnavailableLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    let kernel = build_kernel(config.kernel.clone(), llm, tools);

    let snapshot = kernel_status_snapshot(
        &kernel,
        app_runtime.app_count().await,
        config.kernel.max_agents,
        config.llm.default_provider.clone(),
        config.gateway.enabled,
    )
    .await;
    let facade = SystemFacade::new(
        EmptyCliTaskBoardDataSource,
        StaticSystemStatusDataSource::new(snapshot),
    );
    let snapshot = facade.status_snapshot().await?;

    println!("Agent OS Status");
    println!("{}", "=".repeat(40));
    println!("Version:         {}", snapshot.version);
    println!("Agents:          {}", snapshot.agent_count);
    println!("Loaded apps:     {}", snapshot.loaded_apps);
    println!("Max agents:      {}", snapshot.max_agents);
    println!("LLM provider:    {}", snapshot.llm_provider);
    println!("App runtime:     {}", snapshot.app_runtime);
    println!("Gateway enabled: {}", snapshot.gateway_enabled);
    let service_inventory = facade
        .inspect_services(ServiceInspectionCommand::new("cli-status")?)
        .await?;
    let service_status = if service_inventory.services.is_empty() {
        "unavailable"
    } else {
        "available"
    };
    println!(
        "Services:        {service_status} ({})",
        service_inventory.services.join(", ")
    );
    println!("  LLM Service:    {service_status}");
    println!("  Memory Service: {service_status}");
    println!("  Context Service:{service_status}");

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

/// CLI-only empty task-board adapter used when a command only needs status.
///
/// Keeping this as an explicit adapter avoids making the CLI depend on Web
/// state while satisfying the typed SDK facade shape. Commands that need real
/// task-board data should provide a concrete store-backed adapter instead.
#[derive(Debug, Default)]
struct EmptyCliTaskBoardDataSource;

#[async_trait]
impl SystemTaskClient for EmptyCliTaskBoardDataSource {
    async fn query_task_board(
        &self,
        _command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        Ok(TaskBoardQueryResult {
            todos: Vec::new(),
            count: 0,
        })
    }
}

/// Create a kernel instance from config (for testing and composition).
pub fn create_kernel_with_stub_provider(config: &KernelConfig) -> Kernel {
    let llm: Arc<dyn LlmProvider> = Arc::new(CliUnavailableLlmProvider);
    let tools = Box::new(DefaultToolSet::new());
    build_kernel(config.clone(), llm, tools)
}

/// Display system status information.
#[deprecated(note = "Use StatusCommandHandler through CliCommandHandler dispatch instead")]
pub async fn show_status() -> MacacaResult<()> {
    execute_show_status().await
}

/// Create a kernel instance from config (for testing and composition).
#[deprecated(note = "Use create_kernel_with_stub_provider instead")]
pub fn create_kernel(config: &KernelConfig) -> Kernel {
    create_kernel_with_stub_provider(config)
}

fn build_kernel(
    config: KernelConfig,
    llm: Arc<dyn LlmProvider>,
    tools: Box<dyn ToolCatalog>,
) -> Kernel {
    tracing::debug!(
        llm_provider = %llm.name(),
        "CLI kernel builder ignores legacy provider and uses service-client compatibility seam"
    );
    KernelBuilder::from_service_clients(config, KernelServiceClientCompat::from_boxed_tools(tools))
        .build()
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
        let _kernel = create_kernel_with_stub_provider(&config);
    }

    #[tokio::test]
    async fn list_agents_empty() {
        // Should succeed and print "No agents registered."
        execute_list_agents().await.unwrap();
    }

    #[tokio::test]
    async fn show_status_succeeds() {
        execute_show_status().await.unwrap();
    }

    #[tokio::test]
    async fn kernel_starts_with_unavailable_llm_compat_provider() {
        let config = test_kernel_config();
        let kernel = create_kernel_with_stub_provider(&config);
        assert_eq!(kernel.agent_count().await, 0);
    }

    #[test]
    fn unavailable_llm_compat_provider_name() {
        let provider = CliUnavailableLlmProvider;
        assert_eq!(provider.name(), "cli-unavailable-service-compat");
    }
}
