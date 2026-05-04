//! Command handler primitives for CLI subcommand dispatch.

use async_trait::async_trait;
use macaca_proto::error::MacacaResult;

use crate::commands;

/// Canonical command execution boundary for CLI subcommands.
#[async_trait(?Send)]
pub trait CliCommandHandler {
    /// Execute the selected CLI command.
    async fn run(&self) -> MacacaResult<()>;
}

/// Handler for the `run` command.
#[derive(Debug, Default)]
pub struct RunCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for RunCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_run_kernel().await
    }
}

/// Handler for the `agents` command.
#[derive(Debug, Default)]
pub struct AgentsCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for AgentsCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_list_agents().await
    }
}

/// Handler for the `status` command.
#[derive(Debug, Default)]
pub struct StatusCommandHandler;

#[async_trait(?Send)]
impl CliCommandHandler for StatusCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        commands::execute_show_status().await
    }
}

/// Handler for the `version` command.
#[derive(Debug)]
pub struct VersionCommandHandler {
    version: &'static str,
}

impl VersionCommandHandler {
    /// Build a version command handler.
    pub fn new(version: &'static str) -> Self {
        Self { version }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for VersionCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        println!("Macaca Agent OS v{}", self.version);
        Ok(())
    }
}

/// Handler for the `web` command.
#[derive(Debug)]
pub struct WebCommandHandler {
    port: u16,
}

impl WebCommandHandler {
    /// Build a web command handler for the given port.
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

#[async_trait(?Send)]
impl CliCommandHandler for WebCommandHandler {
    async fn run(&self) -> MacacaResult<()> {
        notify_systemd_ready();
        macaca_web::WebServerBuilder::new()
            .port(self.port)
            .serve()
            .await
    }
}

#[cfg(feature = "systemd")]
fn notify_systemd_ready() {
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);

    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
        }
    });
}

#[cfg(not(feature = "systemd"))]
fn notify_systemd_ready() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn agents_handler_runs() {
        AgentsCommandHandler.run().await.unwrap();
    }

    #[tokio::test]
    async fn status_handler_runs() {
        StatusCommandHandler.run().await.unwrap();
    }

    #[tokio::test]
    async fn version_handler_runs() {
        VersionCommandHandler::new("test-version")
            .run()
            .await
            .unwrap();
    }
}
