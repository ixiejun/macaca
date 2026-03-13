//! Built-in tool implementations: FileReadTool, FileWriteTool, ShellTool, DefaultToolSet.

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use serde_json::Value;
use std::time::Duration;
use tokio::{fs, process::Command, time::timeout};
use tracing::instrument;

use crate::tool::{Tool, ToolSet};

// ── FileReadTool ──────────────────────────────────────────────────────────────

/// Reads a file from the filesystem.
///
/// Input: `{ "path": "<file path>" }`
/// Output: `{ "content": "<file content>" }`
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Input: { \"path\": \"<path>\" }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to read" }
            },
            "required": ["path"]
        })
    }

    #[instrument(name = "file_read", skip(self), fields(path))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("file_read requires 'path' field".into()))?;

        tracing::Span::current().record("path", path);

        let content = fs::read_to_string(path).await.map_err(|e| {
            MacacaError::Agent(format!("file_read failed for '{}': {}", path, e))
        })?;

        Ok(serde_json::json!({ "content": content }))
    }
}

// ── FileWriteTool ─────────────────────────────────────────────────────────────

/// Writes content to a file, creating parent directories as needed.
///
/// Input: `{ "path": "<file path>", "content": "<text>" }`
/// Output: `{ "bytes_written": <n> }`
pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Input: { \"path\": \"<path>\", \"content\": \"<text>\" }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to write" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        })
    }

    #[instrument(name = "file_write", skip(self), fields(path))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("file_write requires 'path' field".into()))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("file_write requires 'content' field".into()))?;

        tracing::Span::current().record("path", path);

        if let Some(parent) = std::path::Path::new(path).parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                MacacaError::Agent(format!("file_write: failed to create dirs for '{}': {}", path, e))
            })?;
        }

        let bytes = content.len();
        fs::write(path, content).await.map_err(|e| {
            MacacaError::Agent(format!("file_write failed for '{}': {}", path, e))
        })?;

        Ok(serde_json::json!({ "bytes_written": bytes }))
    }
}

// ── ShellTool ─────────────────────────────────────────────────────────────────

/// Executes a shell command with a configurable timeout.
///
/// Input: `{ "command": "<shell command>", "timeout_secs": <optional u64> }`
/// Output: `{ "stdout": "...", "stderr": "...", "exit_code": <i32> }`
pub struct ShellTool {
    /// Default timeout for shell commands.
    pub default_timeout: Duration,
}

impl Default for ShellTool {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Input: { \"command\": \"<cmd>\", \"timeout_secs\": <optional> }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (optional)" }
            },
            "required": ["command"]
        })
    }

    #[instrument(name = "shell", skip(self), fields(command))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("shell requires 'command' field".into()))?;

        tracing::Span::current().record("command", command);

        let timeout_secs = input["timeout_secs"]
            .as_u64()
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let fut = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output();

        let output = timeout(timeout_secs, fut).await.map_err(|_| {
            MacacaError::Timeout(format!("shell command timed out after {}s: {}", timeout_secs.as_secs(), command))
        })?.map_err(|e| {
            MacacaError::Agent(format!("shell command failed to spawn: {}", e))
        })?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        }))
    }
}

// ── DefaultToolSet ────────────────────────────────────────────────────────────

/// The default collection of built-in tools.
pub struct DefaultToolSet {
    tools: Vec<Box<dyn Tool>>,
}

impl DefaultToolSet {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Box::new(FileReadTool),
                Box::new(FileWriteTool),
                Box::new(ShellTool::default()),
            ],
        }
    }
}

impl Default for DefaultToolSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSet for DefaultToolSet {
    fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_read_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let path_str = path.to_str().unwrap();

        let write_tool = FileWriteTool;
        let result = write_tool
            .execute(serde_json::json!({ "path": path_str, "content": "hello aos" }))
            .await
            .unwrap();
        assert_eq!(result["bytes_written"], 9);

        let read_tool = FileReadTool;
        let result = read_tool
            .execute(serde_json::json!({ "path": path_str }))
            .await
            .unwrap();
        assert_eq!(result["content"], "hello aos");
    }

    #[tokio::test]
    async fn file_read_missing_path_field() {
        let result = FileReadTool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_write_missing_content_field() {
        let result = FileWriteTool
            .execute(serde_json::json!({ "path": "/tmp/x" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shell_tool_echo() {
        let tool = ShellTool::default();
        let result = tool
            .execute(serde_json::json!({ "command": "echo hello" }))
            .await
            .unwrap();
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "hello");
        assert_eq!(result["exit_code"], 0);
    }

    #[tokio::test]
    async fn shell_tool_timeout() {
        let tool = ShellTool {
            default_timeout: Duration::from_millis(100),
        };
        let result = tool
            .execute(serde_json::json!({ "command": "sleep 10" }))
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("timed out"));
    }

    #[tokio::test]
    async fn shell_tool_exit_code() {
        let tool = ShellTool::default();
        let result = tool
            .execute(serde_json::json!({ "command": "exit 42" }))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 42);
    }

    #[test]
    fn default_toolset_contains_all_builtins() {
        let ts = DefaultToolSet::new();
        let names: Vec<&str> = ts.tools().iter().map(|t| t.name()).collect();
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
        assert!(names.contains(&"shell"));
    }

    #[test]
    fn toolset_get_tool_by_name() {
        let ts = DefaultToolSet::new();
        assert!(ts.get_tool("file_read").is_some());
        assert!(ts.get_tool("nonexistent").is_none());
    }
}
