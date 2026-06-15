//! Standalone process entrypoint for the Macaca Web shell.
//!
//! The CLI starts this binary as a separate process so `macaca-cli` does not
//! depend on `macaca-web` internals while preserving the operator-facing
//! `macaca web --port <port>` workflow.

use macaca_proto::{MacacaError, MacacaResult};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_process_logging();
    if let Err(err) = run().await {
        tracing::error!(error = %err, "macaca web server process failed");
        eprintln!("{err}");
        std::process::exit(1);
    }
}

/// Initialize standalone Web-process logging before runtime bootstrap.
///
/// The CLI launches this binary as a separate process. Because tracing
/// subscribers are process-local, the CLI subscriber cannot observe Web,
/// Runtime Host, Heartbeat, or Agent Execution logs emitted here. This small
/// entrypoint adapter keeps the logging concern at the process boundary and
/// preserves all service/application semantics unchanged. `try_init` is used so
/// tests or embedded launchers that install their own subscriber remain safe.
fn init_process_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

async fn run() -> MacacaResult<()> {
    let port = parse_port(std::env::args().skip(1))?;
    macaca_host_composition::WebServerBuilder::new()
        .port(port)
        .serve()
        .await
}

fn parse_port<I>(mut args: I) -> MacacaResult<u16>
where
    I: Iterator<Item = String>,
{
    let mut port = 3001;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                let Some(value) = args.next() else {
                    return Err(MacacaError::Config(
                        "missing value for --port in macaca-web-server".into(),
                    ));
                };
                port = value.parse::<u16>().map_err(|err| {
                    MacacaError::Config(format!(
                        "invalid --port value `{value}` for macaca-web-server: {err}"
                    ))
                })?;
            }
            "--help" | "-h" => {
                println!("Usage: macaca-web-server [--port <port>]");
                std::process::exit(0);
            }
            other => {
                return Err(MacacaError::Config(format!(
                    "unknown macaca-web-server argument `{other}`"
                )));
            }
        }
    }
    Ok(port)
}
