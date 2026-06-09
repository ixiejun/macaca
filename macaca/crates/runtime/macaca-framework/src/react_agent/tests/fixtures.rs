//! Test fixtures — mock LLM, echo/fail tools, and agent factory.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::formatter::OpenAiFormatter;
use crate::message::{ContentBlock, Msg, TextBlock, ToolUseBlock};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

use super::super::core::ReActAgent;


// -----------------------------------------------------------------------
// MockChatModel
// -----------------------------------------------------------------------

/// A mock model that returns pre-configured responses in order.
pub(crate) struct MockChatModel {
    responses: Arc<Mutex<Vec<ChatResponse>>>,
    /// Shared counter so tests can assert how many LLM turns occurred.
    pub(crate) call_count: Arc<AtomicUsize>,
    /// Captures `ChatOptions` passed on each `chat` call for policy contract tests.
    pub(crate) observed_options: Arc<Mutex<Vec<ChatOptions>>>,
}

impl MockChatModel {
    pub(crate) fn new(responses: Vec<ChatResponse>) -> Self {
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

pub(crate) fn text_response(text: &str) -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text(TextBlock { text: text.into() })],
        id: "r".into(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    }
}

pub(crate) fn tool_call_response(
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

pub(crate) struct EchoTool;

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
pub(crate) struct FailTool;

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

pub(crate) fn make_agent(model: MockChatModel) -> ReActAgent {
    ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    )
}
