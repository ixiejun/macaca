// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Harness context compaction and memory service contracts.
//!
//! These contracts model AgentScope 2.0 Harness memory behavior without taking
//! ownership of storage, filesystem, or sandbox side effects. The framework can
//! compact in-memory message state and describe memory tools; runtime-host
//! services decide where summaries, evicted results, and durable memories live.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::agent::AgentResult;
use crate::message::{ContentBlock, Msg, MsgContent, TextBlock, ToolUseBlock};
use crate::runtime_context::{AgentState, RuntimeContext};

/// Bounded context policy used before formatting model input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessContextPolicy {
    pub max_context_chars: usize,
    pub max_tool_result_chars: usize,
    pub max_tool_argument_chars: usize,
}

impl Default for HarnessContextPolicy {
    fn default() -> Self {
        Self {
            max_context_chars: 64 * 1024,
            max_tool_result_chars: 8 * 1024,
            max_tool_argument_chars: 8 * 1024,
        }
    }
}

/// Result of one provider-neutral compaction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessContextCompactionResult {
    pub state: AgentState,
    pub evicted_tool_results: Vec<String>,
    pub truncated_tool_arguments: Vec<String>,
    pub overflow_retry_required: bool,
}

/// Stateless compaction manager. It uses simple bounded transformations so
/// callers can run it before model formatting without service side effects.
pub struct HarnessContextManager {
    policy: HarnessContextPolicy,
}

impl HarnessContextManager {
    pub fn new(policy: HarnessContextPolicy) -> Self {
        Self { policy }
    }

    pub fn compact(
        &self,
        runtime: &RuntimeContext,
        state: &AgentState,
    ) -> AgentResult<HarnessContextCompactionResult> {
        debug!(
            trace_id = %runtime.trace_id,
            max_context_chars = self.policy.max_context_chars,
            "harness context compaction started"
        );
        let mut compacted = state.clone();
        let mut evicted_tool_results = Vec::new();
        let mut truncated_tool_arguments = Vec::new();
        for message in &mut compacted.conversation {
            compact_message(
                message,
                &self.policy,
                &mut evicted_tool_results,
                &mut truncated_tool_arguments,
            );
        }
        let overflow_retry_required =
            conversation_text_len(&compacted.conversation) > self.policy.max_context_chars;
        if overflow_retry_required {
            warn!(
                trace_id = %runtime.trace_id,
                "harness context still exceeds budget after compaction"
            );
        }
        Ok(HarnessContextCompactionResult {
            state: compacted,
            evicted_tool_results,
            truncated_tool_arguments,
            overflow_retry_required,
        })
    }
}

/// Command sent to a memory service when Harness flushes transient memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessMemoryFlushCommand {
    pub runtime: RuntimeContext,
    pub state: AgentState,
}

/// Result returned by the memory service boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessMemoryFlushResult {
    pub flushed: bool,
    pub evidence_ref: Option<String>,
    pub reason: Option<String>,
}

/// Replaceable memory port. Implementations may be local, remote, plugin,
/// mock, or unavailable service adapters.
#[async_trait]
pub trait HarnessMemoryPort: Send + Sync {
    fn name(&self) -> &str;
    async fn flush(
        &self,
        command: HarnessMemoryFlushCommand,
    ) -> AgentResult<HarnessMemoryFlushResult>;
}

/// Null Object memory port for absent memory providers.
pub struct UnavailableHarnessMemoryPort {
    reason: String,
}

impl UnavailableHarnessMemoryPort {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessMemoryPort for UnavailableHarnessMemoryPort {
    fn name(&self) -> &str {
        "unavailable_harness_memory"
    }

    async fn flush(
        &self,
        command: HarnessMemoryFlushCommand,
    ) -> AgentResult<HarnessMemoryFlushResult> {
        warn!(
            trace_id = %command.runtime.trace_id,
            reason = %self.reason,
            "harness memory flush unavailable"
        );
        Ok(HarnessMemoryFlushResult {
            flushed: false,
            evidence_ref: None,
            reason: Some(self.reason.clone()),
        })
    }
}

/// Descriptor for memory-related tools exposed by service adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessMemoryToolDescriptor {
    pub capability_id: String,
    pub description: String,
}

impl HarnessMemoryToolDescriptor {
    pub fn default_descriptors() -> Vec<Self> {
        vec![
            Self {
                capability_id: "harness.memory.search".to_string(),
                description: "Search authorized harness memory through the memory service."
                    .to_string(),
            },
            Self {
                capability_id: "harness.memory.flush".to_string(),
                description: "Flush transient harness memory through the memory service."
                    .to_string(),
            },
        ]
    }
}

fn compact_message(
    message: &mut Msg,
    policy: &HarnessContextPolicy,
    evicted_tool_results: &mut Vec<String>,
    truncated_tool_arguments: &mut Vec<String>,
) {
    let MsgContent::Blocks(blocks) = &mut message.content else {
        return;
    };
    for block in blocks {
        match block {
            ContentBlock::ToolResult(result)
                if result.output.len() > policy.max_tool_result_chars =>
            {
                evicted_tool_results.push(result.tool_use_id.clone());
                result.output = format!(
                    "[tool result evicted: {} chars; use memory/context service evidence]",
                    result.output.len()
                );
            }
            ContentBlock::ToolUse(tool_use) => {
                maybe_truncate_tool_arguments(tool_use, policy, truncated_tool_arguments);
            }
            ContentBlock::Text(TextBlock { text }) if text.len() > policy.max_context_chars => {
                text.truncate(policy.max_context_chars);
                text.push_str("\n[message text truncated]");
            }
            _ => {}
        }
    }
}

fn maybe_truncate_tool_arguments(
    tool_use: &mut ToolUseBlock,
    policy: &HarnessContextPolicy,
    truncated_tool_arguments: &mut Vec<String>,
) {
    let encoded = tool_use.input.to_string();
    if encoded.len() <= policy.max_tool_argument_chars {
        return;
    }
    truncated_tool_arguments.push(tool_use.id.clone());
    tool_use.raw_input = Some(
        encoded
            .chars()
            .take(policy.max_tool_argument_chars)
            .collect(),
    );
    tool_use.input = serde_json::json!({
        "truncated": true,
        "original_chars": encoded.len()
    });
}

fn conversation_text_len(messages: &[Msg]) -> usize {
    messages
        .iter()
        .map(Msg::get_text)
        .map(|text| text.len())
        .sum()
}
