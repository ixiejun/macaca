//! Command implementations for the Agent OS CLI.

use async_trait::async_trait;
use tracing::{info, warn};

use macaca_proto::config::MacacaConfig;
use macaca_proto::error::MacacaResult;
use macaca_sdk::{
    ServiceInspectionCommand, StaticSystemStatusDataSource, SystemFacade,
    SystemPluginControlClient, SystemStatusSnapshot, SystemTaskClient, TaskBoardQueryCommand,
    TaskBoardQueryResult, UnavailableSystemPluginControlClient,
};

/// Initialize and run the kernel with optional gateway adapters.
///
/// Loads configuration from `config/default.toml` (with env overrides),
/// creates a kernel with the stub LLM, and optionally starts gateway
/// adapters if configured.
pub async fn execute_run_kernel() -> MacacaResult<()> {
    let config = MacacaConfig::load_default();
    let facade = cli_system_facade(&config);
    let snapshot = facade.status_snapshot().await?;
    let service_inventory = facade
        .inspect_services(ServiceInspectionCommand::new("cli-run")?)
        .await?;

    info!(
        max_agents = snapshot.max_agents,
        services = service_inventory.services.len(),
        "CLI run command attached to SDK service inspection boundary"
    );
    warn!(
        "CLI run command no longer constructs kernel, gateway, or tool providers; start concrete runtimes through service hosts or `macaca web`"
    );

    println!("Agent OS is running. Press Ctrl+C to stop.");

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
    let facade = cli_system_facade(&config);
    let snapshot = facade.status_snapshot().await?;
    println!("No agents reported by CLI service inspection boundary.");
    println!("\nTotal: {} agent(s)", snapshot.agent_count);
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
    let facade = cli_system_facade(&config);
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

/// List plugins through the SDK Plugin Control boundary.
///
/// This command intentionally uses the SDK Null Object client until a concrete
/// service-backed dispatcher is injected into CLI.  The shape prevents CLI from
/// growing repository or runtime-host semantics while giving operators a stable
/// command surface that later hosts can wire to real Plugin Control Service.
pub async fn execute_list_plugins() -> MacacaResult<()> {
    let client = UnavailableSystemPluginControlClient;
    let records = client
        .list(macaca_proto::PluginListCommand {
            repository_id: None,
            include_disabled: true,
            trace: macaca_proto::TraceContext::new("cli-plugin-list"),
        })
        .await?;

    if records.is_empty() {
        println!("No plugins reported by Plugin Control Service.");
    } else {
        println!("{:<40} {:<12} {:<12}", "PLUGIN", "VERSION", "STATE");
        println!("{}", "-".repeat(72));
        for record in records {
            println!(
                "{:<40} {:<12} {:?}",
                record.plugin_id, record.version, record.activation_state
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

/// Display system status information.
#[deprecated(note = "Use StatusCommandHandler through CliCommandHandler dispatch instead")]
pub async fn show_status() -> MacacaResult<()> {
    execute_show_status().await
}

fn cli_system_facade(
    config: &MacacaConfig,
) -> SystemFacade<EmptyCliTaskBoardDataSource, StaticSystemStatusDataSource> {
    SystemFacade::new(
        EmptyCliTaskBoardDataSource,
        StaticSystemStatusDataSource::new(SystemStatusSnapshot {
            version: env!("CARGO_PKG_VERSION").into(),
            agent_count: 0,
            loaded_apps: 0,
            max_agents: config.kernel.max_agents,
            llm_provider: config.llm.default_provider.clone(),
            app_runtime: "application-service-client".into(),
            gateway_enabled: config.gateway.enabled,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_agents_empty() {
        execute_list_agents().await.unwrap();
    }

    #[tokio::test]
    async fn show_status_succeeds() {
        execute_show_status().await.unwrap();
    }

    #[tokio::test]
    async fn cli_status_snapshot_is_service_boundary_only() {
        let config = MacacaConfig::load_default();
        let facade = cli_system_facade(&config);
        let snapshot = facade.status_snapshot().await.unwrap();
        assert_eq!(snapshot.agent_count, 0);
        assert_eq!(snapshot.app_runtime, "application-service-client");
    }
}
