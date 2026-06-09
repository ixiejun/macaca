//! Tool execution result value object.
//!
//! [`ToolResponse`] is the canonical return type for every [`super::traits::ToolHandler`]
//! invocation. It wraps one or more [`ContentBlock`] values plus optional metadata so
//! middleware and callers can inspect streaming/interrupt semantics without coupling to
//! a specific application payload shape.

use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

/// The result returned by a tool execution.
///
/// # Value Object semantics
///
/// Instances are cheap to clone and carry no hidden identity — two responses with the
/// same fields are interchangeable. Factory helpers (`text`, `error`, `json`) encode the
/// most common construction paths used by framework adapters and test fixtures.
#[derive(Debug, Clone)]
pub struct ToolResponse {
    /// Content blocks produced by the tool.
    pub content: Vec<ContentBlock>,
    /// Optional arbitrary metadata (e.g. token counts, latency).
    pub metadata: Option<Value>,
    /// Whether the response is a streaming chunk.
    pub is_stream: bool,
    /// Whether this is the final chunk of a stream.
    pub is_last: bool,
    /// Whether the execution was interrupted mid-stream.
    pub is_interrupted: bool,
}

impl ToolResponse {
    /// Construct a plain-text response from a single string body.
    ///
    /// Non-streaming responses set `is_last = true` so callers can treat the payload as
    /// complete without inspecting stream flags.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock { text: s.into() })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct an error-shaped text response.
    ///
    /// Handlers may return this inside `Ok(...)` when they prefer a structured message
    /// over propagating [`super::error::ToolError`]. The `"Error: "` prefix is a
    /// convention for human-readable diagnostics; callers that need machine parsing
    /// should use the `Err` variant of [`super::toolkit::Toolkit::call_tool`] instead.
    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: format!("Error: {}", s.into()),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct a response whose text is the JSON-serialized form of `value`.
    ///
    /// Uses `Value::to_string()` (compact JSON) so echo-style test tools and simple
    /// adapters can round-trip arbitrary argument objects without a custom serializer.
    pub fn json(value: Value) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: value.to_string(),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }
}
