//! Agent OS CLI entry point.

use clap::{Parser, Subcommand};
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
    /// Print version information.
    Version,
    /// Start the web UI server.
    Web {
        /// Port to listen on.
        #[arg(long, default_value_t = 3001)]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = MacacaConfig::load_default();

    // Initialize logging with file output
    if let Err(e) = macaca_cli::logging::init_logging(&config.observability.log_file) {
        eprintln!("Failed to initialize logging: {e}");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run => macaca_cli::run_kernel().await,
        Commands::Agents => macaca_cli::list_agents().await,
        Commands::Status => macaca_cli::show_status().await,
        Commands::Version => {
            println!("Macaca Agent OS v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Web { port } => macaca_web::start_server(port).await,
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Command failed");
        std::process::exit(1);
    }
}
