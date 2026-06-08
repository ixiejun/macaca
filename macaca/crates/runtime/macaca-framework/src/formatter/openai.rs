//! OpenAI Chat Completions formatter (**Strategy** / Adapter).
//!
//! Maps framework [`Msg`] values to the `/chat/completions` JSON message array and
//! parses responses back into unified [`ChatResponse`] envelopes.

use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::message::{ContentBlock, Msg, MsgContent, Role};
use crate::model::ChatResponse;

use super::error::FormatterError;
use super::openai_common::{
    blocks_to_openai_content, extract_tool_result_blocks, extract_tool_use_blocks,
    parse_openai_response,
};
use super::core::Formatter;

/// Formats messages for the OpenAI Chat Completions API.
///
/// - `system` role → `{"role": "system", "content": "..."}`
/// - `user` role → `{"role": "user", "content": "text" | [parts]}`
/// - `assistant` with tool calls → `{"role": "assistant", "tool_calls": [...], "content": null}`
/// - `tool` role → `{"role": "tool", "tool_call_id": "...", "content": "..."}`
pub struct OpenAiFormatter;

impl Formatter for OpenAiFormatter {
    fn format(&self, msgs: &[Msg]) -> Vec<Value> {
        debug!(
            target: "macaca_framework::formatter::openai",
            message_count = msgs.len(),
            "OpenAiFormatter::format entry — encoding framework Msg slice to Chat Completions JSON"
        );
        msgs.iter().map(|msg| format_openai_msg(msg)).collect()
    }

    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError> {
        debug!(
            target: "macaca_framework::formatter::openai",
            "OpenAiFormatter::parse_response entry — decoding provider JSON into ChatResponse"
        );
        match parse_openai_response(raw) {
            Ok(response) => {
                debug!(
                    target: "macaca_framework::formatter::openai",
                    block_count = response.content.len(),
                    response_id = %response.id,
                    "OpenAiFormatter::parse_response succeeded"
                );
                Ok(response)
            }
            Err(err) => {
                warn!(
                    target: "macaca_framework::formatter::openai",
                    error = %err,
                    "OpenAiFormatter::parse_response failed"
                );
                Err(err)
            }
        }
    }
}

pub(crate) fn format_openai_msg(msg: &Msg) -> Value {
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
