//! Deterministic LLM response fixtures for agentic dry-run stages.
//!
//! **Design pattern:** Test Double (stub) — canned [`LlmResponse`] values drive the
//! agentic loop without network access or provider credentials.

use macaca_proto::types::{LlmResponse, TokenUsage, ToolCall};

/// Build a text-only scripted LLM response (final assistant turn).
pub fn response_text(content: impl Into<String>) -> LlmResponse {
    LlmResponse {
        content: content.into(),
        reasoning_content: None,
        model: "scripted".into(),
        usage: TokenUsage::default(),
        finish_reason: "stop".into(),
        tool_calls: None,
    }
}

/// Build a scripted LLM response that requests one or more tool invocations.
pub fn response_with_tools(tool_calls: Vec<ToolCall>) -> LlmResponse {
    LlmResponse {
        content: String::new(),
        reasoning_content: None,
        model: "scripted".into(),
        usage: TokenUsage::default(),
        finish_reason: "tool_calls".into(),
        tool_calls: Some(tool_calls),
    }
}
