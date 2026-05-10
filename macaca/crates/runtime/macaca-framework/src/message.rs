//! Unified message types with rich content blocks.
//!
//! The message system mirrors AgentScope's design:
//! - `Msg` is the universal message envelope
//! - `ContentBlock` is a tagged enum of 7 content types
//! - `MsgContent` supports both plain text and block arrays

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
mod tests {
    use super::*;

    #[test]
    fn test_msg_user_text() {
        let msg = Msg::user("alice", "hello world");
        assert_eq!(msg.name, "alice");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.get_text(), "hello world");
        assert!(!msg.content.has_tool_calls());
    }

    #[test]
    fn test_msg_with_blocks() {
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Let me help.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "call_1".into(),
                name: "search".into(),
                input: serde_json::json!({"query": "rust"}),
                raw_input: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        assert_eq!(msg.get_text(), "Let me help.");
        assert!(msg.content.has_tool_calls());
        assert_eq!(msg.get_tool_calls().len(), 1);
        assert_eq!(msg.get_tool_calls()[0].name, "search");
    }

    #[test]
    fn test_strip_thinking() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "hmm...".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "Here's the answer.".into(),
            }),
        ];
        let content = MsgContent::Blocks(blocks);
        let stripped = content.strip_thinking();
        match &stripped {
            MsgContent::Blocks(b) => {
                assert_eq!(b.len(), 1);
                assert!(matches!(&b[0], ContentBlock::Text(_)));
            }
            _ => panic!("Expected blocks"),
        }
    }

    #[test]
    fn test_tool_result_msg() {
        let msg = Msg::tool_result("call_1", "search", "found 5 results", false);
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.name, "search");
        match &msg.content {
            MsgContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1);
                match &blocks[0] {
                    ContentBlock::ToolResult(r) => {
                        assert_eq!(r.tool_use_id, "call_1");
                        assert!(!r.is_error);
                    }
                    _ => panic!("Expected ToolResult"),
                }
            }
            _ => panic!("Expected Blocks"),
        }
    }

    #[test]
    fn test_msg_serialization_roundtrip() {
        let msg = Msg::user("alice", "hello");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Msg = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "alice");
        assert_eq!(deserialized.get_text(), "hello");
        assert_eq!(deserialized.role, Role::User);
    }

    #[test]
    fn test_content_block_serialization() {
        let block = ContentBlock::ToolUse(ToolUseBlock {
            id: "c1".into(),
            name: "file_read".into(),
            input: serde_json::json!({"path": "/tmp/test.txt"}),
            raw_input: None,
        });
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, deserialized);
    }

    #[test]
    fn test_msg_content_from_string() {
        let content: MsgContent = "hello".into();
        assert_eq!(content.get_text(), "hello");
        assert!(!content.has_tool_calls());
    }

    #[test]
    fn test_empty_content() {
        assert!(MsgContent::Text(String::new()).is_empty());
        assert!(MsgContent::Blocks(vec![]).is_empty());
        assert!(!MsgContent::Text("hi".into()).is_empty());
    }

    #[test]
    fn test_strip_thinking_all_thinking() {
        let blocks = vec![ContentBlock::Thinking(ThinkingBlock {
            thinking: "thought".into(),
        })];
        let stripped = MsgContent::Blocks(blocks).strip_thinking();
        assert!(matches!(stripped, MsgContent::Text(s) if s.is_empty()));
    }

    // -----------------------------------------------------------------------
    // Boundary tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_content_msg_serde_roundtrip() {
        let msg = Msg::user("alice", "");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Msg = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.get_text(), "");
        assert_eq!(deserialized.name, "alice");
        assert_eq!(deserialized.role, Role::User);
        assert!(deserialized.content.is_empty());
    }

    #[test]
    fn test_all_seven_content_blocks_mixed() {
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "hello".into(),
            }),
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "hmm".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "t1".into(),
                name: "search".into(),
                input: serde_json::json!({}),
                raw_input: None,
            }),
            ContentBlock::ToolResult(ToolResultBlock {
                tool_use_id: "t1".into(),
                output: "done".into(),
                name: Some("search".into()),
                is_error: false,
            }),
            ContentBlock::Image(ImageBlock {
                data: Some("base64data".into()),
                url: None,
                mime_type: Some("image/png".into()),
            }),
            ContentBlock::Audio(AudioBlock {
                data: None,
                url: Some("https://example.com/audio.mp3".into()),
                mime_type: Some("audio/mp3".into()),
            }),
            ContentBlock::Video(VideoBlock {
                data: None,
                url: Some("https://example.com/video.mp4".into()),
                mime_type: Some("video/mp4".into()),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        // Verify all 7 blocks are preserved
        match &msg.content {
            MsgContent::Blocks(b) => assert_eq!(b.len(), 7),
            _ => panic!("Expected Blocks"),
        }
        // Serde roundtrip preserves all blocks
        let json = serde_json::to_string(&msg).unwrap();
        let de: Msg = serde_json::from_str(&json).unwrap();
        match &de.content {
            MsgContent::Blocks(b) => assert_eq!(b.len(), 7),
            _ => panic!("Expected Blocks after roundtrip"),
        }
    }

    #[test]
    fn test_strip_thinking_idempotent() {
        let blocks = vec![
            ContentBlock::Text(TextBlock { text: "a".into() }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "t1".into(),
                name: "x".into(),
                input: serde_json::json!(null),
                raw_input: None,
            }),
        ];
        let content = MsgContent::Blocks(blocks.clone());
        let stripped = content.strip_thinking();
        // No thinking blocks, so content should be unchanged
        assert_eq!(stripped, MsgContent::Blocks(blocks));
    }

    #[test]
    fn test_get_tool_calls_text_only() {
        let msg = Msg::user("alice", "just text");
        let calls = msg.get_tool_calls();
        assert!(calls.is_empty());

        // Also test Blocks variant with no ToolUse
        let blocks = vec![ContentBlock::Text(TextBlock { text: "hi".into() })];
        let content = MsgContent::Blocks(blocks);
        assert!(content.get_tool_calls().is_empty());
    }

    #[test]
    fn test_large_text_content() {
        // Create >1MB text
        let large_text = "A".repeat(1_100_000);
        let msg = Msg::user("alice", large_text.as_str());
        let json = serde_json::to_string(&msg).unwrap();
        let de: Msg = serde_json::from_str(&json).unwrap();
        assert_eq!(de.get_text().len(), 1_100_000);
        assert_eq!(de.get_text(), large_text);
    }

    #[test]
    fn test_metadata_nested_json() {
        let nested = serde_json::json!({
            "level1": {
                "level2": {
                    "key": "value",
                    "numbers": [1, 2, 3]
                }
            },
            "tags": ["a", "b", "c"],
            "flag": true,
            "count": 42
        });
        let msg = Msg::user("alice", "hi").with_metadata(nested.clone());
        let json = serde_json::to_string(&msg).unwrap();
        let de: Msg = serde_json::from_str(&json).unwrap();
        assert_eq!(de.metadata, nested);
    }

    #[test]
    fn test_special_characters_unicode() {
        let text = "Hello 🎉 世界 \u{200B} café ñ ü \t\n emoji: 🚀🌍";
        let msg = Msg::user("alice", text);
        assert_eq!(msg.get_text(), text);
        // Serde roundtrip preserves special chars
        let json = serde_json::to_string(&msg).unwrap();
        let de: Msg = serde_json::from_str(&json).unwrap();
        assert_eq!(de.get_text(), text);
    }

    #[test]
    fn test_tool_result_construction() {
        let msg = Msg::tool_result("call_42", "my_tool", "output data", true);
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.name, "my_tool");
        match &msg.content {
            MsgContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1);
                match &blocks[0] {
                    ContentBlock::ToolResult(r) => {
                        assert_eq!(r.tool_use_id, "call_42");
                        assert_eq!(r.output, "output data");
                        assert_eq!(r.name, Some("my_tool".into()));
                        assert!(r.is_error);
                    }
                    _ => panic!("Expected ToolResult block"),
                }
            }
            _ => panic!("Expected Blocks content"),
        }
    }

    #[test]
    fn test_msg_content_text_vs_blocks_text() {
        let text_content = MsgContent::Text("hello".into());
        let blocks_content = MsgContent::Blocks(vec![ContentBlock::Text(TextBlock {
            text: "hello".into(),
        })]);
        // Both should produce the same text via get_text()
        assert_eq!(text_content.get_text(), blocks_content.get_text());
        assert_eq!(text_content.get_text(), "hello");
    }
}
