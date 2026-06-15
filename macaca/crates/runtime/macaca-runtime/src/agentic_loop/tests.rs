//! Integration tests for the agentic loop (mock LLM, no live providers).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, LlmResponse, MacacaResult, Permission,
    TokenUsage, ToolCall,
};
use tokio::sync::mpsc;

use super::helpers::accumulate_usage;
use super::{AgenticLoop, RuntimeConfig};

// ── Mock LLM: returns tool calls on first call, then a final response ──

struct MockToolLlm {
    call_count: Arc<AtomicUsize>,
}

impl MockToolLlm {
    fn new() -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl LlmProvider for MockToolLlm {
    fn name(&self) -> &str {
        "mock-tool-llm"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);

        if n == 0 {
            // First call: request a tool call
            Ok(LlmResponse {
                content: String::new(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: "tool_calls".into(),
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".into(),
                    name: "shell".into(),
                    arguments: serde_json::json!({ "command": "echo hello" }),
                }]),
            })
        } else {
            // Second call: final response
            Ok(LlmResponse {
                content: "Done! The command output was: hello".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 20,
                    completion_tokens: 10,
                    total_tokens: 30,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }
}

// ── Mock LLM: always requests tool calls (for max iteration test) ──

struct InfiniteToolLlm;

#[async_trait]
impl LlmProvider for InfiniteToolLlm {
    fn name(&self) -> &str {
        "infinite-tool-llm"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "thinking...".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 5,
                completion_tokens: 5,
                total_tokens: 10,
            },
            finish_reason: "tool_calls".into(),
            tool_calls: Some(vec![ToolCall {
                id: "call_loop".into(),
                name: "shell".into(),
                arguments: serde_json::json!({ "command": "echo loop" }),
            }]),
        })
    }
}

// ── Mock LLM: no tool calls at all ──

struct DirectResponseLlm;

#[async_trait]
impl LlmProvider for DirectResponseLlm {
    fn name(&self) -> &str {
        "direct-response-llm"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "Direct answer: 42".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

fn default_permission() -> Permission {
    Permission {
        level: macaca_proto::PermissionLevel::User,
        allowed_tools: vec![],
        allowed_paths: vec![],
        network_access: true,
    }
}

#[tokio::test]
async fn direct_response_no_tool_calls() {
    let runner = AgenticLoop::default();
    let llm = DirectResponseLlm;
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();
    let messages = vec![
        LlmMessage::system("You are helpful."),
        LlmMessage::user("What is 6*7?"),
    ];

    let result = runner
        .execute(
            &agent_id,
            &llm,
            &tools,
            messages,
            &LlmOptions::default(),
            &default_permission(),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.content, "Direct answer: 42");
    assert_eq!(result.iterations, 1);
    assert_eq!(result.total_usage.total_tokens, 15);
}

#[tokio::test]
async fn tool_call_then_final_response() {
    let runner = AgenticLoop::new(RuntimeConfig {
        max_iterations: 10,
        tool_timeout: Duration::from_secs(5),
        ..RuntimeConfig::default()
    });
    let llm = MockToolLlm::new();
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();
    let messages = vec![
        LlmMessage::system("You are helpful."),
        LlmMessage::user("Run echo hello"),
    ];

    let result = runner
        .execute(
            &agent_id,
            &llm,
            &tools,
            messages,
            &LlmOptions::default(),
            &default_permission(),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.content, "Done! The command output was: hello");
    assert_eq!(result.iterations, 2);
    assert_eq!(result.total_usage.total_tokens, 45); // 15 + 30
                                                     // Messages should contain: system, user, assistant(tool_call), tool_result, assistant(final)
    assert_eq!(result.messages.len(), 5);
}

#[tokio::test]
async fn max_iterations_stops_loop() {
    let runner = AgenticLoop::new(RuntimeConfig {
        max_iterations: 3,
        tool_timeout: Duration::from_secs(5),
        ..RuntimeConfig::default()
    });
    let llm = InfiniteToolLlm;
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();
    let messages = vec![LlmMessage::user("Loop forever")];

    let result = runner
        .execute(
            &agent_id,
            &llm,
            &tools,
            messages,
            &LlmOptions::default(),
            &default_permission(),
            None,
        )
        .await
        .unwrap();

    // Should stop at max_iterations
    assert_eq!(result.iterations, 3);
    assert_eq!(result.content, "thinking...");
}

#[tokio::test]
async fn permission_denied_returns_error_in_tool_result() {
    use crate::permission::DefaultPermissionChecker;

    let runner = AgenticLoop::new(RuntimeConfig {
        max_iterations: 5,
        tool_timeout: Duration::from_secs(5),
        ..RuntimeConfig::default()
    });
    let llm = MockToolLlm::new();
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();
    let messages = vec![LlmMessage::user("Run shell")];

    // Only allow file_read — shell should be denied
    let permission = Permission {
        level: macaca_proto::PermissionLevel::User,
        allowed_tools: vec!["file_read".into()],
        allowed_paths: vec![],
        network_access: false,
    };
    let checker = DefaultPermissionChecker;

    let result = runner
        .execute(
            &agent_id,
            &llm,
            &tools,
            messages,
            &LlmOptions::default(),
            &permission,
            Some(&checker),
        )
        .await
        .unwrap();

    // The loop should have continued: permission error fed back as tool result,
    // then the LLM gave a final response on iteration 2.
    assert_eq!(result.iterations, 2);
    // Check that the tool result message contains the permission error
    let tool_msg = result
        .messages
        .iter()
        .find(|m| m.role == macaca_proto::LlmRole::Tool)
        .expect("should have a Tool role message");
    assert!(tool_msg.content.contains("not allowed"));
}

#[tokio::test]
async fn tool_not_found_returns_error_in_tool_result() {
    // LLM requests a tool that doesn't exist
    struct UnknownToolLlm;

    #[async_trait]
    impl LlmProvider for UnknownToolLlm {
        fn name(&self) -> &str {
            "unknown-tool-llm"
        }

        async fn chat(
            &self,
            messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            // Only request unknown tool on first call
            let has_tool_result = messages
                .iter()
                .any(|m| m.role == macaca_proto::LlmRole::Tool);

            if !has_tool_result {
                Ok(LlmResponse {
                    content: String::new(),
                    reasoning_content: None,
                    model: "mock".into(),
                    usage: TokenUsage::default(),
                    finish_reason: "tool_calls".into(),
                    tool_calls: Some(vec![ToolCall {
                        id: "call_x".into(),
                        name: "nonexistent_tool".into(),
                        arguments: serde_json::json!({}),
                    }]),
                })
            } else {
                Ok(LlmResponse {
                    content: "Tool not found, giving up.".into(),
                    reasoning_content: None,
                    model: "mock".into(),
                    usage: TokenUsage::default(),
                    finish_reason: "stop".into(),
                    tool_calls: None,
                })
            }
        }
    }

    let runner = AgenticLoop::default();
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();

    let result = runner
        .execute(
            &agent_id,
            &UnknownToolLlm,
            &tools,
            vec![LlmMessage::user("Use nonexistent tool")],
            &LlmOptions::default(),
            &default_permission(),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.iterations, 2);
    let tool_msg = result
        .messages
        .iter()
        .find(|m| m.role == macaca_proto::LlmRole::Tool)
        .unwrap();
    assert!(tool_msg.content.contains("not found"));
}

#[tokio::test]
async fn execute_with_events_preserves_event_order() {
    let runner = AgenticLoop::new(RuntimeConfig {
        max_iterations: 5,
        tool_timeout: Duration::from_secs(5),
        ..RuntimeConfig::default()
    });
    let llm = MockToolLlm::new();
    let tools = macaca_tools::DefaultToolSet::new();
    let agent_id = AgentId::new();
    let (tx, mut rx) = mpsc::channel(16);

    let result = runner
        .execute_with_events(
            &agent_id,
            &llm,
            &tools,
            vec![LlmMessage::user("Run echo hello")],
            &LlmOptions::default(),
            &default_permission(),
            None,
            Some(tx),
        )
        .await
        .unwrap();

    assert_eq!(result.iterations, 2);

    let mut kinds = Vec::new();
    while let Some(event) = rx.recv().await {
        let kind = match event {
            AgentExecutionEvent::Thinking { .. } => "thinking",
            AgentExecutionEvent::ToolCall { .. } => "tool_call",
            AgentExecutionEvent::DriverTrace { trace, .. }
                if trace.get("event_type").and_then(|value| value.as_str())
                    == Some("context_report") =>
            {
                continue;
            }
            AgentExecutionEvent::DriverTrace { .. } => "driver_trace",
            AgentExecutionEvent::ToolResult { .. } => "tool_result",
            AgentExecutionEvent::Assistant { .. } => "assistant",
            AgentExecutionEvent::Completed { .. } => "completed",
        };
        kinds.push(kind);
    }

    assert_eq!(
        kinds,
        vec![
            "thinking",
            "tool_call",
            "driver_trace",
            "driver_trace",
            "tool_result",
            "thinking",
            "assistant",
            "completed"
        ]
    );
}

#[test]
fn runtime_config_default() {
    let config = RuntimeConfig::default();
    assert_eq!(config.max_iterations, 25);
    assert_eq!(config.tool_timeout, Duration::from_secs(60));
}

#[test]
fn accumulate_usage_test() {
    let mut total = TokenUsage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
    };
    let delta = TokenUsage {
        prompt_tokens: 20,
        completion_tokens: 10,
        total_tokens: 30,
    };
    accumulate_usage(&mut total, &delta);
    assert_eq!(total.prompt_tokens, 30);
    assert_eq!(total.completion_tokens, 15);
    assert_eq!(total.total_tokens, 45);
}
