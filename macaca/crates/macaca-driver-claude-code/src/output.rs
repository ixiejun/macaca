//! Claude Code stream-json output parser.
//!
//! Parses the `--output-format stream-json --verbose` output from Claude Code CLI
//! to extract the full execution trace including thinking, tool calls, and results.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use macaca_proto::MacacaError;

/// A single trace event from Claude Code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event type: "thinking", "tool_use", "tool_result", "text"
    #[serde(rename = "type")]
    pub event_type: String,
    /// Tool name for tool_use/tool_result events
    pub tool_name: Option<String>,
    /// Tool input for tool_use events
    pub tool_input: Option<Value>,
    /// Tool result for tool_result events (truncated if too long)
    pub tool_result: Option<String>,
    /// Thinking content for thinking events
    pub thinking: Option<String>,
    /// Text content for text events
    pub text: Option<String>,
    /// Whether this was an error
    pub is_error: Option<bool>,
}

/// Parsed output from a Claude Code CLI invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeOutput {
    /// The final text result from Claude Code.
    pub result: String,
    /// Session ID for continuation.
    pub session_id: Option<String>,
    /// Cost in USD (if reported).
    pub cost_usd: Option<f64>,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Whether the execution was successful.
    pub is_error: bool,
    /// Full execution trace (thinking, tool calls, results).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<TraceEvent>,
}

/// Parse the stream-json output from `claude -p --output-format stream-json --verbose`.
///
/// Each line is a JSON object with types like:
/// - `{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"..."}]}}`
/// - `{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{...}]}}`
/// - `{"type":"user","message":{"content":[{"type":"tool_result","content":"..."}]}}`
/// - `{"type":"result","result":"...","session_id":"...","total_cost_usd":...}`
pub fn parse_claude_stream(stdout: &str, stderr: &str, exit_code: i32) -> Result<ClaudeOutput, MacacaError> {
    // Handle error case with no stdout
    if exit_code != 0 && stdout.trim().is_empty() {
        return Ok(ClaudeOutput {
            result: format!("Claude Code exited with code {exit_code}: {stderr}"),
            session_id: None,
            cost_usd: None,
            duration_ms: None,
            is_error: true,
            trace: vec![],
        });
    }

    let stdout = stdout.trim();
    if stdout.is_empty() {
        return Ok(ClaudeOutput {
            result: String::new(),
            session_id: None,
            cost_usd: None,
            duration_ms: None,
            is_error: exit_code != 0,
            trace: vec![],
        });
    }

    let mut result = String::new();
    let mut session_id = None;
    let mut cost_usd = None;
    let mut duration_ms = None;
    let mut is_error = exit_code != 0;
    let mut trace = Vec::new();

    tracing::debug!("Parsing Claude Code stream output, {} bytes", stdout.len());

    // Process each JSON line
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try to parse as JSON
        let Ok(json) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            // Final result line
            "result" => {
                result = extract_string(&json, "result").unwrap_or_default();
                session_id = extract_string(&json, "session_id");
                cost_usd = json
                    .get("total_cost_usd")
                    .or_else(|| json.get("cost_usd"))
                    .and_then(|v| v.as_f64());
                duration_ms = json.get("duration_ms").and_then(|v| v.as_u64());
                is_error = json.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
            }

            // Assistant message with content blocks
            "assistant" => {
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(content_arr) = content.as_array() {
                            tracing::debug!("Processing assistant message with {} content blocks", content_arr.len());
                            for block in content_arr {
                                let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                tracing::debug!("Content block type: {}", block_type);

                                match block_type {
                                    "thinking" => {
                                        let thinking = extract_string(block, "thinking").unwrap_or_default();
                                        if !thinking.is_empty() {
                                            trace.push(TraceEvent {
                                                event_type: "thinking".into(),
                                                tool_name: None,
                                                tool_input: None,
                                                tool_result: None,
                                                thinking: Some(thinking),
                                                text: None,
                                                is_error: None,
                                            });
                                        }
                                    }
                                    "tool_use" => {
                                        let tool_name = extract_string(block, "name").unwrap_or_default();
                                        let tool_input = block.get("input").cloned();
                                        trace.push(TraceEvent {
                                            event_type: "tool_use".into(),
                                            tool_name: Some(tool_name),
                                            tool_input,
                                            tool_result: None,
                                            thinking: None,
                                            text: None,
                                            is_error: None,
                                        });
                                    }
                                    "text" => {
                                        let text = extract_string(block, "text").unwrap_or_default();
                                        if !text.is_empty() {
                                            trace.push(TraceEvent {
                                                event_type: "text".into(),
                                                tool_name: None,
                                                tool_input: None,
                                                tool_result: None,
                                                thinking: None,
                                                text: Some(text),
                                                is_error: None,
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            // User message with tool result
            "user" => {
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(content_arr) = content.as_array() {
                            for block in content_arr {
                                let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");

                                if block_type == "tool_result" {
                                    // Get tool name from tool_use_result if available
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

                                    let tool_result = extract_string(block, "content")
                                        .or_else(|| {
                                            json.get("tool_use_result")
                                                .and_then(|r| extract_string(r, "stdout"))
                                        })
                                        .unwrap_or_default();

                                    let is_err = block.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);

                                    trace.push(TraceEvent {
                                        event_type: "tool_result".into(),
                                        tool_name: Some(tool_name),
                                        tool_input: None,
                                        tool_result: Some(tool_result),
                                        thinking: None,
                                        text: None,
                                        is_error: Some(is_err),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // System events (hooks, etc.) - skip for now
            "system" => {}

            _ => {}
        }
    }

    // If no result was found, try to extract from last text event
    if result.is_empty() {
        result = trace
            .iter()
            .rev()
            .find(|e| e.event_type == "text")
            .and_then(|e| e.text.clone())
            .unwrap_or_default();
    }

    tracing::debug!("Parsed Claude output: {} trace events, result length {}", trace.len(), result.len());

    Ok(ClaudeOutput {
        result,
        session_id,
        cost_usd,
        duration_ms,
        is_error,
        trace,
    })
}

fn extract_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...[truncated, {} bytes total]", &s[..max_len], s.len())
    } else {
        s.to_string()
    }
}

/// Convert a ClaudeOutput into a JSON Value suitable for Tool output.
pub fn output_to_json(output: &ClaudeOutput) -> Value {
    serde_json::json!({
        "result": output.result,
        "session_id": output.session_id,
        "cost_usd": output.cost_usd,
        "duration_ms": output.duration_ms,
        "is_error": output.is_error,
        "trace": output.trace,
    })
}

// Legacy function for backward compatibility
pub fn parse_claude_json(stdout: &str, stderr: &str, exit_code: i32) -> Result<ClaudeOutput, MacacaError> {
    parse_claude_stream(stdout, stderr, exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stream_json_with_tools() {
        let stdout = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me list the files."}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use","id":"call_1","name":"Bash","input":{"command":"ls -la"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","content":"file1.txt\nfile2.txt"}]}}
{"type":"result","result":"Done!","session_id":"sess-123","total_cost_usd":0.05,"duration_ms":1234}"#;
        let output = parse_claude_stream(stdout, "", 0).unwrap();

        assert_eq!(output.result, "Done!");
        assert_eq!(output.session_id.as_deref(), Some("sess-123"));
        assert_eq!(output.cost_usd, Some(0.05));
        assert_eq!(output.duration_ms, Some(1234));
        assert!(!output.is_error);

        // Check trace
        assert_eq!(output.trace.len(), 3);
        assert_eq!(output.trace[0].event_type, "thinking");
        assert_eq!(output.trace[1].event_type, "tool_use");
        assert_eq!(output.trace[1].tool_name, Some("Bash".to_string()));
        assert_eq!(output.trace[2].event_type, "tool_result");
    }

    #[test]
    fn parse_error_exit() {
        let output = parse_claude_stream("", "command not found", 127).unwrap();
        assert!(output.is_error);
        assert!(output.result.contains("127"));
        assert!(output.result.contains("command not found"));
    }

    #[test]
    fn parse_empty_stdout() {
        let output = parse_claude_stream("", "", 0).unwrap();
        assert!(output.result.is_empty());
        assert!(!output.is_error);
    }

    #[test]
    fn output_to_json_includes_trace() {
        let output = ClaudeOutput {
            result: "test".into(),
            session_id: Some("s1".into()),
            cost_usd: Some(0.01),
            duration_ms: Some(500),
            is_error: false,
            trace: vec![TraceEvent {
                event_type: "tool_use".into(),
                tool_name: Some("Bash".into()),
                tool_input: Some(serde_json::json!({"command": "ls"})),
                tool_result: None,
                thinking: None,
                text: None,
                is_error: None,
            }],
        };
        let json = output_to_json(&output);
        assert_eq!(json["result"], "test");
        assert_eq!(json["session_id"], "s1");
        assert!(json["trace"].is_array());
        assert_eq!(json["trace"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn parse_real_stream_output() {
    let stdout = r#"{"type":"system","subtype":"hook_started","hook_id":"test"}
{"type":"assistant","message":{"id":"msg1","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"Let me think about this"}]}}
{"type":"assistant","message":{"id":"msg2","type":"message","role":"assistant","content":[{"type":"tool_use","id":"call_1","name":"Bash","input":{"command":"ls -la"}}]}}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"file1.txt\nfile2.txt","is_error":false}]}}
{"type":"assistant","message":{"id":"msg3","type":"message","role":"assistant","content":[{"type":"text","text":"Done listing files"}]}}
{"type":"result","result":"Done listing files","session_id":"sess-123","total_cost_usd":0.05,"duration_ms":1234}"#;
    let output = parse_claude_stream(stdout, "", 0).unwrap();
    
    println!("Result: {}", output.result);
    println!("Session: {:?}", output.session_id);
    println!("Trace count: {}", output.trace.len());
    for (i, t) in output.trace.iter().enumerate() {
        println!("  [{}] type={}, tool_name={:?}", i, t.event_type, t.tool_name);
    }
    
    assert_eq!(output.result, "Done listing files");
    assert_eq!(output.session_id, Some("sess-123".to_string()));
    assert_eq!(output.trace.len(), 4, "Should have 4 trace events");
    assert_eq!(output.trace[0].event_type, "thinking");
    assert_eq!(output.trace[1].event_type, "tool_use");
    assert_eq!(output.trace[1].tool_name, Some("Bash".to_string()));
    assert_eq!(output.trace[2].event_type, "tool_result");
    assert_eq!(output.trace[3].event_type, "text");
}
