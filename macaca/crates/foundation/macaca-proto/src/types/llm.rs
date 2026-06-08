//! LLM chat messages, tool-call types, completion options, and agent output.

use serde::{Deserialize, Serialize};

// ── LLM Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool call requested by the LLM (function calling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// A tool definition sent to the LLM so it knows what tools are available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
    /// Provider-specific reasoning content returned by thinking models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Tool calls requested by the assistant (present when role=Assistant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The tool_call id this message is responding to (present when role=Tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl LlmMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::System,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::User,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message (plain text, no tool calls).
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with provider-specific reasoning content.
    pub fn assistant_with_reasoning(
        content: impl Into<String>,
        reasoning_content: impl Into<String>,
    ) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: Some(reasoning_content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message that requests tool calls.
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Tool,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOptions {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
    /// Tool definitions to send to the LLM for function calling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

impl Default for LlmOptions {
    fn default() -> Self {
        Self {
            // Empty model defers routing to the LLM service resolver / user config.
            model: String::new(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stop_sequences: Vec::new(),
            tools: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    /// Provider-specific reasoning content returned by thinking models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: String,
    /// Tool calls requested by the model (if finish_reason indicates tool use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Agent Output ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub result: String,
    pub artifacts: Vec<String>,
    pub tokens_used: TokenUsage,
}
