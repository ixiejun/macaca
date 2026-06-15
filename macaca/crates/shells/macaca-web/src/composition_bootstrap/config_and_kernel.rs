//! Bootstrap phase 1–3: configuration, MCP process env, LLM router, kernel seed.
//!
//! Web acts as the composition root: it loads operator config and seeds the kernel with an
//! explicit unavailable execution port that is hot-swapped once `service.agent_execution` registers.

use std::sync::Arc;

use tracing::info;

use macaca_host_composition::kernel::{KernelBuilder, UnavailableAgentExecutionPort};
use macaca_host_composition::llm::{LlmProvider, LlmRouter};
use macaca_proto::config::{KernelConfig, MacacaConfig};
use macaca_proto::MacacaResult;

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `config-and-kernel` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    // 1. Load configuration from config/default.toml
    let config = MacacaConfig::load_default();
    info!(
        service_id = "bootstrap.config",
        command = "load_default",
        config_scope = "llm",
        "Configuration loaded"
    );

    // 1b. Publish [mcp.env] entries into the current process environment so
    //     every stdio MCP child process (which inherits parent env by default)
    //     automatically receives secrets such as FIGMA_API_KEY.
    let mcp_env_outcomes =
        macaca_host_composition::mcp_runtime::RuntimeEnvBuilder::apply_process_env(&config.mcp.env);
    if !mcp_env_outcomes.is_empty() {
        info!(
            entries = mcp_env_outcomes.len(),
            "Applied [mcp.env] to process environment for MCP child inheritance"
        );
    }

    // 2. Create LLM router/provider registry from configuration.
    let llm_router = Arc::new(LlmRouter::from_config(&config.llm)?);
    let llm: Arc<dyn LlmProvider> = llm_router.clone();

    info!(provider = llm.name(), "LLM provider initialized");

    // 3. Create kernel.
    let kernel_config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 60000,
    };
    // Seed the kernel with an explicit unavailable execution port. The port is
    // hot-swapped to `service.agent_execution` after providers register below.
    let kernel = Arc::new(
        KernelBuilder::from_execution_port(
            kernel_config,
            Arc::new(UnavailableAgentExecutionPort::new(
                "bootstrap placeholder until service.agent_execution hot-swap",
            )),
        )
        .build(),
    );

    ctx.config = Some(config);
    ctx.llm_router = Some(llm_router);
    ctx.llm = Some(llm);
    ctx.kernel = Some(kernel);
    Ok(())
}
