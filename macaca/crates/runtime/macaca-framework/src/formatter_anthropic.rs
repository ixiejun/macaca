// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Anthropic message formatter strategy.
//!
//! Keeping this provider wire-format adapter in a separate module prevents the
//! framework formatter contract from becoming a provider-specific monolith. The
//! adapter is still generic framework code; runtime-host decides which model
//! provider strategy is composed for a concrete service instance.

use serde_json::{json, Value};
use tracing::warn;

use crate::formatter::{Formatter, FormatterError};
use crate::message::{ContentBlock, Msg, MsgContent, Role, TextBlock, ToolUseBlock};
use crate::model::{ChatResponse, ChatUsage};

/// Formats messages for the Anthropic Messages API.
///
/// Key differences from OpenAI:
/// - System messages are returned separately (first system message is extracted)
/// - `tool_use` is a content block, not a top-level `tool_calls` array
/// - `tool_result` is a user message with `type: "tool_result"` content block
/// - Content is always an array of typed objects, never a plain string
///
/// `format()` returns messages without the system prompt.
/// Callers should extract the system message themselves via `extract_system`.
pub struct AnthropicFormatter;

impl AnthropicFormatter {
    /// Extract the system prompt string from a message list (first system message).
    pub fn extract_system(msgs: &[Msg]) -> Option<String> {
        msgs.iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.get_text())
    }
}

impl Formatter for AnthropicFormatter {
    fn format(&self, msgs: &[Msg]) -> Vec<Value> {
        msgs.iter()
            .filter(|m| m.role != Role::System)
            .map(format_anthropic_msg)
            .collect()
    }

    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError> {
        let id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let usage = if let Some(u) = raw.get("usage") {
            ChatUsage {
                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: {
                    let i = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let o = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    (i + o) as u32
                },
                duration_ms: None,
            }
        } else {
            ChatUsage::default()
        };

        let content_arr = raw
            .get("content")
            .and_then(|v| v.as_array())
            .ok_or_else(|| FormatterError::Parse("missing content array".into()))?;

        let mut blocks: Vec<ContentBlock> = Vec::new();

        for item in content_arr {
            let block_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match block_type {
                "text" => {
                    let text = item
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    blocks.push(ContentBlock::Text(TextBlock { text }));
                }
                "thinking" => {
                    let thinking = item
                        .get("thinking")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    blocks.push(ContentBlock::Thinking(crate::message::ThinkingBlock {
                        thinking,
                    }));
                }
                "tool_use" => {
                    let tu_id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let input = item.get("input").cloned().unwrap_or_else(|| json!({}));
                    blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                        id: tu_id,
                        name,
                        input,
                        raw_input: None,
                    }));
                }
                _ => {
                    // Unknown blocks are not fatal because providers can add
                    // forward-compatible block kinds. We log the type only,
                    // never raw provider payloads.
                    if !block_type.is_empty() {
                        warn!(block_type, "skipping unknown Anthropic content block type");
                    }
                }
            }
        }

        Ok(ChatResponse {
            content: blocks,
            id,
            created_at: String::new(),
            usage,
            metadata: None,
        })
    }
}

fn format_anthropic_msg(msg: &Msg) -> Value {
    let role_str = match msg.role {
        Role::User | Role::Tool => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    };

    match &msg.content {
        MsgContent::Text(text) => {
            json!({
                "role": role_str,
                "content": [{"type": "text", "text": text}]
            })
        }
        MsgContent::Blocks(blocks) => {
            let mut content_parts: Vec<Value> = Vec::new();

            for block in blocks {
                match block {
                    ContentBlock::Text(t) => {
                        content_parts.push(json!({"type": "text", "text": t.text}));
                    }
                    ContentBlock::Thinking(t) => {
                        content_parts.push(json!({"type": "thinking", "thinking": t.thinking}));
                    }
                    ContentBlock::ToolUse(t) => {
                        content_parts.push(json!({
                            "type": "tool_use",
                            "id": t.id,
                            "name": t.name,
                            "input": t.input,
                        }));
                    }
                    ContentBlock::ToolResult(r) => {
                        content_parts.push(json!({
                            "type": "tool_result",
                            "tool_use_id": r.tool_use_id,
                            "content": r.output,
                            "is_error": r.is_error,
                        }));
                    }
                    ContentBlock::Image(img) => {
                        if let Some(data) = &img.data {
                            let mime = img.mime_type.as_deref().unwrap_or("image/png");
                            content_parts.push(json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime,
                                    "data": data,
                                }
                            }));
                        } else if let Some(url) = &img.url {
                            content_parts.push(json!({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": url,
                                }
                            }));
                        }
                    }
                    ContentBlock::Data(data) => {
                        // Anthropic has no generic data block in this formatter.
                        // Convert sanitized data to text rather than dropping it.
                        content_parts.push(json!({
                            "type": "text",
                            "text": data.data.to_string(),
                        }));
                    }
                    ContentBlock::Hint(_) => {
                        // Hints are internal middleware guidance and must not be
                        // serialized into provider prompts.
                    }
                    ContentBlock::Audio(_) | ContentBlock::Video(_) => {}
                }
            }

            json!({"role": role_str, "content": content_parts})
        }
    }
}
