//! Agent OS CLI entry point.

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Agent OS — autonomous agent orchestration platform.
#[derive(Parser)]
#[command(name = "aos", version, about = "Agent OS CLI")]
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run => macaca_cli::run_kernel().await,
        Commands::Agents => macaca_cli::list_agents().await,
        Commands::Status => macaca_cli::show_status().await,
        Commands::Version => {
            println!("Agent OS v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Web { port } => macaca_web::start_server(port).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
