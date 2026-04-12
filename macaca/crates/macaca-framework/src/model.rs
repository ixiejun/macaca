//! LLM model abstraction and response types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::message::{ContentBlock, ThinkingBlock, ToolUseBlock};

// ---------------------------------------------------------------------------
// Usage & Response
// ---------------------------------------------------------------------------

/// Token usage statistics for a model call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub duration_ms: Option<u64>,
}

/// Unified model response returned by all `ChatModel` implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Parsed content blocks from the model's output.
    pub content: Vec<ContentBlock>,
    /// Provider-assigned response ID.
    pub id: String,
    /// ISO-8601 creation timestamp as returned by the provider.
    pub created_at: String,
    /// Token usage for this call.
    pub usage: ChatUsage,
    /// Optional provider-specific metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ChatResponse {
    /// Extract and join all `TextBlock` texts in order.
    pub fn get_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Collect references to all `ToolUseBlock`s.
    pub fn get_tool_calls(&self) -> Vec<&ToolUseBlock> {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse(t) => Some(t),
                _ => None,
            })
            .collect()
    }

    /// Returns `true` if the response contains at least one tool call.
    pub fn has_tool_calls(&self) -> bool {
        self.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse(_)))
    }

    /// Collect references to all `ThinkingBlock`s.
    pub fn get_thinking(&self) -> Vec<&ThinkingBlock> {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Thinking(t) => Some(t),
                _ => None,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// ChatOptions
// ---------------------------------------------------------------------------

/// Options passed to a `ChatModel::chat` call.
///
/// All fields are optional; providers apply their own defaults when a field
/// is `None`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    /// Override the model name (e.g. `"gpt-4o"`, `"qwen3-max"`).
    pub model: Option<String>,
    /// Sampling temperature (0.0 – 2.0).
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Nucleus sampling probability (0.0 – 1.0).
    pub top_p: Option<f32>,
    /// Tool definitions in JSON Schema format (provider-specific).
    pub tools: Option<Vec<serde_json::Value>>,
    /// Tool choice strategy.
    pub tool_choice: Option<ToolChoice>,
}

/// Controls how the model selects tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model decides whether to call a tool.
    Auto,
    /// Model must not call any tool.
    None,
    /// Model must call at least one tool.
    Required,
    /// Model must call the named tool.
    Specific(String),
}

// ---------------------------------------------------------------------------
// ChatModel trait
// ---------------------------------------------------------------------------

/// Errors that can be returned by a `ChatModel`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModelError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Timeout")]
    Timeout,
    #[error("Network error: {0}")]
    Network(String),
    #[error("{0}")]
    Other(String),
}

/// Core trait for LLM model providers.
///
/// The `messages` argument is already serialized into provider-specific JSON
/// by a `Formatter` before being passed here, so implementors only need to
/// make the HTTP call and return a `ChatResponse`.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Send messages to the model and await a response.
    ///
    /// `messages` is a list of provider-formatted message objects (as
    /// produced by `Formatter::format`).
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError>;

    /// Human-readable provider name (e.g. `"openai"`, `"anthropic"`).
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{TextBlock, ToolUseBlock};

    fn make_response(blocks: Vec<ContentBlock>) -> ChatResponse {
        ChatResponse {
            content: blocks,
            id: "resp-1".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            usage: ChatUsage::default(),
            metadata: None,
        }
    }

    #[test]
    fn test_get_text_joins_text_blocks() {
        let resp = make_response(vec![
            ContentBlock::Text(TextBlock {
                text: "Hello, ".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "world!".into(),
            }),
        ]);
        assert_eq!(resp.get_text(), "Hello, world!");
    }

    #[test]
    fn test_get_text_skips_non_text() {
        let resp = make_response(vec![
            ContentBlock::Thinking(crate::message::ThinkingBlock {
                thinking: "hmm".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "answer".into(),
            }),
        ]);
        assert_eq!(resp.get_text(), "answer");
    }

    #[test]
    fn test_get_tool_calls() {
        let tool = ToolUseBlock {
            id: "call_1".into(),
            name: "search".into(),
            input: serde_json::json!({"q": "rust"}),
            raw_input: None,
        };
        let resp = make_response(vec![
            ContentBlock::Text(TextBlock { text: "ok".into() }),
            ContentBlock::ToolUse(tool.clone()),
        ]);
        let calls = resp.get_tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search");
    }

    #[test]
    fn test_has_tool_calls_true_and_false() {
        let no_tools = make_response(vec![ContentBlock::Text(TextBlock { text: "hi".into() })]);
        assert!(!no_tools.has_tool_calls());

        let with_tool = make_response(vec![ContentBlock::ToolUse(ToolUseBlock {
            id: "c".into(),
            name: "run".into(),
            input: serde_json::json!({}),
            raw_input: None,
        })]);
        assert!(with_tool.has_tool_calls());
    }

    #[test]
    fn test_chat_usage_default() {
        let usage = ChatUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
        assert!(usage.duration_ms.is_none());
    }

    #[test]
    fn test_chat_response_serialization_roundtrip() {
        let resp = make_response(vec![ContentBlock::Text(TextBlock { text: "hi".into() })]);
        let json = serde_json::to_string(&resp).unwrap();
        let back: ChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "resp-1");
        assert_eq!(back.get_text(), "hi");
        assert!(back.metadata.is_none());
    }

    #[test]
    fn test_chat_options_default_all_none() {
        let opts = ChatOptions::default();
        assert!(opts.model.is_none());
        assert!(opts.temperature.is_none());
        assert!(opts.max_tokens.is_none());
        assert!(opts.tools.is_none());
        assert!(opts.tool_choice.is_none());
    }

    #[test]
    fn test_tool_choice_serialization() {
        let choices = vec![
            ToolChoice::Auto,
            ToolChoice::None,
            ToolChoice::Required,
            ToolChoice::Specific("my_tool".into()),
        ];
        for choice in &choices {
            let json = serde_json::to_string(choice).unwrap();
            let back: ToolChoice = serde_json::from_str(&json).unwrap();
            assert_eq!(choice, &back);
        }
    }
}
