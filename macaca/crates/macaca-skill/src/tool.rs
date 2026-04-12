//! `SkillTool` — wraps a `SkillDefinition` as an `macaca_tools::Tool`.

use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::Tool;

use crate::definition::{SkillDefinition, SkillEntryPoint};

/// A Tool backed by a skill definition.
///
/// Executes the skill's entry point (shell command, script, etc.)
/// when invoked.
pub struct SkillTool {
    definition: SkillDefinition,
}

impl SkillTool {
    /// Create a new SkillTool from a definition.
    pub fn new(definition: SkillDefinition) -> Self {
        Self { definition }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        &self.definition.description
    }

    fn parameters_schema(&self) -> Value {
        self.definition.parameters.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        match &self.definition.entry_point {
            SkillEntryPoint::ShellCommand { command, args } => {
                execute_shell(command, args, &input).await
            }
            SkillEntryPoint::Script { path, interpreter } => {
                let cmd = interpreter.as_deref().unwrap_or("sh");
                execute_shell(cmd, &[path.clone()], &input).await
            }
            SkillEntryPoint::McpServer { .. } => {
                // MCP-backed skills should be connected via McpDriver instead.
                Err(MacacaError::Agent(
                    "MCP skills should be loaded via McpDriver, not SkillTool".into(),
                ))
            }
        }
    }
}

/// Execute a shell command, extracting dynamic arguments from JSON input.
///
/// If the input JSON contains an `action` field (string), it is appended
/// as the first extra argument. If it contains an `args` field (array of
/// strings), those are appended after. This allows LLMs to call skills
/// like `npx openspec init --work-dir ./myproject`.
async fn execute_shell(command: &str, base_args: &[String], input: &Value) -> MacacaResult<Value> {
    // Build full argument list: base_args + action + extra args.
    let mut full_args: Vec<String> = base_args.to_vec();

    if let Some(action) = input.get("action").and_then(|v| v.as_str()) {
        full_args.push(action.to_string());
    }

    if let Some(extra) = input.get("args").and_then(|v| v.as_array()) {
        for item in extra {
            if let Some(s) = item.as_str() {
                full_args.push(s.to_string());
            }
        }
    }

    // Set working directory if provided (cd to the project directory).
    let work_dir = input
        .get("work_dir")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(&full_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(ref dir) = work_dir {
        cmd.current_dir(dir);
    }

    let output = tokio::time::timeout(
        Duration::from_secs(120),
        cmd.spawn()
            .map_err(|e| MacacaError::Agent(format!("Failed to spawn skill command: {e}")))?
            .wait_with_output(),
    )
    .await
    .map_err(|_| MacacaError::Timeout(format!("Skill '{}' timed out", command)))?
    .map_err(|e| MacacaError::Agent(format!("Skill command failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    let dir_info = work_dir
        .as_ref()
        .map(|d| format!(" (in {})", d.display()))
        .unwrap_or_default();

    Ok(serde_json::json!({
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code,
        "command": format!("{} {}{}", command, full_args.join(" "), dir_info),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_skill() -> SkillDefinition {
        SkillDefinition {
            name: "echo-skill".into(),
            description: "Echoes input".into(),
            version: "0.1.0".into(),
            entry_point: SkillEntryPoint::ShellCommand {
                command: "echo".into(),
                args: vec!["hello from skill".into()],
            },
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn skill_tool_metadata() {
        let tool = SkillTool::new(echo_skill());
        assert_eq!(tool.name(), "echo-skill");
        assert_eq!(tool.description(), "Echoes input");
    }

    #[tokio::test]
    async fn skill_tool_execute_shell() {
        let tool = SkillTool::new(echo_skill());
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"]
            .as_str()
            .unwrap()
            .contains("hello from skill"));
        assert!(result["command"].as_str().unwrap().contains("echo"));
    }

    #[tokio::test]
    async fn skill_tool_execute_with_action_and_args() {
        let tool = SkillTool::new(SkillDefinition {
            name: "echo-action".into(),
            description: "Echo with action".into(),
            version: "0.1.0".into(),
            entry_point: SkillEntryPoint::ShellCommand {
                command: "echo".into(),
                args: vec![],
            },
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } }
                }
            }),
        });
        let result = tool
            .execute(serde_json::json!({"action": "init", "args": ["--verbose"]}))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 0);
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("init"));
        assert!(stdout.contains("--verbose"));
    }

    #[tokio::test]
    async fn mcp_skill_returns_error() {
        let tool = SkillTool::new(SkillDefinition {
            name: "mcp-skill".into(),
            description: "An MCP skill".into(),
            version: "0.1.0".into(),
            entry_point: SkillEntryPoint::McpServer {
                command: "npx".into(),
                args: vec![],
            },
            parameters: serde_json::json!({}),
        });
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("McpDriver"));
    }
}
