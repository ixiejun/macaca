//! Agent OS CLI entry point.

#![deny(deprecated)]

use clap::{Parser, Subcommand};
use macaca_cli::command_handlers::{
    AgentsCommandHandler, CliCommandHandler, PluginListCommandHandler, RunCommandHandler,
    StatusCommandHandler, VersionCommandHandler, WebCommandHandler,
};
use macaca_proto::config::MacacaConfig;

/// Agent OS — autonomous agent orchestration platform.
#[derive(Parser)]
#[command(name = "macaca", version, about = "Macaca Agent OS CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Start the Agent OS kernel and gateway adapters.
    Run,
    /// List all registered agents.
    Agents,
    /// Show system status.
    Status,
    /// Plugin Control management commands.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Print version information.
    Version,
    /// Start the web UI server.
    Web {
        /// Port to listen on.
        #[arg(long, default_value_t = 3001)]
        port: u16,
    },
}

/// Plugin management subcommands.
#[derive(Subcommand)]
enum PluginCommands {
    /// List plugins through Plugin Control Service.
    List,
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = MacacaConfig::load_default();

    // Initialize logging with file output
    if let Err(e) = macaca_cli::logging::init_logging(
        &config.observability.log_file,
        &config.observability.log_level,
    ) {
        eprintln!("Failed to initialize logging: {e}");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run => RunCommandHandler.run().await,
        Commands::Agents => AgentsCommandHandler.run().await,
        Commands::Status => StatusCommandHandler.run().await,
        Commands::Plugin { command } => match command {
            PluginCommands::List => PluginListCommandHandler.run().await,
        },
        Commands::Version => {
            VersionCommandHandler::new(env!("CARGO_PKG_VERSION"))
                .run()
                .await
        }
        Commands::Web { port } => WebCommandHandler::new(port).run().await,
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Command failed");
        std::process::exit(1);
    }
}
