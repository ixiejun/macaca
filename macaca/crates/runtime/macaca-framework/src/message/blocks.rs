//! Tagged content blocks inside a [`super::MsgContent`] payload.
//!
//! **Design pattern:** Composite — each [`ContentBlock`] variant is a leaf node;
//! messages assemble heterogeneous blocks (text, tools, media) into one envelope.

use serde::{Deserialize, Serialize};

/// A single block of content within a message.
///
/// Messages can contain multiple blocks of different types, enabling rich
/// multi-modal and tool-call interactions across any application persona.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text(TextBlock),
    /// Internal reasoning (stripped during broadcast to other agents).
    Thinking(ThinkingBlock),
    /// Request to execute a tool.
    ToolUse(ToolUseBlock),
    /// Result of a tool execution.
    ToolResult(ToolResultBlock),
    /// Image data (base64 or URL).
    Image(ImageBlock),
    /// Audio data (base64 or URL).
    Audio(AudioBlock),
    /// Video data (base64 or URL).
    Video(VideoBlock),
}

/// Plain UTF-8 text segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBlock {
    pub text: String,
}

/// Opaque reasoning text that must not leak across agent boundaries on broadcast.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingBlock {
    pub thinking: String,
}

/// Tool invocation request emitted by the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseBlock {
    /// Unique ID for correlating with [`ToolResultBlock`].
    pub id: String,
    /// Registered tool name (provider-neutral identifier).
    pub name: String,
    /// Parsed tool input arguments.
    pub input: serde_json::Value,
    /// Raw string input (before JSON parsing), when the driver preserved it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<String>,
}

/// Tool execution outcome correlated to a prior [`ToolUseBlock`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultBlock {
    /// Correlates with the [`ToolUseBlock`] id.
    pub tool_use_id: String,
    /// Tool output content (text or serialized payload).
    pub output: String,
    /// Tool name (for display and tracing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

/// Image payload referenced by URL or inline base64.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Audio payload referenced by URL or inline base64.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Video payload referenced by URL or inline base64.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
