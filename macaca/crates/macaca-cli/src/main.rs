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
    if let Err(e) = macaca_cli::logging::init_logging(&config.observability.log_file, &config.observability.log_level) {
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
        Commands::Web { port } => {
            #[cfg(feature = "systemd")]
            {
                // Notify systemd we're ready
                let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);

                // Spawn watchdog heartbeat task
                tokio::spawn(async {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(10));
                    loop {
                        interval.tick().await;
                        let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
                    }
                });
            }
            macaca_web::start_server(port).await
        }
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Command failed");
        std::process::exit(1);
    }
}
