// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! MCP content conversion contracts.
//!
//! MCP services own protocol transport and tool execution. The framework owns
//! only the deterministic mapping from service-returned content descriptors into
//! `ContentBlock` and `ToolResult` values used by agent events and messages.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::message::{ContentBlock, DataBlock, ImageBlock, TextBlock, ToolResultBlock};
use crate::tool_contract::ToolExecutionResult;

const MAX_MCP_TEXT_CHARS: usize = 64 * 1024;

/// Provider-neutral MCP content value returned by a delegated MCP service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpContentValue {
    Text {
        text: String,
    },
    Resource {
        uri: String,
        mime_type: Option<String>,
        text: Option<String>,
    },
    Image {
        data: Option<String>,
        url: Option<String>,
        mime_type: Option<String>,
    },
    Binary {
        mime_type: Option<String>,
        size_bytes: usize,
        evidence_ref: Option<String>,
    },
    Structured {
        name: Option<String>,
        value: serde_json::Value,
    },
}

/// Result of converting MCP content into framework message content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpContentConversionResult {
    pub blocks: Vec<ContentBlock>,
    pub diagnostics: Vec<String>,
}

/// Convert MCP content values into bounded framework content blocks.
pub fn convert_mcp_content(values: Vec<McpContentValue>) -> McpContentConversionResult {
    debug!(
        content_count = values.len(),
        "converting mcp content values into framework content blocks"
    );
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    for value in values {
        match value {
            McpContentValue::Text { text } => {
                blocks.push(ContentBlock::Text(TextBlock {
                    text: bounded_text(text, &mut diagnostics),
                }));
            }
            McpContentValue::Resource {
                uri,
                mime_type,
                text,
            } => {
                blocks.push(ContentBlock::Data(DataBlock {
                    id: None,
                    name: Some("mcp_resource".to_string()),
                    data: serde_json::json!({
                        "uri": uri,
                        "mime_type": mime_type,
                        "text": text.map(|value| bounded_text(value, &mut diagnostics)),
                    }),
                }));
            }
            McpContentValue::Image {
                data,
                url,
                mime_type,
            } => blocks.push(ContentBlock::Image(ImageBlock {
                data,
                url,
                mime_type,
            })),
            McpContentValue::Binary {
                mime_type,
                size_bytes,
                evidence_ref,
            } => blocks.push(ContentBlock::Data(DataBlock {
                id: None,
                name: Some("mcp_binary".to_string()),
                data: serde_json::json!({
                    "mime_type": mime_type,
                    "size_bytes": size_bytes,
                    "evidence_ref": evidence_ref,
                    "payload_omitted": true,
                }),
            })),
            McpContentValue::Structured { name, value } => {
                blocks.push(ContentBlock::Data(DataBlock {
                    id: None,
                    name,
                    data: value,
                }));
            }
        }
    }
    McpContentConversionResult {
        blocks,
        diagnostics,
    }
}

/// Convert MCP content into the canonical tool execution result.
pub fn convert_mcp_tool_result(values: Vec<McpContentValue>) -> ToolExecutionResult {
    let converted = convert_mcp_content(values);
    ToolExecutionResult::Completed {
        content: converted.blocks,
        metadata: Some(serde_json::json!({
            "conversion_diagnostics": converted.diagnostics,
        })),
        is_error: false,
    }
}

/// Convert MCP content into a single tool-result block for message roles that
/// require `Role::Tool` content. This keeps MCP output projection deterministic
/// without exposing raw binary payloads.
pub fn convert_mcp_tool_result_block(
    tool_use_id: impl Into<String>,
    name: Option<String>,
    values: Vec<McpContentValue>,
) -> ToolResultBlock {
    let converted = convert_mcp_content(values);
    ToolResultBlock {
        tool_use_id: tool_use_id.into(),
        output: converted
            .blocks
            .iter()
            .map(|block| format!("{block:?}"))
            .collect::<Vec<_>>()
            .join("\n"),
        name,
        is_error: false,
    }
}

fn bounded_text(mut text: String, diagnostics: &mut Vec<String>) -> String {
    if text.len() <= MAX_MCP_TEXT_CHARS {
        return text;
    }
    let original_len = text.len();
    text.truncate(MAX_MCP_TEXT_CHARS);
    text.push_str("\n[mcp text truncated]");
    diagnostics.push(format!(
        "mcp text content truncated from {original_len} chars to {MAX_MCP_TEXT_CHARS} chars"
    ));
    text
}

#[cfg(test)]
#[path = "mcp_content_contract_tests.rs"]
mod mcp_content_contract_tests;
