//! `ClaudeCodeDriver` — wraps Claude Code CLI as a `SoftwareDriver`.
//!
//! This is a user-installable plugin driver, not built into the kernel.
//! It exposes Claude Code's programming capabilities as Agent OS tools.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use macaca_driver::driver::{DriverManifest, DriverType, SoftwareDriver};
use macaca_proto::{DriverId, MacacaResult};
use macaca_tools::Tool;

use crate::config::ClaudeCodeConfig;
use crate::tools::{
    ClaudeCodeExecuteTool, ClaudeCodeResumeTool, ClaudeCodeStatusTool, SharedConfig,
};

/// A software driver that controls Claude Code CLI for autonomous programming.
///
/// # Example
///
/// ```no_run
/// use macaca_driver_claude_code::{ClaudeCodeDriver, ClaudeCodeConfig};
///
/// let config = ClaudeCodeConfig::new("/path/to/project")
///     .with_model("claude-sonnet-4-20250514")
///     .dangerously_skip_permissions();
///
/// let driver = ClaudeCodeDriver::new(config);
/// // Register with DriverRegistry, then agents can use claude_code_execute tool.
/// ```
pub struct ClaudeCodeDriver {
    manifest: DriverManifest,
    config: SharedConfig,
}

impl ClaudeCodeDriver {
    /// Create a new Claude Code Driver with the given configuration.
    pub fn new(config: ClaudeCodeConfig) -> Self {
        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: "claude-code".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::CliSubprocess,
                description: "Control Claude Code CLI for autonomous programming".into(),
                capabilities: vec![
                    "execute_prompt".into(),
                    "continue_session".into(),
                    "check_status".into(),
                ],
            },
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Get a reference to the shared config.
    pub fn config(&self) -> &SharedConfig {
        &self.config
    }
}

#[async_trait]
impl SoftwareDriver for ClaudeCodeDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        // Verify the working directory exists.
        let config = self.config.read().await;
        if !config.work_dir.exists() {
            tracing::warn!(
                work_dir = %config.work_dir.display(),
                "Claude Code work directory does not exist, will be created on first use"
            );
        }
        tracing::info!(
            driver = "claude-code",
            claude_bin = %config.claude_bin,
            work_dir = %config.work_dir.display(),
            "Claude Code driver initialized"
        );
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(ClaudeCodeExecuteTool {
                config: Arc::clone(&self.config),
            }),
            Box::new(ClaudeCodeResumeTool {
                config: Arc::clone(&self.config),
            }),
            Box::new(ClaudeCodeStatusTool {
                config: Arc::clone(&self.config),
            }),
        ]
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        let config = self.config.read().await;
        let result = tokio::process::Command::new(&config.claude_bin)
            .arg("--version")
            .env_remove("CLAUDECODE")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        match result {
            Ok(status) => Ok(status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        tracing::info!(driver = "claude-code", "Claude Code driver shut down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_manifest() {
        let driver = ClaudeCodeDriver::new(ClaudeCodeConfig::new("/tmp/test"));
        let m = driver.manifest();
        assert_eq!(m.name, "claude-code");
        assert_eq!(m.driver_type, DriverType::CliSubprocess);
        assert!(m.capabilities.contains(&"execute_prompt".into()));
        assert!(m.capabilities.contains(&"continue_session".into()));
        assert!(m.capabilities.contains(&"check_status".into()));
    }

    #[test]
    fn driver_exposes_three_tools() {
        let driver = ClaudeCodeDriver::new(ClaudeCodeConfig::new("/tmp/test"));
        let tools = driver.tools();
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"claude_code_execute"));
        assert!(names.contains(&"claude_code_resume"));
        assert!(names.contains(&"claude_code_status"));
    }

    #[tokio::test]
    async fn driver_lifecycle() {
        let mut driver = ClaudeCodeDriver::new(ClaudeCodeConfig::new("/tmp"));
        driver.initialize().await.unwrap();
        // health_check may return false if claude is not installed — that's ok.
        let _healthy = driver.health_check().await.unwrap();
        driver.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn driver_with_custom_config() {
        let config = ClaudeCodeConfig::new("/tmp/project")
            .with_model("claude-sonnet-4-20250514")
            .dangerously_skip_permissions()
            .with_timeout(600);

        let driver = ClaudeCodeDriver::new(config);
        let cfg = driver.config().read().await;
        assert_eq!(cfg.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(
            cfg.permission_mode,
            crate::config::PermissionMode::DangerouslySkipPermissions
        );
    }
}
