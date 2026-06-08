//! Shared OpenAI-compatible wire helpers used by [`super::openai::OpenAiFormatter`]
//! and [`super::dashscope::DashScopeFormatter`].
//!
//! Centralises content-part encoding and response parsing so duplicate provider
//! adapters do not fork the same JSON shape logic.

use serde_json::{json, Value};

use crate::message::{
    ContentBlock, ImageBlock, MsgContent, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock,
};
use crate::model::{ChatResponse, ChatUsage};

use super::error::FormatterError;

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
pub(crate) fn blocks_to_openai_content(blocks: &[ContentBlock]) -> Value {
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
pub(crate) fn extract_tool_use_blocks(blocks: &[ContentBlock]) -> Vec<&ToolUseBlock> {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolUse(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Extract `ToolResultBlock`s from a content block list.
pub(crate) fn extract_tool_result_blocks(blocks: &[ContentBlock]) -> Vec<&ToolResultBlock> {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolResult(r) => Some(r),
            _ => None,
        })
        .collect()
}

/// Parse an OpenAI/DashScope response JSON into a `ChatResponse`.
pub(crate) fn parse_openai_response(raw: Value) -> Result<ChatResponse, FormatterError> {
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
