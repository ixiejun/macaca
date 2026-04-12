//! Configuration for the Claude Code Driver.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Permission mode for Claude Code execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Normal mode — Claude Code may prompt for permissions.
    Default,
    /// Skip all permission checks. Use with caution.
    DangerouslySkipPermissions,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
}

/// Configuration for the Claude Code Driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeConfig {
    /// Path to the `claude` CLI binary. Defaults to `"claude"`.
    #[serde(default = "default_claude_bin")]
    pub claude_bin: String,

    /// Working directory for Claude Code sessions.
    pub work_dir: PathBuf,

    /// LLM model to use (e.g., "claude-sonnet-4-20250514").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Allowed tools for Claude Code sessions.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Maximum number of conversation turns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,

    /// Additional system prompt injected into Claude Code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Permission mode for execution.
    #[serde(default)]
    pub permission_mode: PermissionMode,

    /// Timeout in seconds for a single Claude Code invocation.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_claude_bin() -> String {
    "claude".into()
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

impl ClaudeCodeConfig {
    /// Create a config with only the work directory specified, using defaults for everything else.
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self {
            claude_bin: default_claude_bin(),
            work_dir: work_dir.into(),
            model: None,
            allowed_tools: Vec::new(),
            max_turns: None,
            system_prompt: None,
            permission_mode: PermissionMode::Default,
            timeout_secs: default_timeout(),
        }
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set permission mode to skip permissions.
    pub fn dangerously_skip_permissions(mut self) -> Self {
        self.permission_mode = PermissionMode::DangerouslySkipPermissions;
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Path to the Claude Code CLI binary (overrides default `"claude"` on `PATH`).
    pub fn with_claude_bin(mut self, bin: impl Into<String>) -> Self {
        self.claude_bin = bin.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ClaudeCodeConfig::new("/tmp/test");
        assert_eq!(config.claude_bin, "claude");
        assert_eq!(config.work_dir, PathBuf::from("/tmp/test"));
        assert!(config.model.is_none());
        assert_eq!(config.permission_mode, PermissionMode::Default);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn builder_methods() {
        let config = ClaudeCodeConfig::new("/tmp/test")
            .with_model("claude-sonnet-4-20250514")
            .dangerously_skip_permissions()
            .with_timeout(600)
            .with_claude_bin("/opt/claude");

        assert_eq!(config.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(
            config.permission_mode,
            PermissionMode::DangerouslySkipPermissions
        );
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.claude_bin, "/opt/claude");
    }

    #[test]
    fn serialize_roundtrip() {
        let config = ClaudeCodeConfig::new("/tmp/work").with_model("test-model");
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ClaudeCodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model.as_deref(), Some("test-model"));
        assert_eq!(parsed.work_dir, PathBuf::from("/tmp/work"));
    }
}
