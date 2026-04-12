//! `ShellDriver` — wraps `ShellTool` as a `SoftwareDriver`.
//!
//! Provides CLI subprocess control as a pluggable driver.

use async_trait::async_trait;
use std::time::Duration;

use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::builtin::ShellTool;
use macaca_tools::Tool;

use crate::driver::{DriverManifest, DriverType, SoftwareDriver};

/// A built-in driver that exposes shell command execution.
pub struct ShellDriver {
    manifest: DriverManifest,
    default_timeout: Duration,
}

impl ShellDriver {
    /// Create a new ShellDriver with default settings.
    pub fn new() -> Self {
        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: "shell".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::CliSubprocess,
                description: "Execute shell commands via subprocess".into(),
                capabilities: vec!["execute_command".into()],
            },
            default_timeout: Duration::from_secs(30),
        }
    }

    /// Create with a custom timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
}

impl Default for ShellDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SoftwareDriver for ShellDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        // Shell is always available — no initialization needed.
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(ShellTool {
            default_timeout: self.default_timeout,
        })]
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        // Shell is always healthy on Unix systems.
        Ok(true)
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_driver_manifest() {
        let driver = ShellDriver::new();
        let m = driver.manifest();
        assert_eq!(m.name, "shell");
        assert_eq!(m.driver_type, DriverType::CliSubprocess);
        assert!(m.capabilities.contains(&"execute_command".into()));
    }

    #[test]
    fn shell_driver_tools() {
        let driver = ShellDriver::new();
        let tools = driver.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "shell");
    }

    #[tokio::test]
    async fn shell_driver_lifecycle() {
        let mut driver = ShellDriver::new();
        driver.initialize().await.unwrap();
        assert!(driver.health_check().await.unwrap());
        driver.shutdown().await.unwrap();
    }

    #[test]
    fn shell_driver_custom_timeout() {
        let driver = ShellDriver::new().with_timeout(Duration::from_secs(60));
        assert_eq!(driver.default_timeout, Duration::from_secs(60));
    }
}
