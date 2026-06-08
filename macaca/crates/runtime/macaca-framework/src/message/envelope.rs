//! Universal message envelope (`Msg`) for agent communication.
//!
//! **Design pattern:** Builder — factory helpers (`user`, `assistant`, `system`,
//! `tool_result`, `with_metadata`) construct well-formed envelopes; `new` is the
//! low-level entry point used by all factories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use super::blocks::{ContentBlock, ToolResultBlock, ToolUseBlock};
use super::content::MsgContent;
use super::role::Role;

/// The universal message type for agent communication.
///
/// Every piece of information flowing between agents, models, tools, and pipelines
/// is wrapped in a `Msg`. The `name` field carries the sender identifier configured
/// by the hosting application (agent id, tool id, etc.) — never hardcoded here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    /// Unique message identifier (UUID v4 string).
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
    /// Create a new message with the given parameters and a fresh UUID.
    pub fn new(name: impl Into<String>, content: impl Into<MsgContent>, role: Role) -> Self {
        let name = name.into();
        let msg = Self {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            content: content.into(),
            role,
            metadata: serde_json::Value::Null,
            timestamp: Utc::now(),
            invocation_id: None,
        };
        debug!(
            target: "macaca_framework::message",
            msg_id = %msg.id,
            sender = %name,
            role = ?role,
            has_tool_calls = msg.content.has_tool_calls(),
            "message created"
        );
        msg
    }

    /// Create a user-role message.
    pub fn user(name: impl Into<String>, content: impl Into<MsgContent>) -> Self {
        Self::new(name, content, Role::User)
    }

    /// Create an assistant-role message.
    pub fn assistant(name: impl Into<String>, content: impl Into<MsgContent>) -> Self {
        Self::new(name, content, Role::Assistant)
    }

    /// Create a system-role message (sender name is the literal role label).
    pub fn system(content: impl Into<MsgContent>) -> Self {
        Self::new("system", content, Role::System)
    }

    /// Create a tool-result message wrapping a single [`ToolResultBlock`].
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        let tool_name: String = name.into();
        let tool_use_id = tool_use_id.into();
        let block = ToolResultBlock {
            tool_use_id: tool_use_id.clone(),
            output: output.into(),
            name: Some(tool_name.clone()),
            is_error,
        };
        debug!(
            target: "macaca_framework::message",
            tool_use_id = %tool_use_id,
            tool_name = %tool_name,
            is_error,
            "tool_result message created"
        );
        Self::new(
            tool_name,
            MsgContent::Blocks(vec![ContentBlock::ToolResult(block)]),
            Role::Tool,
        )
    }

    /// Extract concatenated text from the inner content.
    pub fn get_text(&self) -> String {
        self.content.get_text()
    }

    /// Collect tool-use block references from the inner content.
    pub fn get_tool_calls(&self) -> Vec<&ToolUseBlock> {
        self.content.get_tool_calls()
    }

    /// Return a copy with thinking blocks stripped (safe for cross-agent broadcast).
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
