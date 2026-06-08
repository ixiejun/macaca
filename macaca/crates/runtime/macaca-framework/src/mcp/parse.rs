//! Parse MCP `tools/call` JSON-RPC results into framework [`ContentBlock`] slices.
//!
//! Shared by stdio and HTTP transports so multimodal content decoding stays DRY.

use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

use super::error::McpError;
use super::types::McpCallResult;

/// Decode a MCP `tools/call` `result` object into a normalized [`McpCallResult`].
///
/// Handles text, image, audio, resource, and unknown block types with JSON fallbacks
/// so operator traces remain inspectable even when servers emit experimental shapes.
pub(crate) fn parse_call_result(result: &Value) -> Result<McpCallResult, McpError> {
    tracing::debug!(
        target = "macaca_framework::mcp::parse",
        has_content = result.get("content").is_some(),
        "parsing MCP tools/call result"
    );

    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let content_arr = result
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut blocks = Vec::new();
    for item in &content_arr {
        let block_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("text");
        match block_type {
            "text" => {
                let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                blocks.push(ContentBlock::Text(TextBlock {
                    text: text.to_string(),
                }));
            }
            "image" => {
                blocks.push(ContentBlock::Image(crate::message::ImageBlock {
                    data: item
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    mime_type: item
                        .get("mimeType")
                        .or_else(|| item.get("mime_type"))
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                }));
            }
            "audio" => {
                blocks.push(ContentBlock::Audio(crate::message::AudioBlock {
                    data: item
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    mime_type: item
                        .get("mimeType")
                        .or_else(|| item.get("mime_type"))
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                }));
            }
            "resource" => {
                let text = item
                    .get("resource")
                    .and_then(|resource| {
                        resource
                            .get("text")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string)
                            .or_else(|| serde_json::to_string(resource).ok())
                    })
                    .or_else(|| serde_json::to_string(item).ok())
                    .unwrap_or_default();
                blocks.push(ContentBlock::Text(TextBlock { text }));
            }
            _ => {
                // Fallback: wrap the whole item as JSON text for auditability.
                blocks.push(ContentBlock::Text(TextBlock {
                    text: serde_json::to_string(item).unwrap_or_default(),
                }));
            }
        }
    }

    if blocks.is_empty() {
        blocks.push(ContentBlock::Text(TextBlock {
            text: String::new(),
        }));
    }

    let metadata = result.get("_meta").cloned();

    if is_error {
        tracing::warn!(
            target = "macaca_framework::mcp::parse",
            block_count = blocks.len(),
            "MCP tool call returned isError=true"
        );
    }

    Ok(McpCallResult {
        content: blocks,
        is_error,
        metadata,
    })
}
