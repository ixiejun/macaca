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

use serde_json::{json, Value};
use tracing::warn;

use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock,
};
use crate::model::{ChatResponse, ChatUsage};

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
// AnthropicFormatter
// ---------------------------------------------------------------------------

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
            .map(|msg| format_anthropic_msg(msg))
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
                    // Unknown block types are logged and skipped.
                    if !block_type.is_empty() {
                        warn!(block_type, "Skipping unknown Anthropic content block type");
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
        Role::System => "system", // filtered out before reaching here
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
                        // Anthropic image block
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
                    ContentBlock::Audio(_) | ContentBlock::Video(_) => {
                        // Not supported by Anthropic — skip
                    }
                }
            }

            json!({"role": role_str, "content": content_parts})
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{
        ContentBlock, ImageBlock, Msg, MsgContent, TextBlock, ThinkingBlock, ToolResultBlock,
        ToolUseBlock,
    };

    // -----------------------------------------------------------------------
    // OpenAiFormatter
    // -----------------------------------------------------------------------

    #[test]
    fn test_openai_format_system_message() {
        let msg = Msg::system("You are a helpful assistant.");
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["role"], "system");
        assert_eq!(result[0]["content"], "You are a helpful assistant.");
    }

    #[test]
    fn test_openai_format_user_text() {
        let msg = Msg::user("alice", "hello");
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"], "hello");
    }

    #[test]
    fn test_openai_format_assistant_with_tool_calls() {
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Let me search.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "call_1".into(),
                name: "web_search".into(),
                input: json!({"query": "rust"}),
                raw_input: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);

        assert_eq!(result[0]["role"], "assistant");
        let tool_calls = result[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_1");
        assert_eq!(tool_calls[0]["function"]["name"], "web_search");
    }

    #[test]
    fn test_openai_format_tool_result() {
        let msg = Msg::tool_result("call_1", "search", "5 results found", false);
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);
        assert_eq!(result[0]["role"], "tool");
        assert_eq!(result[0]["tool_call_id"], "call_1");
        assert_eq!(result[0]["content"], "5 results found");
    }

    #[test]
    fn test_openai_format_image_url() {
        let blocks = vec![ContentBlock::Image(ImageBlock {
            url: Some("https://example.com/img.png".into()),
            data: None,
            mime_type: None,
        })];
        let msg = Msg::user("alice", MsgContent::Blocks(blocks));
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image_url");
        assert_eq!(
            content[0]["image_url"]["url"],
            "https://example.com/img.png"
        );
    }

    #[test]
    fn test_openai_thinking_omitted() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "internal".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "visible".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[msg]);
        // content should only contain "visible"
        assert_eq!(result[0]["content"], "visible");
    }

    #[test]
    fn test_openai_assistant_reasoning_content_roundtrip() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "reasoning".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "answer".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = OpenAiFormatter;
        let formatted = fmt.format(&[msg]);
        assert_eq!(formatted[0]["reasoning_content"], "reasoning");
        assert_eq!(formatted[0]["content"], "answer");

        let parsed = fmt
            .parse_response(json!({
                "id": "chatcmpl-1",
                "created": 1,
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "reasoning_content": "reasoning",
                        "content": "answer"
                    }
                }]
            }))
            .unwrap();
        assert_eq!(parsed.get_thinking()[0].thinking, "reasoning");
        assert_eq!(parsed.get_text(), "answer");
    }

    #[test]
    fn test_openai_parse_response_text() {
        let raw = json!({
            "id": "chatcmpl-abc",
            "created": 1700000000,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                }
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });
        let fmt = OpenAiFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "chatcmpl-abc");
        assert_eq!(resp.get_text(), "Hello!");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn test_openai_parse_response_tool_calls() {
        let raw = json!({
            "id": "chatcmpl-xyz",
            "created": 1700000001,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "my_tool",
                            "arguments": "{\"x\": 1}"
                        }
                    }]
                }
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
        });
        let fmt = OpenAiFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert!(resp.has_tool_calls());
        let calls = resp.get_tool_calls();
        assert_eq!(calls[0].name, "my_tool");
        assert_eq!(calls[0].input["x"], 1);
    }

    // -----------------------------------------------------------------------
    // DashScopeFormatter
    // -----------------------------------------------------------------------

    #[test]
    fn test_dashscope_format_matches_openai() {
        let msg = Msg::user("user", "hi");
        let openai = OpenAiFormatter;
        let dash = DashScopeFormatter;
        assert_eq!(openai.format(&[msg.clone()]), dash.format(&[msg]));
    }

    #[test]
    fn test_dashscope_parse_openai_compatible() {
        let raw = json!({
            "id": "ds-1",
            "created": 1700000002,
            "choices": [{
                "message": {"role": "assistant", "content": "ok"}
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "ok");
    }

    #[test]
    fn test_dashscope_parse_native_format() {
        let raw = json!({
            "request_id": "ds-native-1",
            "output": {
                "choices": [{
                    "message": {"role": "assistant", "content": "native response"}
                }]
            },
            "usage": {
                "input_tokens": 8,
                "output_tokens": 3,
                "total_tokens": 11
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-native-1");
        assert_eq!(resp.get_text(), "native response");
        assert_eq!(resp.usage.input_tokens, 8);
    }

    // -----------------------------------------------------------------------
    // AnthropicFormatter
    // -----------------------------------------------------------------------

    #[test]
    fn test_anthropic_extract_system() {
        let msgs = vec![Msg::system("Be helpful."), Msg::user("alice", "hi")];
        let sys = AnthropicFormatter::extract_system(&msgs);
        assert_eq!(sys.as_deref(), Some("Be helpful."));
    }

    #[test]
    fn test_anthropic_format_excludes_system() {
        let msgs = vec![Msg::system("Be helpful."), Msg::user("alice", "hi")];
        let fmt = AnthropicFormatter;
        let result = fmt.format(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["role"], "user");
    }

    #[test]
    fn test_anthropic_format_user_text() {
        let msg = Msg::user("alice", "hello");
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg]);
        assert_eq!(result[0]["role"], "user");
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "hello");
    }

    #[test]
    fn test_anthropic_format_tool_use() {
        let blocks = vec![ContentBlock::ToolUse(ToolUseBlock {
            id: "tu_1".into(),
            name: "calculator".into(),
            input: json!({"expr": "1+1"}),
            raw_input: None,
        })];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_use");
        assert_eq!(content[0]["id"], "tu_1");
        assert_eq!(content[0]["name"], "calculator");
    }

    #[test]
    fn test_anthropic_format_tool_result() {
        let blocks = vec![ContentBlock::ToolResult(ToolResultBlock {
            tool_use_id: "tu_1".into(),
            output: "2".into(),
            name: Some("calculator".into()),
            is_error: false,
        })];
        let msg = Msg::new("calculator", MsgContent::Blocks(blocks), Role::Tool);
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "tu_1");
        assert_eq!(content[0]["content"], "2");
    }

    #[test]
    fn test_anthropic_format_thinking_preserved() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "hmm".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "answer".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[1]["type"], "text");
    }

    #[test]
    fn test_anthropic_parse_response_text() {
        let raw = json!({
            "id": "msg-123",
            "content": [{"type": "text", "text": "Hello from Claude"}],
            "usage": {"input_tokens": 10, "output_tokens": 4}
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "msg-123");
        assert_eq!(resp.get_text(), "Hello from Claude");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 4);
        assert_eq!(resp.usage.total_tokens, 14);
    }

    #[test]
    fn test_anthropic_parse_response_tool_use() {
        let raw = json!({
            "id": "msg-456",
            "content": [{
                "type": "tool_use",
                "id": "toolu_1",
                "name": "search",
                "input": {"query": "rust lang"}
            }],
            "usage": {"input_tokens": 20, "output_tokens": 8}
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert!(resp.has_tool_calls());
        let calls = resp.get_tool_calls();
        assert_eq!(calls[0].id, "toolu_1");
        assert_eq!(calls[0].name, "search");
    }

    #[test]
    fn test_anthropic_parse_response_thinking() {
        let raw = json!({
            "id": "msg-789",
            "content": [
                {"type": "thinking", "thinking": "let me think"},
                {"type": "text", "text": "result"}
            ],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        let thinking = resp.get_thinking();
        assert_eq!(thinking.len(), 1);
        assert_eq!(thinking[0].thinking, "let me think");
        assert_eq!(resp.get_text(), "result");
    }

    // -----------------------------------------------------------------------
    // Robustness / fault-tolerance tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_openai_parse_missing_choices() {
        // Response JSON has no "choices" field at all — returns empty content.
        let raw = json!({
            "id": "chatcmpl-no-choices",
            "created": 1700000000,
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        });
        let fmt = OpenAiFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert!(resp.content.is_empty());
        assert_eq!(resp.id, "chatcmpl-no-choices");
    }

    #[test]
    fn test_openai_parse_invalid_tool_args() {
        // tool_calls[0].function.arguments is not valid JSON.
        let raw = json!({
            "id": "chatcmpl-bad-args",
            "created": 1700000000,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_bad",
                        "type": "function",
                        "function": {
                            "name": "broken_tool",
                            "arguments": "NOT VALID JSON {{{{"
                        }
                    }]
                }
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        });
        let fmt = OpenAiFormatter;
        // Should not panic — gracefully falls back to empty object.
        let resp = fmt.parse_response(raw).unwrap();
        assert!(resp.has_tool_calls());
        let calls = resp.get_tool_calls();
        assert_eq!(calls[0].name, "broken_tool");
        // input falls back to {"_raw": "<original string>"}
        assert_eq!(calls[0].input, json!({"_raw": "NOT VALID JSON {{{{"}));
        // raw_input preserves original string
        assert_eq!(calls[0].raw_input.as_deref(), Some("NOT VALID JSON {{{{"));
    }

    #[test]
    fn test_openai_format_empty_messages() {
        let fmt = OpenAiFormatter;
        let result = fmt.format(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_anthropic_extract_system_none() {
        // No system message present.
        let msgs = vec![Msg::user("alice", "hello"), Msg::assistant("bot", "hi")];
        let sys = AnthropicFormatter::extract_system(&msgs);
        assert!(sys.is_none());
    }

    #[test]
    fn test_anthropic_thinking_preserved() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "deep thoughts".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "final answer".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg]);
        let content = result[0]["content"].as_array().unwrap();
        // ThinkingBlock must be preserved in Anthropic format.
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "deep thoughts");
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "final answer");
    }

    #[test]
    fn test_anthropic_image_base64_and_url() {
        // Base64 image
        let base64_blocks = vec![ContentBlock::Image(ImageBlock {
            data: Some("aGVsbG8=".into()),
            url: None,
            mime_type: Some("image/jpeg".into()),
        })];
        let msg_b64 = Msg::user("alice", MsgContent::Blocks(base64_blocks));
        let fmt = AnthropicFormatter;
        let result = fmt.format(&[msg_b64]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "base64");
        assert_eq!(content[0]["source"]["media_type"], "image/jpeg");
        assert_eq!(content[0]["source"]["data"], "aGVsbG8=");

        // URL image
        let url_blocks = vec![ContentBlock::Image(ImageBlock {
            data: None,
            url: Some("https://example.com/photo.png".into()),
            mime_type: None,
        })];
        let msg_url = Msg::user("alice", MsgContent::Blocks(url_blocks));
        let result = fmt.format(&[msg_url]);
        let content = result[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "url");
        assert_eq!(content[0]["source"]["url"], "https://example.com/photo.png");
    }

    #[test]
    fn test_dashscope_native_response_format() {
        let raw = json!({
            "request_id": "ds-native-test",
            "output": {
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "native ok"
                    }
                }]
            },
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-native-test");
        assert_eq!(resp.get_text(), "native ok");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn test_dashscope_openai_compat_format() {
        let raw = json!({
            "id": "ds-compat-test",
            "created": 1700000000,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "compat ok"
                }
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 6,
                "total_tokens": 18
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-compat-test");
        assert_eq!(resp.get_text(), "compat ok");
        assert_eq!(resp.usage.input_tokens, 12);
        assert_eq!(resp.usage.output_tokens, 6);
    }

    #[test]
    fn test_format_mixed_content_blocks() {
        // A message with Text + ToolUse + Image blocks.
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Here is the result.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tu_mix".into(),
                name: "search".into(),
                input: json!({"q": "test"}),
                raw_input: None,
            }),
            ContentBlock::Image(ImageBlock {
                url: Some("https://img.example.com/pic.png".into()),
                data: None,
                mime_type: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));

        // OpenAI: tool_calls present, text present, image not in tool_calls path
        let openai = OpenAiFormatter;
        let openai_result = openai.format(&[msg.clone()]);
        assert_eq!(openai_result[0]["role"], "assistant");
        let tool_calls = openai_result[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["function"]["name"], "search");

        // Anthropic: all blocks appear in content array
        let anthropic = AnthropicFormatter;
        let anthropic_result = anthropic.format(&[msg.clone()]);
        let content = anthropic_result[0]["content"].as_array().unwrap();
        let types: Vec<&str> = content
            .iter()
            .map(|c| c["type"].as_str().unwrap())
            .collect();
        assert!(types.contains(&"text"));
        assert!(types.contains(&"tool_use"));
        assert!(types.contains(&"image"));

        // DashScope: same as OpenAI
        let dashscope = DashScopeFormatter;
        let ds_result = dashscope.format(&[msg]);
        assert_eq!(ds_result[0]["role"], "assistant");
        assert!(ds_result[0]["tool_calls"].as_array().unwrap().len() == 1);
    }

    // -----------------------------------------------------------------------
    // Additional robustness tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_openai_parse_null_content_with_tool_calls() {
        // OpenAI sends content=null when there are tool_calls.
        let raw = json!({
            "id": "chatcmpl-null-content",
            "created": 1700000000,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_100",
                        "type": "function",
                        "function": {
                            "name": "my_tool",
                            "arguments": "{\"key\": \"value\"}"
                        }
                    }]
                }
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
        });
        let fmt = OpenAiFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        // No text block (content was null).
        assert_eq!(resp.get_text(), "");
        // Tool call should be present.
        assert!(resp.has_tool_calls());
        let calls = resp.get_tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "my_tool");
        assert_eq!(calls[0].input["key"], "value");
    }

    #[test]
    fn test_openai_parse_empty_choices_array() {
        // choices exists but is empty — returns empty content.
        let raw = json!({
            "id": "chatcmpl-empty-choices",
            "created": 1700000000,
            "choices": [],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });
        let fmt = OpenAiFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert!(resp.content.is_empty());
    }

    #[test]
    fn test_dashscope_parse_missing_usage() {
        // DashScope native format without usage field.
        let raw = json!({
            "request_id": "ds-no-usage",
            "output": {
                "choices": [{
                    "message": {"role": "assistant", "content": "no usage here"}
                }]
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "no usage here");
        // Usage should default to zeros.
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);
        assert_eq!(resp.usage.total_tokens, 0);
    }

    #[test]
    fn test_anthropic_parse_unknown_content_type() {
        // Content array has an unknown type — it should be skipped.
        let raw = json!({
            "id": "msg-unknown-type",
            "content": [
                {"type": "text", "text": "Hello"},
                {"type": "server_tool_use", "id": "st_1", "name": "web"},
                {"type": "text", "text": " World"}
            ],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        // Only the two text blocks survive; the unknown type is skipped.
        assert_eq!(resp.content.len(), 2);
        assert_eq!(resp.get_text(), "Hello World");
    }

    #[test]
    fn test_anthropic_parse_missing_usage() {
        // Anthropic response without usage field.
        let raw = json!({
            "id": "msg-no-usage",
            "content": [{"type": "text", "text": "ok"}]
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "ok");
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);
    }

    #[test]
    fn test_anthropic_parse_missing_stop_reason() {
        // stop_reason field is absent — should not panic.
        let raw = json!({
            "id": "msg-no-stop",
            "content": [{"type": "text", "text": "partial"}],
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });
        let fmt = AnthropicFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "partial");
    }
}
