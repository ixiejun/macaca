//! Bidirectional Adapter between framework [`Msg`] and A2A wire types.
//!
//! **Design pattern:** Adapter — translates the internal message envelope into the
//! A2A `parts` representation (and back) while enforcing cross-agent boundary
//! policy (e.g. stripping internal reasoning blocks).

use tracing::{debug, warn};

use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ToolResultBlock, ToolUseBlock,
};

use super::error::A2AError;
use super::types::{A2AMessage, A2APart, A2AFile, A2ARole};

/// Converts between framework-internal `Msg` and A2A protocol types.
pub struct A2AFormatter;

impl A2AFormatter {
    /// Convert an internal `Msg` to an `A2AMessage`.
    ///
    /// `ThinkingBlock`s are stripped — internal reasoning must not cross
    /// agent boundaries. `Audio` and `Video` blocks are also omitted as A2A
    /// has no standard representation for them.
    pub fn to_a2a(msg: &Msg) -> A2AMessage {
        debug!(
            target: "macaca_framework::a2a::formatter",
            message_id = %msg.id,
            role = ?msg.role,
            "A2AFormatter::to_a2a entry — converting internal Msg to A2A wire shape"
        );

        let role = match msg.role {
            Role::User | Role::System | Role::Tool => A2ARole::User,
            Role::Assistant => A2ARole::Agent,
        };

        let parts = match &msg.content {
            MsgContent::Text(t) => vec![A2APart::Text { text: t.clone() }],
            MsgContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text(t) => Some(A2APart::Text {
                        text: t.text.clone(),
                    }),
                    ContentBlock::Image(img) => Some(A2APart::File {
                        file: A2AFile {
                            uri: img.url.clone(),
                            bytes: img.data.clone(),
                            mime_type: img.mime_type.clone(),
                        },
                    }),
                    ContentBlock::ToolUse(tu) => Some(A2APart::Data {
                        data: serde_json::json!({
                            "kind": "tool_use",
                            "id": tu.id,
                            "name": tu.name,
                            "input": tu.input,
                        }),
                    }),
                    ContentBlock::ToolResult(tr) => Some(A2APart::Data {
                        data: serde_json::json!({
                            "kind": "tool_result",
                            "tool_use_id": tr.tool_use_id,
                            "output": tr.output,
                            "is_error": tr.is_error,
                        }),
                    }),
                    // Strip internal reasoning — must not cross agent boundaries.
                    ContentBlock::Thinking(_) => None,
                    // Audio/Video have no A2A representation yet.
                    ContentBlock::Audio(_) | ContentBlock::Video(_) => None,
                })
                .collect(),
        };

        let a2a = A2AMessage {
            message_id: msg.id.clone(),
            role,
            parts,
            context_id: None,
        };

        debug!(
            target: "macaca_framework::a2a::formatter",
            message_id = %a2a.message_id,
            part_count = a2a.parts.len(),
            a2a_role = ?a2a.role,
            "A2AFormatter::to_a2a complete"
        );

        a2a
    }

    /// Convert an `A2AMessage` to an internal `Msg`.
    ///
    /// `name` is the local name to assign to the sender (used in multi-agent
    /// pipelines to identify the remote participant).
    ///
    /// Returns an error when a `Data` part contains an unrecognised `kind`,
    /// or when required fields for `tool_use` / `tool_result` are missing.
    pub fn from_a2a(name: &str, a2a_msg: &A2AMessage) -> Result<Msg, A2AError> {
        debug!(
            target: "macaca_framework::a2a::formatter",
            message_id = %a2a_msg.message_id,
            remote_name = name,
            part_count = a2a_msg.parts.len(),
            a2a_role = ?a2a_msg.role,
            "A2AFormatter::from_a2a entry — converting A2A wire message to internal Msg"
        );

        let role = match a2a_msg.role {
            A2ARole::User => Role::User,
            A2ARole::Agent => Role::Assistant,
        };

        let mut blocks: Vec<ContentBlock> = Vec::new();

        for p in &a2a_msg.parts {
            match p {
                A2APart::Text { text } => {
                    blocks.push(ContentBlock::Text(TextBlock { text: text.clone() }));
                }
                A2APart::File { file } => {
                    blocks.push(ContentBlock::Image(ImageBlock {
                        url: file.uri.clone(),
                        data: file.bytes.clone(),
                        mime_type: file.mime_type.clone(),
                    }));
                }
                A2APart::Data { data } => {
                    let kind = data.get("kind").and_then(|v| v.as_str()).ok_or_else(|| {
                        warn!(
                            target: "macaca_framework::a2a::formatter",
                            message_id = %a2a_msg.message_id,
                            "A2AFormatter::from_a2a rejected Data part — missing kind field"
                        );
                        A2AError::MissingField("kind".into())
                    })?;

                    match kind {
                        "tool_use" => {
                            // Validate required fields
                            if data.get("name").and_then(|v| v.as_str()).is_none() {
                                warn!(
                                    target: "macaca_framework::a2a::formatter",
                                    message_id = %a2a_msg.message_id,
                                    kind = "tool_use",
                                    "A2AFormatter::from_a2a rejected Data part — missing required field"
                                );
                                return Err(A2AError::MissingField(
                                    "name (required for tool_use)".into(),
                                ));
                            }
                            if data.get("id").and_then(|v| v.as_str()).is_none() {
                                warn!(
                                    target: "macaca_framework::a2a::formatter",
                                    message_id = %a2a_msg.message_id,
                                    kind = "tool_use",
                                    "A2AFormatter::from_a2a rejected Data part — missing required field"
                                );
                                return Err(A2AError::MissingField(
                                    "id (required for tool_use)".into(),
                                ));
                            }
                            blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                                id: data["id"].as_str().unwrap_or("").to_string(),
                                name: data["name"].as_str().unwrap_or("").to_string(),
                                input: data["input"].clone(),
                                raw_input: None,
                            }));
                        }
                        "tool_result" => {
                            if data.get("tool_use_id").and_then(|v| v.as_str()).is_none() {
                                warn!(
                                    target: "macaca_framework::a2a::formatter",
                                    message_id = %a2a_msg.message_id,
                                    kind = "tool_result",
                                    "A2AFormatter::from_a2a rejected Data part — missing required field"
                                );
                                return Err(A2AError::MissingField(
                                    "tool_use_id (required for tool_result)".into(),
                                ));
                            }
                            blocks.push(ContentBlock::ToolResult(ToolResultBlock {
                                tool_use_id: data["tool_use_id"].as_str().unwrap_or("").to_string(),
                                output: data["output"].as_str().unwrap_or("").to_string(),
                                name: None,
                                is_error: data["is_error"].as_bool().unwrap_or(false),
                            }));
                        }
                        other => {
                            warn!(
                                target: "macaca_framework::a2a::formatter",
                                message_id = %a2a_msg.message_id,
                                kind = other,
                                "A2AFormatter::from_a2a rejected Data part — unrecognised kind"
                            );
                            return Err(A2AError::InvalidDataType(other.to_string()));
                        }
                    }
                }
            }
        }

        // Prefer a simple Text content when there is exactly one text block.
        let content = match blocks.as_slice() {
            [ContentBlock::Text(t)] => MsgContent::Text(t.text.clone()),
            _ => MsgContent::Blocks(blocks),
        };

        let msg = Msg::new(name, content, role);

        debug!(
            target: "macaca_framework::a2a::formatter",
            message_id = %a2a_msg.message_id,
            internal_role = ?msg.role,
            "A2AFormatter::from_a2a complete"
        );

        Ok(msg)
    }
}
