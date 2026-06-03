//! ReActAgent — Reasoning-Action loop implementation.
//!
//! The ReActAgent drives the core Think → Act → Observe cycle:
//! 1. **Reasoning**: call the LLM with the current memory context
//! 2. **Acting**: if the LLM requests tool calls, execute each one and store results
//! 3. Repeat until the LLM produces a plain text reply or `max_iters` is exceeded

use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    ContextAssembleInput, ContextEngineSelection, ContextFacade, ContextFacadeAssemblyPolicy,
};
use macaca_proto::AgentId;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::agent::{Agent, AgentError, AgentResult};
use crate::formatter::Formatter;
use crate::llm_wire::{
    chat_options_to_llm_options, llm_messages_to_json_values, messages_from_json_values,
};
use crate::memory::{CompressionConfig, InMemoryWorkingMemory, MemoryCompressor, WorkingMemory};
use crate::message::{ContentBlock, Msg, MsgContent, Role};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ToolChoice};
use crate::tool::Toolkit;

// ---------------------------------------------------------------------------
// ReActAgent
// ---------------------------------------------------------------------------

/// An agent that implements the ReAct (Reasoning + Acting) loop.
///
/// On each call to `reply`:
/// 1. The user message is stored in working memory.
/// 2. The LLM is called with the full memory context and available tools.
/// 3. If the LLM returns tool calls, each tool is executed and the result
///    is stored in memory before the next iteration.
/// 4. When the LLM returns a plain text response (no tool calls), that
///    response is returned as the agent's reply.
/// 5. If `max_iters` is reached before a final text response, the last
///    assistant message from memory is returned.
pub struct ReActAgent {
    name: String,
    id: AgentId,
    sys_prompt: String,
    model: Arc<dyn ChatModel>,
    formatter: Arc<dyn Formatter>,
    toolkit: Arc<Mutex<Toolkit>>,
    memory: Arc<Mutex<Box<dyn WorkingMemory>>>,
    max_iters: usize,
    cancel_token: CancellationToken,
    compressor: Option<MemoryCompressor>,
    /// Model name passed to ChatOptions (e.g. "qwen3-max", "gpt-4o").
    model_name: Option<String>,
    /// Optional provider-neutral tool-selection policy for every reasoning turn.
    ///
    /// The policy is intentionally generic: callers may require "some tool" for
    /// evidence-backed work, but this framework layer never branches on an
    /// application name, workflow name, business domain, or concrete provider.
    /// The actual tool catalog still comes from the caller-owned Toolkit and
    /// capability policy, so a required tool decision remains auditable through
    /// the same trace events as normal ReAct tool calls.
    tool_choice: Option<ToolChoice>,
}

impl ReActAgent {
    /// Create a new `ReActAgent` with default settings (no tools, in-memory store, 10 iterations).
    pub fn new(
        name: impl Into<String>,
        sys_prompt: impl Into<String>,
        model: Arc<dyn ChatModel>,
        formatter: Arc<dyn Formatter>,
    ) -> Self {
        Self {
            name: name.into(),
            id: AgentId::new(),
            sys_prompt: sys_prompt.into(),
            model,
            formatter,
            toolkit: Arc::new(Mutex::new(Toolkit::new())),
            memory: Arc::new(Mutex::new(Box::new(InMemoryWorkingMemory::new()))),
            max_iters: 10,
            cancel_token: CancellationToken::new(),
            compressor: None,
            model_name: None,
            tool_choice: None,
        }
    }

    /// Set the model name passed to the LLM provider (e.g. "qwen3-max").
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Set a provider-neutral tool-selection strategy for reasoning calls.
    ///
    /// This is used by higher-level execution policies that already know a
    /// durable side effect is required, for example an artifact-backed task that
    /// must write evidence before completion. The agent stores only the generic
    /// strategy and leaves all tool authorization to the Toolkit/middleware
    /// chain, preserving the OS/application boundary.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Replace the default (empty) toolkit with the given one.
    pub fn with_toolkit(mut self, toolkit: Toolkit) -> Self {
        self.toolkit = Arc::new(Mutex::new(toolkit));
        self
    }

    /// Replace the default in-memory working memory with the given one.
    pub fn with_memory(mut self, memory: Box<dyn WorkingMemory>) -> Self {
        self.memory = Arc::new(Mutex::new(memory));
        self
    }

    /// Set the maximum number of reasoning-action iterations before giving up.
    pub fn with_max_iters(mut self, max_iters: usize) -> Self {
        self.max_iters = max_iters;
        self
    }

    /// Enable automatic memory compression with the given configuration.
    pub fn with_compression(mut self, config: CompressionConfig) -> Self {
        self.compressor = Some(MemoryCompressor::new(config));
        self
    }

    /// Return a clone of the cancellation token.
    ///
    /// Callers can cancel the agent's current loop by calling `.cancel()` on
    /// the returned token (or by calling `interrupt` on the agent).
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Encodes working memory + system prompt as provider JSON, then runs
    /// `macaca_context::ContextFacade` (Composer → Engine) before calling the underlying
    /// `ChatModel`, sharing the same OS boundary as `macaca-runtime` / `macaca-web`.
    async fn reasoning(&self, _iteration: usize) -> AgentResult<ChatResponse> {
        let memory_msgs = {
            let mem = self.memory.lock().await;
            mem.get_with_summary().await
        };

        // Build message list: system prompt + memory contents
        let mut msgs = vec![Msg::system(self.sys_prompt.as_str())];
        msgs.extend(memory_msgs);

        // Format for the provider
        let formatted = self.formatter.format(&msgs);

        // Collect tool definitions
        let tool_defs = {
            let toolkit = self.toolkit.lock().await;
            toolkit.get_definitions()
        };
        let has_tools = !tool_defs.is_empty();

        let options = ChatOptions {
            model: self.model_name.clone(),
            tools: if has_tools { Some(tool_defs) } else { None },
            tool_choice: if has_tools {
                self.tool_choice.clone()
            } else {
                None
            },
            ..Default::default()
        };

        tracing::debug!(
            agent = %self.name,
            tool_count = options.tools.as_ref().map(|tools| tools.len()).unwrap_or_default(),
            tool_choice = ?options.tool_choice,
            "react_agent.reasoning prepared provider-neutral tool policy"
        );

        let llm_opts = chat_options_to_llm_options(&options);
        let base_messages = messages_from_json_values(&formatted);
        let facade_input = ContextAssembleInput::legacy(
            self.name.clone(),
            llm_opts.model.clone(),
            base_messages,
            llm_opts,
        );
        let assembled = ContextFacade::builtins(ContextEngineSelection::legacy())
            .assemble_model_context(facade_input, &[], ContextFacadeAssemblyPolicy::default())
            .await
            .map_err(|e| AgentError::Llm(e.to_string()))?;
        let chat_payload = llm_messages_to_json_values(&assembled.messages);

        // Call the LLM with OS-assembled messages (legacy equivalent when provider list is empty).
        let response = self
            .model
            .chat(chat_payload, &options)
            .await
            .map_err(|e| AgentError::Llm(e.to_string()))?;

        // Persist the assistant's response into memory
        let assistant_content = response_to_content(&response);
        let assistant_msg = Msg::assistant(&self.name, assistant_content);
        {
            let mut mem = self.memory.lock().await;
            mem.add(assistant_msg, vec![]).await;
        }

        Ok(response)
    }

    /// Acting step: execute a single tool call and store the result in memory.
    async fn acting(&self, tool_call: &crate::message::ToolUseBlock) -> AgentResult<()> {
        let result = {
            let toolkit = self.toolkit.lock().await;
            toolkit
                .call_tool(&tool_call.name, tool_call.input.clone())
                .await
        };

        let (output, is_error) = match result {
            Ok(response) => {
                let text = response
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                (text, false)
            }
            Err(e) => (format!("Error: {e}"), true),
        };

        let result_msg = Msg::tool_result(&tool_call.id, &tool_call.name, output, is_error);
        {
            let mut mem = self.memory.lock().await;
            mem.add(result_msg, vec![]).await;
        }

        Ok(())
    }

    /// Summarize: return the last assistant message from memory (used when
    /// `max_iters` is exhausted without a tool-free response).
    async fn summarize(&self) -> AgentResult<Msg> {
        let mem = self.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        let last_content = msgs
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .map(|m| m.get_text())
            .unwrap_or_else(|| "Max iterations reached.".to_string());
        Ok(Msg::assistant(&self.name, last_content))
    }
}

// ---------------------------------------------------------------------------
// Agent trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Agent for ReActAgent {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        // Store the incoming user message
        {
            let mut mem = self.memory.lock().await;
            mem.add(msg, vec![]).await;
        }

        // Compress memory if configured and needed
        if let Some(ref compressor) = self.compressor {
            let mut mem = self.memory.lock().await;
            let _ = compressor
                .compress_if_needed(
                    mem.as_mut(),
                    self.model.as_ref(),
                    self.formatter.as_ref(),
                    &self.sys_prompt,
                )
                .await;
            // Ignore compression errors — don't fail the reply
        }

        for _iteration in 0..self.max_iters {
            // Check for cancellation before each reasoning step
            if self.cancel_token.is_cancelled() {
                return Err(AgentError::Interrupted);
            }

            // Reasoning: call the LLM
            let response = self.reasoning(_iteration).await?;

            // If no tool calls → this is the final response
            if !response.has_tool_calls() {
                let text = response.get_text();
                let reply = Msg::assistant(&self.name, text);
                // The assistant message was already stored by `reasoning`; no
                // need to store it again. Return the reply directly.
                return Ok(reply);
            }

            // Acting: execute every requested tool call
            for tool_call in response.get_tool_calls() {
                // Check cancellation between tool calls as well
                if self.cancel_token.is_cancelled() {
                    return Err(AgentError::Interrupted);
                }
                self.acting(tool_call).await?;
            }
        }

        // Exceeded max iterations — return whatever the last assistant said
        self.summarize().await
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        let mut mem = self.memory.lock().await;
        mem.add(msg, vec!["observed".to_string()]).await;
        Ok(())
    }

    async fn interrupt(&self, _msg: Msg) -> AgentResult<()> {
        self.cancel_token.cancel();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `ChatResponse` into a `MsgContent` suitable for storing in memory.
fn response_to_content(response: &ChatResponse) -> MsgContent {
    if response.content.is_empty() {
        MsgContent::Text(String::new())
    } else if response.content.len() == 1 {
        match response.content.first() {
            Some(ContentBlock::Text(t)) => MsgContent::Text(t.text.clone()),
            _ => MsgContent::Blocks(response.content.clone()),
        }
    } else {
        MsgContent::Blocks(response.content.clone())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::Mutex;

    use crate::formatter::OpenAiFormatter;
    use crate::message::{TextBlock, ToolUseBlock};
    use crate::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};
    use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

    // -----------------------------------------------------------------------
    // MockChatModel
    // -----------------------------------------------------------------------

    /// A mock model that returns pre-configured responses in order.
    struct MockChatModel {
        responses: Arc<Mutex<Vec<ChatResponse>>>,
        call_count: Arc<AtomicUsize>,
        observed_options: Arc<Mutex<Vec<ChatOptions>>>,
    }

    impl MockChatModel {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
                call_count: Arc::new(AtomicUsize::new(0)),
                observed_options: Arc::new(Mutex::new(Vec::new())),
            }
        }

        #[allow(dead_code)]
        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ChatModel for MockChatModel {
        async fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            options: &ChatOptions,
        ) -> Result<ChatResponse, ModelError> {
            self.observed_options.lock().await.push(options.clone());
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            let responses = self.responses.lock().await;
            responses
                .get(idx)
                .cloned()
                .ok_or_else(|| ModelError::Other("no more responses".into()))
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    // -----------------------------------------------------------------------
    // Helpers to build responses
    // -----------------------------------------------------------------------

    fn text_response(text: &str) -> ChatResponse {
        ChatResponse {
            content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
            id: "r".into(),
            created_at: String::new(),
            usage: ChatUsage::default(),
            metadata: None,
        }
    }

    fn tool_call_response(
        tool_id: &str,
        tool_name: &str,
        input: serde_json::Value,
    ) -> ChatResponse {
        ChatResponse {
            content: vec![ContentBlock::ToolUse(ToolUseBlock {
                id: tool_id.into(),
                name: tool_name.into(),
                input,
                raw_input: None,
            })],
            id: "r".into(),
            created_at: String::new(),
            usage: ChatUsage::default(),
            metadata: None,
        }
    }

    // -----------------------------------------------------------------------
    // EchoTool — returns input as text
    // -----------------------------------------------------------------------

    struct EchoTool;

    #[async_trait]
    impl ToolHandler for EchoTool {
        async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::text(args.to_string()))
        }
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo input"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
    }

    // -----------------------------------------------------------------------
    // FailTool — always returns an error
    // -----------------------------------------------------------------------

    #[allow(dead_code)]
    struct FailTool;

    #[async_trait]
    impl ToolHandler for FailTool {
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResponse, ToolError> {
            Err(ToolError::ExecutionFailed("deliberate failure".into()))
        }
        fn name(&self) -> &str {
            "fail"
        }
        fn description(&self) -> &str {
            "Always fails"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
    }

    fn make_agent(model: MockChatModel) -> ReActAgent {
        ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(model),
            Arc::new(OpenAiFormatter),
        )
    }

    // -----------------------------------------------------------------------
    // 1. test_simple_reply: no tool calls → returns text directly
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_simple_reply() {
        let model = MockChatModel::new(vec![text_response("Hello!")]);
        let agent = make_agent(model);
        let msg = Msg::user("alice", "hi");
        let reply = agent.reply(msg).await.unwrap();
        assert_eq!(reply.get_text(), "Hello!");
        assert_eq!(reply.name, "bot");
        assert_eq!(reply.role, Role::Assistant);
    }

    // -----------------------------------------------------------------------
    // 2. test_tool_call_cycle: first call returns tool request, second returns text
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_call_cycle() {
        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let model2 = MockChatModel::new(vec![
            tool_call_response("call_1", "echo", serde_json::json!({"x": 1})),
            text_response("Done!"),
        ]);
        let model_call_count = model2.call_count.clone();

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(model2),
            Arc::new(OpenAiFormatter),
        )
        .with_toolkit(toolkit);

        let reply = agent.reply(Msg::user("alice", "run echo")).await.unwrap();
        assert_eq!(reply.get_text(), "Done!");
        // LLM was called twice: once for tool call, once for final reply
        assert_eq!(model_call_count.load(Ordering::SeqCst), 2);

        // Verify memory contains: user msg, assistant tool-call msg, tool result, assistant final
        let mem = agent.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        // user + assistant(tool_call) + tool_result + assistant(final)
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].role, Role::User);
        assert_eq!(msgs[1].role, Role::Assistant); // tool call stored
        assert_eq!(msgs[2].role, Role::Tool); // tool result
        assert_eq!(msgs[3].role, Role::Assistant); // final reply
    }

    #[tokio::test]
    async fn test_tool_choice_is_forwarded_when_tools_are_available() {
        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let model = MockChatModel::new(vec![text_response("tool policy observed")]);
        let observed_options = Arc::clone(&model.observed_options);

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(model),
            Arc::new(OpenAiFormatter),
        )
        .with_toolkit(toolkit)
        .with_tool_choice(ToolChoice::Required);

        let reply = agent
            .reply(Msg::user("alice", "write evidence"))
            .await
            .unwrap();
        assert_eq!(reply.get_text(), "tool policy observed");

        let options = observed_options.lock().await;
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].tool_choice, Some(ToolChoice::Required));
        assert!(options[0]
            .tools
            .as_ref()
            .is_some_and(|tools| !tools.is_empty()));
    }

    #[tokio::test]
    async fn test_tool_choice_is_suppressed_without_tools() {
        let model = MockChatModel::new(vec![text_response("no tools")]);
        let observed_options = Arc::clone(&model.observed_options);

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(model),
            Arc::new(OpenAiFormatter),
        )
        .with_tool_choice(ToolChoice::Required);

        let reply = agent
            .reply(Msg::user("alice", "plain answer"))
            .await
            .unwrap();
        assert_eq!(reply.get_text(), "no tools");

        let options = observed_options.lock().await;
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].tool_choice, None);
        assert!(options[0].tools.is_none());
    }

    // -----------------------------------------------------------------------
    // 3. test_max_iterations: always returns tool calls → summarize after max_iters
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_max_iterations() {
        // Build 5 tool-call responses (max_iters = 3, so only first 3 are consumed)
        let responses: Vec<ChatResponse> = (0..5)
            .map(|i| tool_call_response(&format!("call_{i}"), "echo", serde_json::json!({})))
            .collect();

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let model = MockChatModel::new(responses);
        let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
            .with_toolkit(toolkit)
            .with_max_iters(3);

        // Should not error — returns summarized last assistant message
        let reply = agent.reply(Msg::user("alice", "go")).await.unwrap();
        assert_eq!(reply.role, Role::Assistant);
        assert_eq!(reply.name, "bot");
    }

    // -----------------------------------------------------------------------
    // 4. test_interrupt: calling interrupt causes next iteration to return Interrupted
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_interrupt() {
        // Provide many tool-call responses so the loop keeps running
        let responses: Vec<ChatResponse> = (0..10)
            .map(|i| tool_call_response(&format!("c_{i}"), "echo", serde_json::json!({})))
            .collect();

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let model = MockChatModel::new(responses);
        let agent = Arc::new(
            ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
                .with_toolkit(toolkit)
                .with_max_iters(10),
        );

        // Cancel immediately before the loop gets a chance to finish
        agent.cancel_token().cancel();

        let result = agent.reply(Msg::user("alice", "go")).await;
        assert!(matches!(result, Err(AgentError::Interrupted)));
    }

    // -----------------------------------------------------------------------
    // 5. test_observe_stores_message
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_observe_stores_message() {
        let model = MockChatModel::new(vec![]);
        let agent = make_agent(model);

        let observed = Msg::user("external", "external message");
        agent.observe(observed.clone()).await.unwrap();

        let mem = agent.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, observed.id);

        // Observed messages carry the "observed" mark
        let marked = mem.get_memory(Some("observed"), None).await;
        assert_eq!(marked.len(), 1);
    }

    // -----------------------------------------------------------------------
    // 6. test_memory_persists_across_calls
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_memory_persists_across_calls() {
        let model = MockChatModel::new(vec![
            text_response("first reply"),
            text_response("second reply"),
        ]);
        let agent = make_agent(model);

        agent.reply(Msg::user("alice", "msg1")).await.unwrap();
        agent.reply(Msg::user("alice", "msg2")).await.unwrap();

        let mem = agent.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        // msg1 + assistant1 + msg2 + assistant2 = 4 messages
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].get_text(), "msg1");
        assert_eq!(msgs[1].get_text(), "first reply");
        assert_eq!(msgs[2].get_text(), "msg2");
        assert_eq!(msgs[3].get_text(), "second reply");
    }

    // -----------------------------------------------------------------------
    // 7. test_tool_returns_error_feeds_back_to_llm
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_returns_error_feeds_back_to_llm() {
        // First call: LLM requests "fail" tool → tool returns ToolError
        // Second call: LLM returns text (after seeing the error feedback)
        let model = MockChatModel::new(vec![
            tool_call_response("call_1", "fail", serde_json::json!({})),
            text_response("I see the error, here is my answer."),
        ]);

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(FailTool), None);

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(model),
            Arc::new(OpenAiFormatter),
        )
        .with_toolkit(toolkit);

        let reply = agent
            .reply(Msg::user("alice", "do something"))
            .await
            .unwrap();
        assert_eq!(reply.get_text(), "I see the error, here is my answer.");

        // Verify memory contains the error tool result with is_error
        let mem = agent.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        // user + assistant(tool_call) + tool_result(error) + assistant(final)
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[2].role, Role::Tool);
        // The tool result should contain the error text
        let tool_text = msgs[2].get_text();
        assert!(tool_text.is_empty() || true); // ToolResult has output in block, not get_text
                                               // Check the actual ToolResult block
        if let MsgContent::Blocks(blocks) = &msgs[2].content {
            if let ContentBlock::ToolResult(ref r) = blocks[0] {
                assert!(r.is_error);
                assert!(r.output.contains("deliberate failure"));
            } else {
                panic!("Expected ToolResult block");
            }
        } else {
            panic!("Expected Blocks content");
        }
    }

    // -----------------------------------------------------------------------
    // 8. test_max_iters_one_boundary
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_max_iters_one_boundary() {
        // max_iters=1: LLM returns a tool call, only 1 iteration allowed,
        // then summarize should be called
        let model = MockChatModel::new(vec![tool_call_response(
            "call_1",
            "echo",
            serde_json::json!({}),
        )]);

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
            .with_toolkit(toolkit)
            .with_max_iters(1);

        let reply = agent.reply(Msg::user("alice", "go")).await.unwrap();
        assert_eq!(reply.role, Role::Assistant);
        assert_eq!(reply.name, "bot");
    }

    // -----------------------------------------------------------------------
    // 9. test_interrupt_during_execution
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_interrupt_during_execution() {
        // Cancel the token before calling reply → should return Interrupted
        let responses: Vec<ChatResponse> = (0..5)
            .map(|i| tool_call_response(&format!("c_{i}"), "echo", serde_json::json!({})))
            .collect();

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(EchoTool), None);

        let model = MockChatModel::new(responses);
        let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
            .with_toolkit(toolkit)
            .with_max_iters(10);

        // Use the CancellationToken and cancel it
        let token = agent.cancel_token();
        token.cancel();

        let result = agent.reply(Msg::user("alice", "go")).await;
        assert!(matches!(result, Err(AgentError::Interrupted)));
    }

    // -----------------------------------------------------------------------
    // 10. test_consecutive_replies_accumulate_memory
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_consecutive_replies_accumulate_memory() {
        let model = MockChatModel::new(vec![
            text_response("reply1"),
            text_response("reply2"),
            text_response("reply3"),
        ]);
        let agent = make_agent(model);

        agent.reply(Msg::user("alice", "msg1")).await.unwrap();
        {
            let mem = agent.memory.lock().await;
            assert_eq!(mem.size().await, 2); // user + assistant
        }

        agent.reply(Msg::user("alice", "msg2")).await.unwrap();
        {
            let mem = agent.memory.lock().await;
            assert_eq!(mem.size().await, 4);
        }

        agent.reply(Msg::user("alice", "msg3")).await.unwrap();
        {
            let mem = agent.memory.lock().await;
            assert_eq!(mem.size().await, 6);
        }
    }

    // -----------------------------------------------------------------------
    // 11. test_llm_returns_empty_content
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_llm_returns_empty_content() {
        // LLM returns a response with empty content (no text, no tool calls)
        let empty_response = ChatResponse {
            content: vec![],
            id: "r".into(),
            created_at: String::new(),
            usage: ChatUsage::default(),
            metadata: None,
        };
        let model = MockChatModel::new(vec![empty_response]);
        let agent = make_agent(model);

        let reply = agent.reply(Msg::user("alice", "hi")).await.unwrap();
        // Empty content → has_tool_calls() is false → loop ends, returns empty text
        assert_eq!(reply.get_text(), "");
        assert_eq!(reply.role, Role::Assistant);
    }

    // -----------------------------------------------------------------------
    // 12. test_llm_returns_error
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_llm_returns_error() {
        // MockChatModel with no responses → returns ModelError::Other
        let model = MockChatModel::new(vec![]);
        let agent = make_agent(model);

        let result = agent.reply(Msg::user("alice", "hi")).await;
        assert!(matches!(result, Err(AgentError::Llm(_))));
    }

    // -----------------------------------------------------------------------
    // 13. test_tool_not_found_handling
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_not_found_handling() {
        // LLM requests a tool that doesn't exist in the toolkit.
        // The acting() method calls toolkit.call_tool() which returns ToolError::NotFound.
        // That error is formatted and stored as a ToolResult with is_error=true.
        // Then the next LLM call returns text.
        let model = MockChatModel::new(vec![
            tool_call_response("call_1", "nonexistent_tool", serde_json::json!({})),
            text_response("Ok, the tool was not found."),
        ]);

        // Empty toolkit — no tools registered
        let agent = make_agent(model);

        let reply = agent.reply(Msg::user("alice", "use tool")).await.unwrap();
        assert_eq!(reply.get_text(), "Ok, the tool was not found.");

        // Verify the error tool result is in memory
        let mem = agent.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        assert_eq!(msgs.len(), 4); // user + assistant(tool) + tool_result(error) + assistant(final)
        assert_eq!(msgs[2].role, Role::Tool);
        if let MsgContent::Blocks(blocks) = &msgs[2].content {
            if let ContentBlock::ToolResult(ref r) = blocks[0] {
                assert!(r.is_error);
                assert!(r.output.contains("nonexistent_tool"));
            } else {
                panic!("Expected ToolResult block");
            }
        } else {
            panic!("Expected Blocks content");
        }
    }

    // -----------------------------------------------------------------------
    // 14. test_system_prompt_position
    // -----------------------------------------------------------------------

    /// A MockChatModel that captures the messages it receives.
    struct CapturingMockModel {
        responses: Arc<Mutex<Vec<ChatResponse>>>,
        call_count: Arc<AtomicUsize>,
        captured_messages: Arc<Mutex<Vec<Vec<serde_json::Value>>>>,
    }

    impl CapturingMockModel {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
                call_count: Arc::new(AtomicUsize::new(0)),
                captured_messages: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl ChatModel for CapturingMockModel {
        async fn chat(
            &self,
            messages: Vec<serde_json::Value>,
            _options: &ChatOptions,
        ) -> Result<ChatResponse, ModelError> {
            self.captured_messages.lock().await.push(messages.clone());
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            let responses = self.responses.lock().await;
            responses
                .get(idx)
                .cloned()
                .ok_or_else(|| ModelError::Other("no more responses".into()))
        }

        fn name(&self) -> &str {
            "capturing-mock"
        }
    }

    #[tokio::test]
    async fn test_system_prompt_position() {
        let capturing_model = CapturingMockModel::new(vec![text_response("ok")]);
        let captured = capturing_model.captured_messages.clone();

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(capturing_model),
            Arc::new(OpenAiFormatter),
        );

        agent.reply(Msg::user("alice", "hi")).await.unwrap();

        let calls = captured.lock().await;
        assert_eq!(calls.len(), 1);
        let first_call_msgs = &calls[0];
        // The first message must be the system prompt
        assert_eq!(first_call_msgs[0]["role"], "system");
        assert_eq!(first_call_msgs[0]["content"], "You are helpful.");
    }

    // -----------------------------------------------------------------------
    // 15. test_observe_then_reply_memory
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_observe_then_reply_memory() {
        let capturing_model =
            CapturingMockModel::new(vec![text_response("I saw your observation.")]);
        let captured = capturing_model.captured_messages.clone();

        let agent = ReActAgent::new(
            "bot",
            "You are helpful.",
            Arc::new(capturing_model),
            Arc::new(OpenAiFormatter),
        );

        // Observe a message first
        let observed_msg = Msg::user("external", "context information");
        agent.observe(observed_msg).await.unwrap();

        // Now reply
        agent
            .reply(Msg::user("alice", "what do you know?"))
            .await
            .unwrap();

        let calls = captured.lock().await;
        assert_eq!(calls.len(), 1);
        let msgs_sent = &calls[0];
        // System prompt + observed msg + user msg = 3
        assert_eq!(msgs_sent.len(), 3);
        assert_eq!(msgs_sent[0]["role"], "system");
        // The observed message should appear before the user's reply message
        assert_eq!(msgs_sent[1]["content"], "context information");
        assert_eq!(msgs_sent[2]["content"], "what do you know?");
    }
}
