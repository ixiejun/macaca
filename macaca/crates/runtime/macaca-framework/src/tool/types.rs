// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Provider-neutral tool DTOs.
//!
//! These values are intentionally independent from concrete tool providers.
//! Framework adapters, runtime-host services, and shell bridges can translate
//! provider-specific outputs into this bounded shape before emitting traces or
//! returning model-visible content.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

/// Provider-neutral trace event emitted during delegated tool execution.
///
/// Framework owns this DTO so `Toolkit` can route trace evidence without
/// depending on concrete tool-service crates. Shell or runtime-host adapters
/// are responsible for converting service-specific trace events into this
/// bounded, sanitized shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolTraceEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// The result returned by a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResponse {
    /// Content blocks produced by the tool.
    pub content: Vec<ContentBlock>,
    /// Optional arbitrary metadata such as bounded latency or token counts.
    pub metadata: Option<Value>,
    /// Whether the response is a streaming chunk.
    pub is_stream: bool,
    /// Whether this is the final chunk of a stream.
    pub is_last: bool,
    /// Whether the execution was interrupted mid-stream.
    pub is_interrupted: bool,
}

impl ToolResponse {
    /// Construct a plain-text response.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock { text: s.into() })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct an error response.
    ///
    /// The response still uses a text block because callers distinguish hard
    /// failures through `Result::Err`. This constructor is for handlers that
    /// need to return an explicit, model-visible error payload.
    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: format!("Error: {}", s.into()),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct a response whose text is the JSON-serialized form of `value`.
    pub fn json(value: Value) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: value.to_string(),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }
}

/// Errors that can occur during tool lookup or execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout")]
    Timeout,
}
