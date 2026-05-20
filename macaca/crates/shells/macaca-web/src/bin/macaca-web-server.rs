//! Standalone process entrypoint for the Macaca Web shell.
//!
//! The CLI starts this binary as a separate process so `macaca-cli` does not
//! depend on `macaca-web` internals while preserving the operator-facing
//! `macaca web --port <port>` workflow.

use macaca_proto::{MacacaError, MacacaResult};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        tracing::error!(error = %err, "macaca web server process failed");
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> MacacaResult<()> {
    let port = parse_port(std::env::args().skip(1))?;
    macaca_web::WebServerBuilder::new().port(port).serve().await
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
