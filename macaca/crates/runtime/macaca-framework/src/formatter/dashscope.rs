//! DashScope (OpenAI-compatible variant) formatter (**Strategy** / Adapter).
//!
//! Reuses OpenAI message encoding; response parsing branches on native vs
//! compatible endpoint wire shapes.

use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::message::{ContentBlock, Msg, TextBlock, ToolUseBlock};
use crate::model::{ChatResponse, ChatUsage};

use super::error::FormatterError;
use super::openai::format_openai_msg;
use super::openai_common::parse_openai_response;
use super::core::Formatter;

/// Formats messages for the Alibaba DashScope API.
///
/// DashScope's Qwen models use an OpenAI-compatible interface with minor
/// differences in response field names (`input.choices` vs top-level `choices`,
/// `usage.input_tokens` vs `usage.prompt_tokens`).
pub struct DashScopeFormatter;

impl Formatter for DashScopeFormatter {
    fn format(&self, msgs: &[Msg]) -> Vec<Value> {
        debug!(
            target: "macaca_framework::formatter::dashscope",
            message_count = msgs.len(),
            "DashScopeFormatter::format entry — reusing OpenAI wire encoding for messages"
        );
        // Message format is identical to OpenAI
        msgs.iter().map(|msg| format_openai_msg(msg)).collect()
    }

    fn parse_response(&self, raw: Value) -> Result<ChatResponse, FormatterError> {
        debug!(
            target: "macaca_framework::formatter::dashscope",
            has_top_level_choices = raw.get("choices").is_some(),
            "DashScopeFormatter::parse_response entry — selecting OpenAI-compatible or native decoder"
        );
        // DashScope wraps choices in `output` for non-compatible endpoint,
        // but the OpenAI-compatible endpoint matches OpenAI exactly.
        // We try OpenAI format first, then fall back to DashScope native.
        if raw.get("choices").is_some() {
            debug!(
                target: "macaca_framework::formatter::dashscope",
                "DashScopeFormatter::parse_response using OpenAI-compatible path"
            );
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

        debug!(
            target: "macaca_framework::formatter::dashscope",
            "DashScopeFormatter::parse_response using native output.choices path"
        );

        let output = raw
            .get("output")
            .ok_or_else(|| {
                warn!(
                    target: "macaca_framework::formatter::dashscope",
                    "DashScope native response missing top-level output field"
                );
                FormatterError::Parse("missing output".into())
            })?;

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

        let response = ChatResponse {
            content: content_blocks,
            id,
            created_at: String::new(),
            usage,
            metadata: None,
        };
        debug!(
            target: "macaca_framework::formatter::dashscope",
            block_count = response.content.len(),
            response_id = %response.id,
            "DashScopeFormatter::parse_response succeeded via native path"
        );
        Ok(response)
    }
}
