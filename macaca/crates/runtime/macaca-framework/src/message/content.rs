//! Message body: plain text or a composite block array.
//!
//! **Design pattern:** Strategy — [`MsgContent`] exposes uniform accessors
//! (`get_text`, `get_tool_calls`, `strip_thinking`) regardless of internal shape.

use serde::{Deserialize, Serialize};
use tracing::debug;

use super::blocks::{ContentBlock, ToolUseBlock};

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
    /// Concatenate all [`ContentBlock::Text`] segments into one string.
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

    /// Collect references to all tool-use blocks (empty for plain-text content).
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

    /// Return a copy with all [`ContentBlock::Thinking`] nodes removed.
    ///
    /// Used when broadcasting messages to peer agents — internal reasoning must
    /// not leak across agent boundaries (Decorator / filter semantics).
    pub fn strip_thinking(&self) -> MsgContent {
        match self {
            MsgContent::Text(_) => self.clone(),
            MsgContent::Blocks(blocks) => {
                let before = blocks.len();
                let filtered: Vec<ContentBlock> = blocks
                    .iter()
                    .filter(|b| !matches!(b, ContentBlock::Thinking(_)))
                    .cloned()
                    .collect();
                let removed = before.saturating_sub(filtered.len());
                if removed > 0 {
                    debug!(
                        target: "macaca_framework::message",
                        removed_thinking_blocks = removed,
                        remaining_blocks = filtered.len(),
                        "strip_thinking applied"
                    );
                }
                if filtered.is_empty() {
                    MsgContent::Text(String::new())
                } else {
                    MsgContent::Blocks(filtered)
                }
            }
        }
    }

    /// Returns true when any block is a tool-use request.
    pub fn has_tool_calls(&self) -> bool {
        match self {
            MsgContent::Text(_) => false,
            MsgContent::Blocks(blocks) => {
                blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse(_)))
            }
        }
    }

    /// Returns true when there is no textual or block payload.
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
