//! Core agentic loop — the "heartbeat" of Agent OS agents.
//!
//! The loop sends messages to an LLM, checks for tool calls in the response,
//! executes the requested tools, feeds results back to the LLM, and repeats
//! until the LLM produces a final (non-tool-call) response or a safety limit is hit.

use std::time::Duration;

use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentId, MacacaError, MacacaResult, LlmMessage, LlmOptions, Permission,
    TokenUsage, ToolCall,
};
use macaca_tools::ToolSet;
use tracing::{debug, instrument, warn};

use crate::permission::PermissionChecker;

/// Configuration for the agentic runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of LLM round-trips before forcing a stop.
    pub max_iterations: usize,
    /// Timeout for a single tool execution.
    pub tool_timeout: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            tool_timeout: Duration::from_secs(60),
        }
    }
}

/// The result of an agentic loop execution.
#[derive(Debug, Clone)]
pub struct LoopResult {
    /// The final text response from the LLM.
    pub content: String,
    /// Total token usage across all iterations.
    pub total_usage: TokenUsage,
    /// Number of LLM round-trips performed.
    pub iterations: usize,
    /// The complete conversation history.
    pub messages: Vec<LlmMessage>,
}

/// The agentic execution loop.
///
/// Drives the LLM → tool → LLM cycle until the model produces a final
/// response or the iteration limit is reached.
pub struct AgenticLoop {
    config: RuntimeConfig,
}

impl AgenticLoop {
    pub fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }

    /// Run the agentic loop.
    ///
    /// # Arguments
    /// - `agent_id` — identity of the running agent (for permission checks)
    /// - `llm` — the LLM provider to call
    /// - `tools` — the tool set available to the agent
    /// - `initial_messages` — the starting conversation (system + user messages)
    /// - `options` — LLM options (model, temperature, etc.)
    /// - `permission` — the agent's permission policy
    /// - `permission_checker` — optional checker; if `None`, all tools are allowed
    #[instrument(name = "agentic_loop", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn run(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolSet,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> MacacaResult<LoopResult> {
        let mut messages = initial_messages;
        let mut total_usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        let mut iterations = 0;

        // Build LlmOptions with tool definitions injected.
        let options_with_tools = {
            let mut opts = options.clone();
            let defs = tools.to_definitions();
            if !defs.is_empty() {
                opts.tools = Some(defs);
            }
            opts
        };

        loop {
            iterations += 1;

            if iterations > self.config.max_iterations {
                warn!(
                    iterations,
                    max = self.config.max_iterations,
                    "Agentic loop hit max iterations — forcing stop"
                );
                // Return accumulated content from the last assistant message.
                let last_content = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                    .map(|m| m.content.clone())
                    .unwrap_or_default();

                return Ok(LoopResult {
                    content: last_content,
                    total_usage,
                    iterations: iterations - 1,
                    messages,
                });
            }

            debug!(iteration = iterations, "Sending request to LLM");

            // 1. Call LLM
            let response = llm.chat(messages.clone(), &options_with_tools).await?;
            accumulate_usage(&mut total_usage, &response.usage);

            // 2. Check for tool calls
            let tool_calls = response.tool_calls.clone().unwrap_or_default();

            // Append the assistant message (with or without tool_calls)
            if tool_calls.is_empty() {
                messages.push(LlmMessage::assistant(response.content.clone()));
            } else {
                messages.push(LlmMessage::assistant_with_tool_calls(
                    response.content.clone(),
                    tool_calls.clone(),
                ));
            }

            if tool_calls.is_empty() {
                // No tool calls — the model produced a final response.
                debug!(iteration = iterations, "LLM returned final response");
                return Ok(LoopResult {
                    content: response.content,
                    total_usage,
                    iterations,
                    messages,
                });
            }

            // 3. Execute each tool call
            debug!(
                iteration = iterations,
                count = tool_calls.len(),
                "Executing tool calls"
            );

            for tc in &tool_calls {
                let tool_result = self
                    .execute_tool_call(agent_id, tools, tc, permission, permission_checker)
                    .await;

                let result_value = match tool_result {
                    Ok(value) => serde_json::to_string(&value)
                        .unwrap_or_else(|_| value.to_string()),
                    Err(e) => format!("Error: {e}"),
                };

                // Append tool result message
                messages.push(LlmMessage::tool_result(&tc.id, result_value));
            }
        }
    }

    /// Execute a single tool call with permission checking and timeout.
    async fn execute_tool_call(
        &self,
        agent_id: &AgentId,
        tools: &dyn ToolSet,
        tool_call: &ToolCall,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> MacacaResult<serde_json::Value> {
        // Permission check
        if let Some(checker) = permission_checker {
            checker.check_tool_permission(agent_id, permission, &tool_call.name)?;
        }

        // Find tool
        let tool = tools.get_tool(&tool_call.name).ok_or_else(|| {
            MacacaError::NotFound(format!("Tool '{}' not found", tool_call.name))
        })?;

        // Execute with timeout
        let result = tokio::time::timeout(
            self.config.tool_timeout,
            tool.execute(tool_call.arguments.clone()),
        )
        .await
        .map_err(|_| {
            MacacaError::Timeout(format!(
                "Tool '{}' timed out after {}s",
                tool_call.name,
                self.config.tool_timeout.as_secs()
            ))
        })??;

        Ok(result)
    }
}

impl Default for AgenticLoop {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

/// Accumulate token usage across iterations.
fn accumulate_usage(total: &mut TokenUsage, delta: &TokenUsage) {
    total.prompt_tokens += delta.prompt_tokens;
    total.completion_tokens += delta.completion_tokens;
    total.total_tokens += delta.total_tokens;
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::LlmResponse;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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
            .run(
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
        });
        let llm = MockToolLlm::new();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![
            LlmMessage::system("You are helpful."),
            LlmMessage::user("Run echo hello"),
        ];

        let result = runner
            .run(
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
        });
        let llm = InfiniteToolLlm;
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![LlmMessage::user("Loop forever")];

        let result = runner
            .run(
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
            .run(
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
            .run(
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
}
