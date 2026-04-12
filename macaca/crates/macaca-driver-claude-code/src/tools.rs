//! Tools exposed by the Claude Code Driver.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::{Tool, TraceEvent};

use crate::config::{ClaudeCodeConfig, PermissionMode};
use crate::output::{output_to_json, parse_claude_stream};

/// Shared config for all Claude Code tools within one driver instance.
pub(crate) type SharedConfig = Arc<RwLock<ClaudeCodeConfig>>;

// ---------------------------------------------------------------------------
// claude_code_execute
// ---------------------------------------------------------------------------

/// Execute a prompt with Claude Code CLI.
pub struct ClaudeCodeExecuteTool {
    pub(crate) config: SharedConfig,
}

#[async_trait]
impl Tool for ClaudeCodeExecuteTool {
    fn name(&self) -> &str {
        "claude_code_execute"
    }

    fn description(&self) -> &str {
        "Execute a programming task using Claude Code CLI. Returns the result, session ID, cost, and duration."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The programming task or instruction to execute"
                },
                "work_dir": {
                    "type": "string",
                    "description": "Optional working directory override"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID to continue a previous conversation"
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt to inject"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("Missing 'prompt' parameter".into()))?;

        let config = self.config.read().await;

        let work_dir = input["work_dir"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| config.work_dir.clone());

        let session_id = input["session_id"].as_str();
        let system_prompt = input["system_prompt"]
            .as_str()
            .or(config.system_prompt.as_deref());

        let output = run_claude_cli(
            &config.claude_bin,
            prompt,
            &work_dir,
            session_id,
            system_prompt,
            config.model.as_deref(),
            &config.permission_mode,
            config.max_turns,
            Duration::from_secs(config.timeout_secs),
        )
        .await?;

        Ok(output)
    }

    async fn execute_streaming(
        &self,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> MacacaResult<Value> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("Missing 'prompt' parameter".into()))?;

        let config = self.config.read().await;

        let work_dir = input["work_dir"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| config.work_dir.clone());

        let session_id = input["session_id"].as_str();
        let system_prompt = input["system_prompt"]
            .as_str()
            .or(config.system_prompt.as_deref());

        let output = run_claude_cli_streaming(
            &config.claude_bin,
            prompt,
            &work_dir,
            session_id,
            system_prompt,
            config.model.as_deref(),
            &config.permission_mode,
            config.max_turns,
            Duration::from_secs(config.timeout_secs),
            event_tx,
        )
        .await?;

        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// claude_code_resume
// ---------------------------------------------------------------------------

/// Resume a previous Claude Code session.
pub struct ClaudeCodeResumeTool {
    pub(crate) config: SharedConfig,
}

#[async_trait]
impl Tool for ClaudeCodeResumeTool {
    fn name(&self) -> &str {
        "claude_code_resume"
    }

    fn description(&self) -> &str {
        "Continue a previous Claude Code session by its session ID."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session ID to resume"
                },
                "prompt": {
                    "type": "string",
                    "description": "The instruction to continue with"
                }
            },
            "required": ["session_id", "prompt"]
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let session_id = input["session_id"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("Missing 'session_id' parameter".into()))?;

        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("Missing 'prompt' parameter".into()))?;

        let config = self.config.read().await;

        let output = run_claude_cli(
            &config.claude_bin,
            prompt,
            &config.work_dir,
            Some(session_id),
            config.system_prompt.as_deref(),
            config.model.as_deref(),
            &config.permission_mode,
            config.max_turns,
            Duration::from_secs(config.timeout_secs),
        )
        .await?;

        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// claude_code_status
// ---------------------------------------------------------------------------

/// Check if Claude Code CLI is available.
pub struct ClaudeCodeStatusTool {
    pub(crate) config: SharedConfig,
}

#[async_trait]
impl Tool for ClaudeCodeStatusTool {
    fn name(&self) -> &str {
        "claude_code_status"
    }

    fn description(&self) -> &str {
        "Check if Claude Code CLI is installed and available. Returns version info."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let config = self.config.read().await;

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::process::Command::new(&config.claude_bin)
                .arg("--version")
                .env_remove("CLAUDECODE")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let available = output.status.success();

                Ok(serde_json::json!({
                    "available": available,
                    "version": if available { &stdout } else { &stderr },
                    "binary": config.claude_bin,
                }))
            }
            Ok(Err(e)) => Ok(serde_json::json!({
                "available": false,
                "error": format!("Failed to run claude: {e}"),
                "binary": config.claude_bin,
            })),
            Err(_) => Ok(serde_json::json!({
                "available": false,
                "error": "Timed out checking claude version",
                "binary": config.claude_bin,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared CLI execution logic
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn run_claude_cli(
    claude_bin: &str,
    prompt: &str,
    work_dir: &std::path::Path,
    session_id: Option<&str>,
    system_prompt: Option<&str>,
    model: Option<&str>,
    permission_mode: &PermissionMode,
    max_turns: Option<u32>,
    timeout: Duration,
) -> MacacaResult<Value> {
    let mut cmd = tokio::process::Command::new(claude_bin);

    // Print mode with stream-json output for full trace visibility.
    cmd.arg("-p").arg(prompt);
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--verbose");

    // Session continuation.
    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }

    // System prompt.
    if let Some(sp) = system_prompt {
        cmd.arg("--system-prompt").arg(sp);
    }

    // Model selection.
    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    // Permission mode.
    if *permission_mode == PermissionMode::DangerouslySkipPermissions {
        cmd.arg("--dangerously-skip-permissions");
    }

    // Max turns.
    if let Some(turns) = max_turns {
        cmd.arg("--max-turns").arg(turns.to_string());
    }

    // Working directory.
    cmd.current_dir(work_dir);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Prevent "nested session" error when Agent OS itself runs inside Claude Code.
    cmd.env_remove("CLAUDECODE");

    tracing::debug!(claude_bin, ?work_dir, ?session_id, "executing claude code");

    let child_result = cmd.spawn();
    let child =
        child_result.map_err(|e| MacacaError::Agent(format!("Failed to spawn claude: {e}")))?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| MacacaError::Timeout("Claude Code execution timed out".into()))?
        .map_err(|e| MacacaError::Agent(format!("Claude Code process error: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    tracing::debug!(exit_code, "claude code finished");

    let parsed = parse_claude_stream(&stdout, &stderr, exit_code)?;
    Ok(output_to_json(&parsed))
}

/// Streaming version that reads stdout line by line and sends trace events.
#[allow(clippy::too_many_arguments)]
async fn run_claude_cli_streaming(
    claude_bin: &str,
    prompt: &str,
    work_dir: &std::path::Path,
    session_id: Option<&str>,
    system_prompt: Option<&str>,
    model: Option<&str>,
    permission_mode: &PermissionMode,
    max_turns: Option<u32>,
    timeout: Duration,
    event_tx: Option<UnboundedSender<TraceEvent>>,
) -> MacacaResult<Value> {
    let mut cmd = tokio::process::Command::new(claude_bin);

    // Print mode with stream-json output for full trace visibility.
    cmd.arg("-p").arg(prompt);
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--verbose");

    // Session continuation.
    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }

    // System prompt.
    if let Some(sp) = system_prompt {
        cmd.arg("--system-prompt").arg(sp);
    }

    // Model selection.
    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    // Permission mode.
    if *permission_mode == PermissionMode::DangerouslySkipPermissions {
        cmd.arg("--dangerously-skip-permissions");
    }

    // Max turns.
    if let Some(turns) = max_turns {
        cmd.arg("--max-turns").arg(turns.to_string());
    }

    // Working directory.
    cmd.current_dir(work_dir);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Prevent "nested session" error when Agent OS itself runs inside Claude Code.
    cmd.env_remove("CLAUDECODE");

    tracing::debug!(
        claude_bin,
        ?work_dir,
        ?session_id,
        "executing claude code (streaming)"
    );

    let mut child = cmd
        .spawn()
        .map_err(|e| MacacaError::Agent(format!("Failed to spawn claude: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| MacacaError::Agent("Failed to capture stdout from claude process".into()))?;

    let mut stderr_buf = String::new();
    let mut all_stdout = String::new();
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // Read lines in a timeout wrapper
    let read_result = tokio::time::timeout(timeout, async {
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| MacacaError::Agent(format!("Error reading claude stdout: {e}")))?
        {
            all_stdout.push_str(&line);
            all_stdout.push('\n');

            // Try to parse and emit trace events
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                if let Some(tx) = &event_tx {
                    emit_trace_events(&json, tx);
                }
            }
        }
        Ok::<_, MacacaError>(())
    })
    .await;

    match read_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            let _ = child.kill().await;
            return Err(MacacaError::Timeout(
                "Claude Code execution timed out".into(),
            ));
        }
    }

    // Wait for process to complete
    let status = child
        .wait()
        .await
        .map_err(|e| MacacaError::Agent(format!("Claude Code process wait error: {e}")))?;

    let exit_code = status.code().unwrap_or(-1);
    tracing::debug!(exit_code, "claude code finished (streaming)");

    // Read any stderr
    if let Some(mut stderr_handle) = child.stderr.take() {
        use tokio::io::AsyncReadExt;
        let _ = stderr_handle.read_to_string(&mut stderr_buf).await;
    }

    let parsed = parse_claude_stream(&all_stdout, &stderr_buf, exit_code)?;
    Ok(output_to_json(&parsed))
}

/// Extract and emit trace events from a single JSON line.
fn emit_trace_events(json: &Value, event_tx: &UnboundedSender<TraceEvent>) {
    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "assistant" => {
            if let Some(message) = json.get("message") {
                if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                    for block in content {
                        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        match block_type {
                            "thinking" => {
                                if let Some(thinking) =
                                    block.get("thinking").and_then(|v| v.as_str())
                                {
                                    if !thinking.is_empty() {
                                        let _ = event_tx.send(TraceEvent {
                                            event_type: "thinking".into(),
                                            tool_name: None,
                                            tool_input: None,
                                            tool_result: None,
                                            thinking: Some(thinking.to_string()),
                                            text: None,
                                            is_error: None,
                                        });
                                    }
                                }
                            }
                            "tool_use" => {
                                let tool_name =
                                    block.get("name").and_then(|v| v.as_str()).unwrap_or("tool");
                                let tool_input = block.get("input").cloned();
                                let _ = event_tx.send(TraceEvent {
                                    event_type: "tool_use".into(),
                                    tool_name: Some(tool_name.to_string()),
                                    tool_input,
                                    tool_result: None,
                                    thinking: None,
                                    text: None,
                                    is_error: None,
                                });
                            }
                            "text" => {
                                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                    if !text.is_empty() {
                                        let _ = event_tx.send(TraceEvent {
                                            event_type: "text".into(),
                                            tool_name: None,
                                            tool_input: None,
                                            tool_result: None,
                                            thinking: None,
                                            text: Some(text.to_string()),
                                            is_error: None,
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        "user" => {
            if let Some(message) = json.get("message") {
                if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                    for block in content {
                        if block.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                            let tool_name = json
                                .get("tool_use_result")
                                .and_then(|r| {
                                    if r.get("stdout").is_some() {
                                        Some("Bash".to_string())
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(|| "tool".to_string());

                            let tool_result = block
                                .get("content")
                                .and_then(|v| v.as_str())
                                .or_else(|| {
                                    json.get("tool_use_result")
                                        .and_then(|r| r.get("stdout").and_then(|s| s.as_str()))
                                })
                                .unwrap_or("");

                            let is_err = block
                                .get("is_error")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            let _ = event_tx.send(TraceEvent {
                                event_type: "tool_result".into(),
                                tool_name: Some(tool_name),
                                tool_input: None,
                                tool_result: Some(tool_result.to_string()),
                                thinking: None,
                                text: None,
                                is_error: Some(is_err),
                            });
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> SharedConfig {
        Arc::new(RwLock::new(ClaudeCodeConfig::new("/tmp/test")))
    }

    #[test]
    fn execute_tool_metadata() {
        let tool = ClaudeCodeExecuteTool {
            config: make_config(),
        };
        assert_eq!(tool.name(), "claude_code_execute");
        assert!(!tool.description().is_empty());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["prompt"].is_object());
    }

    #[test]
    fn resume_tool_metadata() {
        let tool = ClaudeCodeResumeTool {
            config: make_config(),
        };
        assert_eq!(tool.name(), "claude_code_resume");
        let schema = tool.parameters_schema();
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("session_id")));
    }

    #[test]
    fn status_tool_metadata() {
        let tool = ClaudeCodeStatusTool {
            config: make_config(),
        };
        assert_eq!(tool.name(), "claude_code_status");
    }

    #[tokio::test]
    async fn execute_missing_prompt_returns_error() {
        let tool = ClaudeCodeExecuteTool {
            config: make_config(),
        };
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("prompt"));
    }

    #[tokio::test]
    async fn resume_missing_fields_returns_error() {
        let tool = ClaudeCodeResumeTool {
            config: make_config(),
        };
        let result = tool.execute(serde_json::json!({"prompt": "hi"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("session_id"));
    }

    #[tokio::test]
    async fn status_tool_runs() {
        // This test calls the real `claude --version` if available,
        // but gracefully handles the case where it's not installed.
        let config = Arc::new(RwLock::new(ClaudeCodeConfig::new("/tmp")));
        let tool = ClaudeCodeStatusTool { config };
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        // Should always return a JSON object, whether claude is installed or not.
        assert!(result.is_object());
        assert!(result.get("available").is_some() || result.get("error").is_some());
    }

    use crate::config::ClaudeCodeConfig;
}
