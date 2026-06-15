//! Runtime-host factory for workspace-scoped file and shell toolkit tools.
//!
//! Shells can decide which base tools are allowed for a specific application
//! agent, but concrete file and process tools are runtime-host owned capability
//! adapters. This module keeps workspace tool construction out of presentation
//! shells while preserving a narrow, provider-neutral request/response surface.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::{Tool, ToolCommand};
use tokio::{fs, process::Command, time::timeout};

/// Request used by shells to ask runtime-host for workspace-scoped tools.
///
/// The caller passes policy decisions as booleans instead of runtime-host
/// reading application manifests directly. This preserves the shell/framework
/// policy boundary while moving concrete tool construction into the approved
/// runtime-host composition root.
#[derive(Debug, Clone)]
pub struct WorkspaceToolkitBootstrapRequest {
    /// Trusted application workspace root resolved by the platform registry.
    pub workspace_root: PathBuf,
    /// Whether to include the workspace-scoped file read tool.
    pub allow_file_read: bool,
    /// Whether to include the workspace-scoped file write tool.
    pub allow_file_write: bool,
    /// Whether to include the workspace-scoped shell tool.
    pub allow_shell: bool,
    /// Default shell timeout applied when the tool input omits one.
    pub default_shell_timeout: Duration,
}

/// Build workspace-scoped toolkit tools for one application agent.
///
/// Runtime-host logs the bootstrap boundary because the returned tools can
/// perform file-system or process side effects. Actual invocations still run
/// through the framework toolkit trace path after shells adapt the generic
/// tools to framework handlers.
pub fn bootstrap_workspace_toolkit_tools(
    request: WorkspaceToolkitBootstrapRequest,
    trace_label: impl Into<String>,
) -> Vec<Box<dyn Tool>> {
    let trace_label = trace_label.into();
    tracing::info!(
        trace_label = %trace_label,
        workspace_root = %request.workspace_root.display(),
        allow_file_read = request.allow_file_read,
        allow_file_write = request.allow_file_write,
        allow_shell = request.allow_shell,
        "runtime-host workspace toolkit bootstrap requested"
    );

    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    if request.allow_file_read {
        tools.push(Box::new(WorkspaceFileReadTool {
            workspace_root: request.workspace_root.clone(),
        }));
    }
    if request.allow_file_write {
        tools.push(Box::new(WorkspaceFileWriteTool {
            workspace_root: request.workspace_root.clone(),
        }));
    }
    if request.allow_shell {
        tools.push(Box::new(WorkspaceShellTool {
            workspace_root: request.workspace_root,
            default_timeout: request.default_shell_timeout,
        }));
    }

    tracing::info!(
        trace_label = %trace_label,
        count = tools.len(),
        "runtime-host workspace toolkit bootstrap completed"
    );
    tools
}

/// Normalize framework tool input that may arrive as a JSON object or as a
/// stringified JSON object from a model/tool-call provider.
pub fn normalize_workspace_tool_input(input: &serde_json::Value) -> Cow<'_, serde_json::Value> {
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

/// Resolve a path against the trusted workspace root.
///
/// Absolute paths are preserved for behavior parity with the previous Web
/// adapter. Relative paths are joined to the registered workspace root.
pub fn resolve_workspace_tool_path(workspace_root: &Path, raw_path: &str) -> PathBuf {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

struct WorkspaceFileReadTool {
    workspace_root: PathBuf,
}

#[async_trait]
impl Tool for WorkspaceFileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Relative paths resolve from the app workspace root."
    }

    fn tool_schema(&self) -> serde_json::Value {
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

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let input = normalize_workspace_tool_input(&input);
        let raw_path = pick_path_str(&input).ok_or_else(|| {
            MacacaError::Agent(
                "file_read requires non-empty 'path' (or alias 'file_path' / 'filepath')".into(),
            )
        })?;
        let path = resolve_workspace_tool_path(&self.workspace_root, raw_path);
        let content = fs::read_to_string(&path).await.map_err(|e| {
            MacacaError::Agent(format!("file_read failed for '{}': {}", path.display(), e))
        })?;
        Ok(serde_json::json!({ "content": content, "path": path.display().to_string() }))
    }
}

struct WorkspaceFileWriteTool {
    workspace_root: PathBuf,
}

#[async_trait]
impl Tool for WorkspaceFileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Relative paths resolve from the app workspace root."
    }

    fn tool_schema(&self) -> serde_json::Value {
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

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let input = normalize_workspace_tool_input(&input);
        let raw_path = pick_path_str(&input)
            .ok_or_else(|| MacacaError::Agent("file_write requires non-empty 'path'".into()))?;
        let content = pick_content_str(&input).ok_or_else(|| {
            MacacaError::Agent(
                "file_write requires 'content' as a string (or alias 'text' / 'body')".into(),
            )
        })?;
        let path = resolve_workspace_tool_path(&self.workspace_root, raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                MacacaError::Agent(format!(
                    "file_write: failed to create dirs for '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }
        fs::write(&path, content).await.map_err(|e| {
            MacacaError::Agent(format!("file_write failed for '{}': {}", path.display(), e))
        })?;
        Ok(serde_json::json!({
            "bytes_written": content.len(),
            "path": path.display().to_string()
        }))
    }
}

struct WorkspaceShellTool {
    workspace_root: PathBuf,
    default_timeout: Duration,
}

#[async_trait]
impl Tool for WorkspaceShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command from the app workspace root. Relative paths resolve from that root."
    }

    fn tool_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (optional)" }
            },
            "required": ["command"]
        })
    }

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let input = normalize_workspace_tool_input(&input);
        let command = input["command"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("shell requires 'command' field".into()))?;
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
                MacacaError::Timeout(format!(
                    "shell command timed out after {}s: {}",
                    timeout_secs.as_secs(),
                    command
                ))
            })?
            .map_err(|e| MacacaError::Agent(format!("shell command failed to spawn: {}", e)))?;

        Ok(serde_json::json!({
            "stdout": String::from_utf8_lossy(&output.stdout).into_owned(),
            "stderr": String::from_utf8_lossy(&output.stderr).into_owned(),
            "exit_code": output.status.code().unwrap_or(-1),
            "cwd": self.workspace_root.display().to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_respects_workspace_tool_policy() {
        let tools = bootstrap_workspace_toolkit_tools(
            WorkspaceToolkitBootstrapRequest {
                workspace_root: PathBuf::from("/tmp/macaca-workspace"),
                allow_file_read: true,
                allow_file_write: false,
                allow_shell: true,
                default_shell_timeout: Duration::from_secs(30),
            },
            "workspace-toolkit-test",
        );

        let names = tools.iter().map(|tool| tool.name()).collect::<Vec<_>>();
        assert_eq!(names, vec!["file_read", "shell"]);
    }
}
