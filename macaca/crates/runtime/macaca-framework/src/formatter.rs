//! Message formatters — convert framework `Msg` to provider-specific API formats.
//!
//! Each formatter implements two operations:
//! - `format`: converts a slice of `Msg` into the JSON array the provider API expects
//! - `parse_response`: converts the raw provider response JSON into a unified `ChatResponse`
//!
//! Three built-in formatters are provided:
//! - [`OpenAiFormatter`] — OpenAI Chat Completions format
//! - [`DashScopeFormatter`] — Alibaba DashScope (QwenLM) format (OpenAI-compatible variant)
//! - [`AnthropicFormatter`] — Anthropic Messages API format

use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock,
};
use crate::model::{ChatResponse, ChatUsage};
use serde_json::{json, Value};

#[path = "formatter_anthropic.rs"]
mod formatter_anthropic;
pub use formatter_anthropic::AnthropicFormatter;

// ---------------------------------------------------------------------------
// FormatterError
// ---------------------------------------------------------------------------

/// Errors produced during message formatting or response parsing.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FormatterError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unsupported content type: {0}")]
    Unsupported(String),
}

// ---------------------------------------------------------------------------
// Formatter trait
// ---------------------------------------------------------------------------

/// Converts between framework `Msg` values and a provider's wire format.
pub trait Formatter: Send + Sync {
    /// Convert a slice of framework messages into provider-specific JSON objects.
    ///
    /// The returned `Vec<Value>` is passed directly as the `messages` array in
    /// the provider's API request.
    fn format(&self, msgs: &[Msg]) -> Vec<Value>;

    /// Parse the raw JSON body returned by the provider into a `ChatResponse`.
    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError>;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build an image content part for OpenAI / DashScope format.
fn image_block_to_openai_part(img: &ImageBlock) -> Value {
    if let Some(url) = &img.url {
        json!({"type": "image_url", "image_url": {"url": url}})
    } else if let Some(data) = &img.data {
        let mime = img.mime_type.as_deref().unwrap_or("image/png");
        let data_url = format!("data:{};base64,{}", mime, data);
        json!({"type": "image_url", "image_url": {"url": data_url}})
    } else {
        json!({"type": "image_url", "image_url": {"url": ""}})
    }
}

/// Convert a `ContentBlock` list into OpenAI-style content parts.
/// Returns either a plain string (if only one text block) or a JSON array.
fn blocks_to_openai_content(blocks: &[ContentBlock]) -> Value {
    // Collect non-thinking parts
    let mut parts: Vec<Value> = Vec::new();
    for block in blocks {
        match block {
            ContentBlock::Text(t) => {
                parts.push(json!({"type": "text", "text": t.text}));
            }
            ContentBlock::Image(img) => {
                parts.push(image_block_to_openai_part(img));
            }
            // ThinkingBlock is internal — omit from wire format
            ContentBlock::Thinking(_) => {}
            // ToolUse / ToolResult are handled at the message level
            ContentBlock::ToolUse(_) | ContentBlock::ToolResult(_) => {}
            // OpenAI chat content has no generic structured data part. Preserve
            // sanitized data as text so consumers do not silently lose content.
            ContentBlock::Data(data) => {
                parts.push(json!({"type": "text", "text": data.data.to_string()}));
            }
            // Hints are internal middleware guidance and are never sent to models.
            ContentBlock::Hint(_) => {}
            ContentBlock::Audio(_) | ContentBlock::Video(_) => {}
        }
    }

    if parts.len() == 1 {
        if let Some(text) = parts[0].get("text").and_then(|v| v.as_str()) {
            return Value::String(text.to_string());
        }
    }
    Value::Array(parts)
}

/// Extract `ToolUseBlock`s from a content block list.
fn extract_tool_use_blocks(blocks: &[ContentBlock]) -> Vec<&ToolUseBlock> {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolUse(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Extract `ToolResultBlock`s from a content block list.
fn extract_tool_result_blocks(blocks: &[ContentBlock]) -> Vec<&ToolResultBlock> {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolResult(r) => Some(r),
            _ => None,
        })
        .collect()
}

/// Parse an OpenAI/DashScope response JSON into a `ChatResponse`.
fn parse_openai_response(raw: Value) -> Result<ChatResponse, FormatterError> {
    let id = raw
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let created_at = raw
        .get("created")
        .map(|v| v.to_string())
        .unwrap_or_default();

    // Usage
    let usage = if let Some(u) = raw.get("usage") {
        ChatUsage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            duration_ms: None,
        }
    } else {
        ChatUsage::default()
    };

    // Extract first choice message — empty choices is not an error,
    // it just means no content was returned.
    let choice_msg = raw
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"));

    // If there is no choice message, return an empty response.
    let choice_msg = match choice_msg {
        Some(m) => m,
        None => {
            return Ok(ChatResponse {
                content: Vec::new(),
                id,
                created_at,
                usage,
                metadata: None,
            });
        }
    };

    let mut content_blocks: Vec<ContentBlock> = Vec::new();

    // Text content
    if let Some(reasoning_content) = choice_msg
        .get("reasoning_content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        content_blocks.push(ContentBlock::Thinking(ThinkingBlock {
            thinking: reasoning_content.to_string(),
        }));
    }

    if let Some(text) = choice_msg.get("content").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            content_blocks.push(ContentBlock::Text(TextBlock {
                text: text.to_string(),
            }));
        }
    }

    // Tool calls
    if let Some(tool_calls) = choice_msg.get("tool_calls").and_then(|v| v.as_array()) {
        for tc in tool_calls {
            let id = tc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let fn_obj = tc
                .get("function")
                .ok_or_else(|| FormatterError::Parse("tool_call missing function".into()))?;
            let name = fn_obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let args_str = fn_obj
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let input: Value =
                serde_json::from_str(args_str).unwrap_or_else(|_| json!({"_raw": args_str}));
            content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                id,
                name,
                input,
                raw_input: Some(args_str.to_string()),
            }));
        }
    }

    Ok(ChatResponse {
        content: content_blocks,
        id,
        created_at,
        usage,
        metadata: None,
    })
}

// ---------------------------------------------------------------------------
// OpenAiFormatter
// ---------------------------------------------------------------------------

/// Formats messages for the OpenAI Chat Completions API.
///
/// - `system` role → `{"role": "system", "content": "..."}`
/// - `user` role → `{"role": "user", "content": "text" | [parts]}`
/// - `assistant` with tool calls → `{"role": "assistant", "tool_calls": [...], "content": null}`
/// - `tool` role → `{"role": "tool", "tool_call_id": "...", "content": "..."}`
pub struct OpenAiFormatter;

impl Formatter for OpenAiFormatter {
    fn format(&self, msgs: &[Msg]) -> Vec<Value> {
        msgs.iter().map(|msg| format_openai_msg(msg)).collect()
    }

    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError> {
        parse_openai_response(raw)
    }
}

fn format_openai_msg(msg: &Msg) -> Value {
    let role_str = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    };

    match &msg.content {
        MsgContent::Text(text) => {
            json!({"role": role_str, "content": text})
        }
        MsgContent::Blocks(blocks) => {
            let reasoning_content = if msg.role == Role::Assistant {
                blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Thinking(t) => Some(t.thinking.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                String::new()
            };

            // Tool results → OpenAI tool messages
            let tool_results = extract_tool_result_blocks(blocks);
            if !tool_results.is_empty() {
                // OpenAI expects one tool message per tool result
                // We return the first one; callers should split multi-result messages
                let r = tool_results[0];
                return json!({
                    "role": "tool",
                    "tool_call_id": r.tool_use_id,
                    "content": r.output,
                });
            }

            // Tool calls → assistant message with tool_calls array
            let tool_uses = extract_tool_use_blocks(blocks);
            if !tool_uses.is_empty() {
                let tool_calls: Vec<Value> = tool_uses
                    .iter()
                    .map(|t| {
                        json!({
                            "id": t.id,
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "arguments": t.input.to_string(),
                            }
                        })
                    })
                    .collect();

                // Include text if present alongside tool calls
                let text_content: String = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                let mut message = json!({
                    "role": "assistant",
                    "content": if text_content.is_empty() { Value::Null } else { Value::String(text_content) },
                    "tool_calls": tool_calls,
                });
                if !reasoning_content.is_empty() {
                    message["reasoning_content"] = Value::String(reasoning_content);
                }
                return message;
            }

            // Regular content (text + images)
            let content = blocks_to_openai_content(blocks);
            let mut message = json!({"role": role_str, "content": content});
            if !reasoning_content.is_empty() {
                message["reasoning_content"] = Value::String(reasoning_content);
            }
            message
        }
    }
}

// ---------------------------------------------------------------------------
// DashScopeFormatter
// ---------------------------------------------------------------------------

/// Formats messages for the Alibaba DashScope API.
///
/// DashScope's Qwen models use an OpenAI-compatible interface with minor
/// differences in response field names (`input.choices` vs top-level `choices`,
/// `usage.input_tokens` vs `usage.prompt_tokens`).
pub struct DashScopeFormatter;

impl Formatter for DashScopeFormatter {
    fn format(&self, msgs: &[Msg]) -> Vec<Value> {
        // Message format is identical to OpenAI
        msgs.iter().map(|msg| format_openai_msg(msg)).collect()
    }

    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError> {
        // DashScope wraps choices in `output` for non-compatible endpoint,
        // but the OpenAI-compatible endpoint matches OpenAI exactly.
        // We try OpenAI format first, then fall back to DashScope native.
        if raw.get("choices").is_some() {
            return parse_openai_response(raw);
        }

        // DashScope native format: { "output": { "choices": [...] }, "usage": {...} }
        let id = raw
            .get("request_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let usage = if let Some(u) = raw.get("usage") {
            ChatUsage {
                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                duration_ms: None,
            }
        } else {
            ChatUsage::default()
        };

        let output = raw
            .get("output")
            .ok_or_else(|| FormatterError::Parse("missing output".into()))?;

        let choice_msg = output
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .ok_or_else(|| FormatterError::Parse("missing output.choices[0].message".into()))?;

        let mut content_blocks: Vec<ContentBlock> = Vec::new();

        if let Some(text) = choice_msg.get("content").and_then(|v| v.as_str()) {
            if !text.is_empty() {
                content_blocks.push(ContentBlock::Text(TextBlock {
                    text: text.to_string(),
                }));
            }
        }

        if let Some(tool_calls) = choice_msg.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in tool_calls {
                let tc_id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let fn_obj = tc
                    .get("function")
                    .ok_or_else(|| FormatterError::Parse("tool_call missing function".into()))?;
                let name = fn_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = fn_obj
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let input: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
                content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                    id: tc_id,
                    name,
                    input,
                    raw_input: Some(args_str.to_string()),
                }));
            }
        }

        Ok(ChatResponse {
            content: content_blocks,
            id,
            created_at: String::new(),
            usage,
            metadata: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "formatter_tests.rs"]
mod formatter_tests;

#[cfg(test)]
#[path = "formatter_robustness_tests.rs"]
mod formatter_robustness_tests;
