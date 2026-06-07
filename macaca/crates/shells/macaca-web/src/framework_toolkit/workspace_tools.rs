//! Workspace-scoped file and shell tools (Adapter over app workspace root).
//!
//! Relative paths resolve against the registered application workspace. These tools
//! replace global builtins so agents cannot escape workspace boundaries.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use tokio::{fs, process::Command, time::timeout};

pub(crate) fn normalize_tool_input(input: &serde_json::Value) -> Cow<'_, serde_json::Value> {
    if let Some(s) = input.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            if parsed.is_object() {
                return Cow::Owned(parsed);
            }
        }
    }
    Cow::Borrowed(input)
}

fn pick_path_str(input: &serde_json::Value) -> Option<&str> {
    for key in ["path", "file_path", "filepath", "file", "filename"] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn pick_content_str(input: &serde_json::Value) -> Option<&str> {
    for key in ["content", "text", "body"] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            return Some(s);
        }
    }
    None
}

pub(crate) fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> PathBuf {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

pub(super) struct WorkspaceFileReadTool {
    pub(super) workspace_root: PathBuf,
}

#[async_trait]
impl macaca_sdk::tools::Tool for WorkspaceFileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Relative paths resolve from the app workspace root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or workspace-relative file path to read" },
                "file_path": { "type": "string", "description": "Alias for path (same meaning)" },
                "filepath": { "type": "string", "description": "Alias for path (same meaning)" }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let raw_path = pick_path_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent(
                "file_read requires non-empty 'path' (or alias 'file_path' / 'filepath')".into(),
            )
        })?;
        let path = resolve_workspace_path(&self.workspace_root, raw_path);
        let content = fs::read_to_string(&path).await.map_err(|e| {
            macaca_proto::MacacaError::Agent(format!(
                "file_read failed for '{}': {}",
                path.display(),
                e
            ))
        })?;
        Ok(serde_json::json!({ "content": content, "path": path.display().to_string() }))
    }
}

pub(super) struct WorkspaceFileWriteTool {
    pub(super) workspace_root: PathBuf,
}

#[async_trait]
impl macaca_sdk::tools::Tool for WorkspaceFileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Relative paths resolve from the app workspace root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Workspace-relative or absolute file path to write" },
                "file_path": { "type": "string", "description": "Alias for path" },
                "filepath": { "type": "string", "description": "Alias for path" },
                "content": { "type": "string", "description": "Full file content as a string" },
                "text": { "type": "string", "description": "Alias for content" },
                "body": { "type": "string", "description": "Alias for content" }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let raw_path = pick_path_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent("file_write requires non-empty 'path'".into())
        })?;
        let content = pick_content_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent(
                "file_write requires 'content' as a string (or alias 'text' / 'body')".into(),
            )
        })?;
        let path = resolve_workspace_path(&self.workspace_root, raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                macaca_proto::MacacaError::Agent(format!(
                    "file_write: failed to create dirs for '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }
        fs::write(&path, content).await.map_err(|e| {
            macaca_proto::MacacaError::Agent(format!(
                "file_write failed for '{}': {}",
                path.display(),
                e
            ))
        })?;
        Ok(serde_json::json!({
            "bytes_written": content.len(),
            "path": path.display().to_string()
        }))
    }
}

pub(super) struct WorkspaceShellTool {
    pub(super) workspace_root: PathBuf,
    pub(super) default_timeout: Duration,
}

#[async_trait]
impl macaca_sdk::tools::Tool for WorkspaceShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command from the app workspace root. Relative paths resolve from that root."
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

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let command = input["command"].as_str().ok_or_else(|| {
            macaca_proto::MacacaError::Agent("shell requires 'command' field".into())
        })?;
        let timeout_secs = input["timeout_secs"]
            .as_u64()
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let fut = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace_root)
            .output();

        let output = timeout(timeout_secs, fut)
            .await
            .map_err(|_| {
                macaca_proto::MacacaError::Timeout(format!(
                    "shell command timed out after {}s: {}",
                    timeout_secs.as_secs(),
                    command
                ))
            })?
            .map_err(|e| {
                macaca_proto::MacacaError::Agent(format!("shell command failed to spawn: {}", e))
            })?;

        Ok(serde_json::json!({
            "stdout": String::from_utf8_lossy(&output.stdout).into_owned(),
            "stderr": String::from_utf8_lossy(&output.stderr).into_owned(),
            "exit_code": output.status.code().unwrap_or(-1),
            "cwd": self.workspace_root.display().to_string(),
        }))
    }
}
