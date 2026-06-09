// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It preserves the stable
// Macaca message constructors while adding AgentScope 2.0-equivalent validation
// helpers and content blocks for provider-neutral framework execution.

//! Unified message types with rich content blocks.
//!
//! The message system mirrors AgentScope's design:
//! - `Msg` is the universal message envelope
//! - `ContentBlock` is a tagged enum of 7 content types
//! - `MsgContent` supports both plain text and block arrays

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::message_metadata::{GenerateReason, MessageUsage, ToolCallState, ToolResultState};

// ---------------------------------------------------------------------------
// Content Blocks
// ---------------------------------------------------------------------------

/// A single block of content within a message.
///
/// Messages can contain multiple blocks of different types, enabling
/// rich multi-modal and tool-call interactions.
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
    /// Structured data emitted by a model, tool, or middleware.
    Data(DataBlock),
    /// Provider-neutral hint used by middleware or harness features.
    Hint(HintBlock),
    /// Image data (base64 or URL).
    Image(ImageBlock),
    /// Audio data (base64 or URL).
    Audio(AudioBlock),
    /// Video data (base64 or URL).
    Video(VideoBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingBlock {
    pub thinking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseBlock {
    /// Unique ID for correlating with ToolResultBlock.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool input arguments.
    pub input: serde_json::Value,
    /// Raw string input (before JSON parsing), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultBlock {
    /// Correlates with the ToolUseBlock's id.
    pub tool_use_id: String,
    /// Tool output content.
    pub output: String,
    /// Tool name (for display purposes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataBlock {
    /// Stable block identifier when the data originates from an event stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Structured data value. Callers must keep this sanitized and bounded.
    pub data: serde_json::Value,
    /// Optional semantic name for protocol adapters and diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HintBlock {
    /// Stable block identifier when the hint originates from an event stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Hint text or compact instruction for a framework middleware stage.
    pub hint: String,
    /// Optional hint category. This is generic framework metadata, not an app name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageBlock {
    /// Base64-encoded image data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// URL to the image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// MIME type (e.g., "image/png").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Message Content (String or Blocks)
// ---------------------------------------------------------------------------

/// Message content: either a simple text string or a list of content blocks.
///
/// Simple use cases pass plain text; complex interactions (tool calls,
/// multi-modal) use the block array form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MsgContent {
    /// Plain text content.
    Text(String),
    /// Array of content blocks (multi-modal, tool calls, etc.).
    Blocks(Vec<ContentBlock>),
}

impl MsgContent {
    /// Extract all text from this content.
    pub fn get_text(&self) -> String {
        match self {
            MsgContent::Text(s) => s.clone(),
            MsgContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// Extract all tool use blocks.
    pub fn get_tool_calls(&self) -> Vec<&ToolUseBlock> {
        match self {
            MsgContent::Text(_) => vec![],
            MsgContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse(t) => Some(t),
                    _ => None,
                })
                .collect(),
        }
    }

    /// Return a new content with all ThinkingBlocks removed.
    ///
    /// Used when broadcasting messages to other agents — internal
    /// reasoning should not leak across agent boundaries.
    pub fn strip_thinking(&self) -> MsgContent {
        match self {
            MsgContent::Text(_) => self.clone(),
            MsgContent::Blocks(blocks) => {
                let filtered: Vec<ContentBlock> = blocks
                    .iter()
                    .filter(|b| !matches!(b, ContentBlock::Thinking(_)))
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    MsgContent::Text(String::new())
                } else {
                    MsgContent::Blocks(filtered)
                }
            }
        }
    }

    /// Check if this content has any tool calls.
    pub fn has_tool_calls(&self) -> bool {
        match self {
            MsgContent::Text(_) => false,
            MsgContent::Blocks(blocks) => {
                blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse(_)))
            }
        }
    }

    /// Check if content is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            MsgContent::Text(s) => s.is_empty(),
            MsgContent::Blocks(blocks) => blocks.is_empty(),
        }
    }
}

/// Structured message validation errors for AgentScope 2.0-equivalent execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MsgValidationError {
    #[error("user messages cannot contain tool-use blocks")]
    UserToolUse,
    #[error("system messages can only contain text content")]
    SystemNonText,
    #[error("tool messages must contain only tool-result blocks")]
    ToolRoleNonToolResult,
}

impl From<String> for MsgContent {
    fn from(s: String) -> Self {
        MsgContent::Text(s)
    }
}

impl From<&str> for MsgContent {
    fn from(s: &str) -> Self {
        MsgContent::Text(s.to_string())
    }
}

impl From<Vec<ContentBlock>> for MsgContent {
    fn from(blocks: Vec<ContentBlock>) -> Self {
        MsgContent::Blocks(blocks)
    }
}

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

// ---------------------------------------------------------------------------
// Msg — the universal message envelope
// ---------------------------------------------------------------------------

/// The universal message type for agent communication.
///
/// Every piece of information flowing between agents, models, tools,
/// and pipelines is wrapped in a `Msg`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    /// Unique message identifier.
    pub id: String,
    /// Name of the sender (agent name, user name, tool name).
    pub name: String,
    /// Message content (text or content blocks).
    pub content: MsgContent,
    /// Role of the message sender.
    pub role: Role,
    /// Structured metadata (e.g., validated output from structured generation).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
    /// When this message was created.
    pub timestamp: DateTime<Utc>,
    /// Optional invocation ID linking to a specific API call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
}

impl Msg {
    /// Create a new message with the given parameters.
    pub fn new(name: impl Into<String>, content: impl Into<MsgContent>, role: Role) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            content: content.into(),
            role,
            metadata: serde_json::Value::Null,
            timestamp: Utc::now(),
            invocation_id: None,
        }
    }

    /// Create a message and validate it for AgentScope 2.0-equivalent execution.
    ///
    /// Legacy code can still call `Msg::new`; framework providers should use
    /// this constructor or call `validate_for_framework` before model/tool side
    /// effects so invalid role/content combinations fail closed.
    pub fn try_new(
        name: impl Into<String>,
        content: impl Into<MsgContent>,
        role: Role,
    ) -> Result<Self, MsgValidationError> {
        let message = Self::new(name, content, role);
        message.validate_for_framework()?;
        Ok(message)
    }

    /// Validate role/content combinations for framework execution.
    pub fn validate_for_framework(&self) -> Result<(), MsgValidationError> {
        match (&self.role, &self.content) {
            (Role::User, MsgContent::Blocks(blocks))
                if blocks
                    .iter()
                    .any(|block| matches!(block, ContentBlock::ToolUse(_))) =>
            {
                Err(MsgValidationError::UserToolUse)
            }
            (Role::System, MsgContent::Blocks(blocks))
                if blocks
                    .iter()
                    .any(|block| !matches!(block, ContentBlock::Text(_))) =>
            {
                Err(MsgValidationError::SystemNonText)
            }
            (Role::Tool, MsgContent::Blocks(blocks))
                if blocks
                    .iter()
                    .any(|block| !matches!(block, ContentBlock::ToolResult(_))) =>
            {
                Err(MsgValidationError::ToolRoleNonToolResult)
            }
            (Role::Tool, MsgContent::Text(_)) => Err(MsgValidationError::ToolRoleNonToolResult),
            _ => Ok(()),
        }
    }

    /// Create a user message.
    pub fn user(name: impl Into<String>, content: impl Into<MsgContent>) -> Self {
        Self::new(name, content, Role::User)
    }

    /// Create an assistant message.
    pub fn assistant(name: impl Into<String>, content: impl Into<MsgContent>) -> Self {
        Self::new(name, content, Role::Assistant)
    }

    /// Create a system message.
    pub fn system(content: impl Into<MsgContent>) -> Self {
        Self::new("system", content, Role::System)
    }

    /// Create a tool result message.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        let tool_name: String = name.into();
        let block = ToolResultBlock {
            tool_use_id: tool_use_id.into(),
            output: output.into(),
            name: Some(tool_name.clone()),
            is_error,
        };
        Self::new(
            tool_name,
            MsgContent::Blocks(vec![ContentBlock::ToolResult(block)]),
            Role::Tool,
        )
    }

    /// Get the text content of this message.
    pub fn get_text(&self) -> String {
        self.content.get_text()
    }

    /// Get all tool call blocks from this message.
    pub fn get_tool_calls(&self) -> Vec<&ToolUseBlock> {
        self.content.get_tool_calls()
    }

    /// Return a copy with thinking blocks stripped (for broadcasting).
    pub fn stripped_for_broadcast(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            content: self.content.strip_thinking(),
            role: self.role,
            metadata: self.metadata.clone(),
            timestamp: self.timestamp,
            invocation_id: self.invocation_id.clone(),
        }
    }

    /// Attach metadata to this message (builder pattern).
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "message_tests.rs"]
mod message_tests;
