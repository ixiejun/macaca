// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! AgentScope 2.0-equivalent message metadata helpers.
//!
//! These helpers add stable ids, tool states, usage, and generate reasons
//! without changing existing message block structs into breaking required-field
//! shapes. Explicit upstream ids are preserved; otherwise a deterministic
//! content hash supplies a stable projection id for replay and protocol
//! adapters.

use serde::{Deserialize, Serialize};

use crate::message::{ContentBlock, Msg};

const GENERATE_REASON_KEY: &str = "generate_reason";
const USAGE_KEY: &str = "usage";

/// Provider-neutral reason a generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerateReason {
    EndTurn,
    ToolCalls,
    MaxIterations,
    Interrupted,
    UserConfirmRequired,
    ExternalExecutionRequired,
    Denied,
    GracefulShutdown,
    StructuredOutputRetry,
}

/// Tool-call lifecycle state projected from AgentScope 2.0 semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallState {
    Started,
    Streaming,
    Completed,
    Suspended,
    Cancelled,
}

/// Tool-result lifecycle state projected from AgentScope 2.0 semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultState {
    Started,
    Streaming,
    Completed,
    Error,
}

/// Provider-neutral token and latency usage metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl ContentBlock {
    /// Return a stable block id for replay, projection, and protocol adapters.
    ///
    /// AgentScope 2.0 event-originated blocks can carry explicit ids. Legacy
    /// Macaca blocks may not, so this method derives a deterministic id from
    /// sanitized serialized block content. The hash is stable for the same
    /// serialized block and avoids leaking raw content in the id itself.
    pub fn stable_block_id(&self) -> String {
        match self {
            ContentBlock::ToolUse(block) => block.id.clone(),
            ContentBlock::ToolResult(block) => format!("tool_result:{}", block.tool_use_id),
            ContentBlock::Data(block) => block
                .id
                .clone()
                .unwrap_or_else(|| stable_hash_id("data", self)),
            ContentBlock::Hint(block) => block
                .id
                .clone()
                .unwrap_or_else(|| stable_hash_id("hint", self)),
            ContentBlock::Text(_) => stable_hash_id("text", self),
            ContentBlock::Thinking(_) => stable_hash_id("thinking", self),
            ContentBlock::Image(_) => stable_hash_id("image", self),
            ContentBlock::Audio(_) => stable_hash_id("audio", self),
            ContentBlock::Video(_) => stable_hash_id("video", self),
        }
    }

    /// Return the best-known tool call state for this block.
    pub fn tool_call_state(&self) -> Option<ToolCallState> {
        match self {
            ContentBlock::ToolUse(_) => Some(ToolCallState::Completed),
            _ => None,
        }
    }

    /// Return the best-known tool result state for this block.
    pub fn tool_result_state(&self) -> Option<ToolResultState> {
        match self {
            ContentBlock::ToolResult(block) if block.is_error => Some(ToolResultState::Error),
            ContentBlock::ToolResult(_) => Some(ToolResultState::Completed),
            _ => None,
        }
    }
}

impl Msg {
    /// Attach AgentScope 2.0-equivalent generate reason metadata.
    pub fn with_generate_reason(mut self, reason: GenerateReason) -> Self {
        ensure_metadata_object(&mut self);
        if let Some(object) = self.metadata.as_object_mut() {
            object.insert(
                GENERATE_REASON_KEY.to_string(),
                serde_json::to_value(reason).unwrap_or(serde_json::Value::Null),
            );
        }
        self
    }

    /// Read AgentScope 2.0-equivalent generate reason metadata.
    pub fn generate_reason(&self) -> Option<GenerateReason> {
        self.metadata
            .get(GENERATE_REASON_KEY)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// Attach provider-neutral usage metadata.
    pub fn with_usage(mut self, usage: MessageUsage) -> Self {
        ensure_metadata_object(&mut self);
        if let Some(object) = self.metadata.as_object_mut() {
            object.insert(
                USAGE_KEY.to_string(),
                serde_json::to_value(usage).unwrap_or(serde_json::Value::Null),
            );
        }
        self
    }

    /// Read provider-neutral usage metadata.
    pub fn usage(&self) -> Option<MessageUsage> {
        self.metadata
            .get(USAGE_KEY)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

fn ensure_metadata_object(message: &mut Msg) {
    if !message.metadata.is_object() {
        message.metadata = serde_json::json!({});
    }
}

fn stable_hash_id(prefix: &str, block: &ContentBlock) -> String {
    let serialized = serde_json::to_string(block).unwrap_or_else(|_| format!("{block:?}"));
    format!("{prefix}:{:016x}", fnv1a64(serialized.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
