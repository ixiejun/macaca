// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It keeps event data
// provider-neutral so shells, gateways, protocol adapters, and future framework
// providers can consume the same contract without depending on AgentScope Java.

//! Provider-neutral AgentScope 2.0-style event contract.
//!
//! AgentScope 2.0 exposes execution as a typed event stream and reconstructs the
//! final assistant message from that stream. Macaca uses the same operating
//! model here, but the types are intentionally generic OS contracts:
//!
//! - **Observer**: `AgentEvent` is a durable fact that downstream adapters can
//!   observe without owning execution semantics.
//! - **Command/Event Sourcing**: every lifecycle, model, content, tool, and
//!   human-in-the-loop transition is represented as a typed fact.
//! - **Memento**: `AgentEventAccumulator` projects a bounded final `Msg` from
//!   the observed facts and can be used by tests, protocol adapters, or replay.
//! - **Builder**: event constructors provide small, explicit creation helpers
//!   without forcing callers into provider-specific structs.

#[path = "event_contract_projection.rs"]
mod event_contract_projection;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

use event_contract_projection::BlockDraft;

use crate::message::{Msg, MsgContent};

/// AgentScope 2.0-style event kinds supported by the Macaca framework contract.
///
/// These names are intentionally descriptive rather than provider-specific.
/// Protocol adapters can map them to SSE, AG-UI, Chat Completions, A2A, or
/// Agent Protocol frames while preserving the same execution facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    AgentStart,
    AgentEnd,
    ExceedMaxIters,
    RequestStop,
    Interrupted,
    ModelCallStart,
    ModelCallEnd,
    TextBlockStart,
    TextBlockDelta,
    TextBlockEnd,
    ThinkingBlockStart,
    ThinkingBlockDelta,
    ThinkingBlockEnd,
    DataBlockStart,
    DataBlockDelta,
    DataBlockEnd,
    ToolCallStart,
    ToolCallDelta,
    ToolCallEnd,
    ToolResultStart,
    ToolResultTextDelta,
    ToolResultDataDelta,
    ToolResultEnd,
    RequireUserConfirm,
    UserConfirmResult,
    RequireExternalExecution,
    ExternalExecutionResult,
}

/// Structured event payload.
///
/// Payloads are sanitized contract values. Implementations must avoid placing
/// credentials, raw prompts, unbounded provider output, package bytes, or other
/// secret material in these fields because events may be logged, replayed, or
/// projected into shell transports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "payload_type", rename_all = "snake_case")]
pub enum AgentEventPayload {
    Empty,
    Lifecycle {
        agent_name: String,
        reason: Option<String>,
    },
    ModelCall {
        model_id: Option<String>,
        request_id: Option<String>,
        usage: Option<serde_json::Value>,
        reason: Option<String>,
    },
    TextDelta {
        delta: String,
    },
    ThinkingDelta {
        delta: String,
    },
    DataDelta {
        data: serde_json::Value,
    },
    ToolCall {
        tool_call_id: String,
        name: String,
        input: serde_json::Value,
        raw_delta: Option<String>,
    },
    ToolResult {
        tool_call_id: String,
        name: Option<String>,
        delta: String,
        is_error: bool,
    },
    UserConfirmRequest {
        request_id: String,
        prompt: String,
        metadata: serde_json::Value,
    },
    UserConfirmResult {
        request_id: String,
        approved: bool,
        reason: Option<String>,
    },
    ExternalExecutionRequest {
        request_id: String,
        tool_call_id: Option<String>,
        metadata: serde_json::Value,
    },
    ExternalExecutionResult {
        request_id: String,
        success: bool,
        output: Option<String>,
        reason: Option<String>,
    },
}

/// Provider-neutral event envelope for one agent execution fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub reply_id: String,
    pub sequence: u64,
    pub event_type: AgentEventType,
    pub payload: AgentEventPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl AgentEvent {
    /// Build an event envelope with a generated event id and timestamp.
    ///
    /// The caller owns `reply_id` and `sequence` so replay systems can preserve
    /// deterministic ordering across process restarts and transport reconnects.
    pub fn new(
        reply_id: impl Into<String>,
        sequence: u64,
        event_type: AgentEventType,
        payload: AgentEventPayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            reply_id: reply_id.into(),
            sequence,
            event_type,
            payload,
            block_id: None,
            tool_call_id: None,
            trace_id: None,
        }
    }

    /// Attach a stable block id for block start, delta, and end events.
    pub fn with_block_id(mut self, block_id: impl Into<String>) -> Self {
        self.block_id = Some(block_id.into());
        self
    }

    /// Attach a stable tool-call id for tool call/result correlation.
    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    /// Attach a trace id without exposing provider-private trace payloads.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

/// Structured projection errors raised by `AgentEventAccumulator`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AgentEventAccumulatorError {
    #[error("event reply id mismatch: expected {expected}, got {actual}")]
    ReplyIdMismatch { expected: String, actual: String },
    #[error("event sequence regression: last {last}, got {actual}")]
    SequenceRegression { last: u64, actual: u64 },
    #[error("event block id is missing for {event_type:?}")]
    MissingBlockId { event_type: AgentEventType },
    #[error("event block not found: {block_id}")]
    MissingBlock { block_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentEventAccumulator {
    reply_id: String,
    agent_name: String,
    trace_id: Option<String>,
    usage: Option<serde_json::Value>,
    generate_reason: Option<String>,
    blocks: Vec<BlockDraft>,
    accepted_event_count: usize,
    last_sequence: Option<u64>,
}

impl AgentEventAccumulator {
    /// Create a projection for a single assistant reply.
    pub fn new(reply_id: impl Into<String>, agent_name: impl Into<String>) -> Self {
        Self {
            reply_id: reply_id.into(),
            agent_name: agent_name.into(),
            trace_id: None,
            usage: None,
            generate_reason: None,
            blocks: Vec::new(),
            accepted_event_count: 0,
            last_sequence: None,
        }
    }

    /// Number of accepted events. This is useful for audit summaries.
    pub fn accepted_event_count(&self) -> usize {
        self.accepted_event_count
    }

    /// Consume one event and update the message projection.
    pub fn accept(&mut self, event: AgentEvent) -> Result<(), AgentEventAccumulatorError> {
        if event.reply_id != self.reply_id {
            warn!(
                expected_reply_id = %self.reply_id,
                actual_reply_id = %event.reply_id,
                event_type = ?event.event_type,
                "agent event rejected because reply id does not match accumulator"
            );
            return Err(AgentEventAccumulatorError::ReplyIdMismatch {
                expected: self.reply_id.clone(),
                actual: event.reply_id,
            });
        }
        if let Some(last) = self.last_sequence {
            if event.sequence <= last {
                warn!(
                    reply_id = %self.reply_id,
                    last_sequence = last,
                    actual_sequence = event.sequence,
                    "agent event rejected because sequence regressed"
                );
                return Err(AgentEventAccumulatorError::SequenceRegression {
                    last,
                    actual: event.sequence,
                });
            }
        }

        if self.trace_id.is_none() {
            self.trace_id = event.trace_id.clone();
        }
        self.last_sequence = Some(event.sequence);

        debug!(
            reply_id = %self.reply_id,
            sequence = event.sequence,
            event_type = ?event.event_type,
            "agent event accepted by accumulator"
        );

        match event.event_type {
            AgentEventType::ModelCallEnd => {
                if let AgentEventPayload::ModelCall { usage, reason, .. } = event.payload {
                    self.usage = usage;
                    self.generate_reason = reason;
                }
            }
            AgentEventType::TextBlockStart => {
                let block_id = required_block_id(&event)?;
                self.blocks.push(BlockDraft::Text {
                    block_id,
                    text: String::new(),
                });
            }
            AgentEventType::TextBlockDelta => {
                let block_id = required_block_id(&event)?;
                let delta = match event.payload {
                    AgentEventPayload::TextDelta { delta } => delta,
                    _ => String::new(),
                };
                match self.find_block_mut(&block_id)? {
                    BlockDraft::Text { text, .. } => text.push_str(&delta),
                    _ => warn!(block_id = %block_id, "text delta targeted a non-text block"),
                }
            }
            AgentEventType::ThinkingBlockStart => {
                let block_id = required_block_id(&event)?;
                self.blocks.push(BlockDraft::Thinking {
                    block_id,
                    thinking: String::new(),
                });
            }
            AgentEventType::ThinkingBlockDelta => {
                let block_id = required_block_id(&event)?;
                let delta = match event.payload {
                    AgentEventPayload::ThinkingDelta { delta } => delta,
                    _ => String::new(),
                };
                match self.find_block_mut(&block_id)? {
                    BlockDraft::Thinking { thinking, .. } => thinking.push_str(&delta),
                    _ => {
                        warn!(block_id = %block_id, "thinking delta targeted a non-thinking block")
                    }
                }
            }
            AgentEventType::ToolCallStart => {
                let block_id = required_block_id(&event)?;
                if let AgentEventPayload::ToolCall {
                    tool_call_id,
                    name,
                    input,
                    raw_delta,
                } = event.payload
                {
                    self.blocks.push(BlockDraft::ToolUse {
                        block_id,
                        tool_call_id,
                        name,
                        input,
                        raw_input: raw_delta,
                    });
                }
            }
            AgentEventType::ToolCallDelta => {
                let block_id = required_block_id(&event)?;
                if let AgentEventPayload::ToolCall {
                    input, raw_delta, ..
                } = event.payload
                {
                    if let BlockDraft::ToolUse {
                        input: stored_input,
                        raw_input,
                        ..
                    } = self.find_block_mut(&block_id)?
                    {
                        *stored_input = input;
                        if let Some(delta) = raw_delta {
                            raw_input.get_or_insert_with(String::new).push_str(&delta);
                        }
                    }
                }
            }
            AgentEventType::ToolResultStart => {
                let block_id = required_block_id(&event)?;
                if let AgentEventPayload::ToolResult {
                    tool_call_id,
                    name,
                    delta,
                    is_error,
                } = event.payload
                {
                    self.blocks.push(BlockDraft::ToolResult {
                        block_id,
                        tool_call_id,
                        name,
                        output: delta,
                        is_error,
                    });
                }
            }
            AgentEventType::ToolResultTextDelta | AgentEventType::ToolResultDataDelta => {
                let block_id = required_block_id(&event)?;
                if let AgentEventPayload::ToolResult {
                    delta, is_error, ..
                } = event.payload
                {
                    if let BlockDraft::ToolResult {
                        output,
                        is_error: stored_error,
                        ..
                    } = self.find_block_mut(&block_id)?
                    {
                        output.push_str(&delta);
                        *stored_error = *stored_error || is_error;
                    }
                }
            }
            AgentEventType::DataBlockStart => {
                let block_id = required_block_id(&event)?;
                let data = match event.payload {
                    AgentEventPayload::DataDelta { data } => data,
                    _ => serde_json::Value::Null,
                };
                self.blocks.push(BlockDraft::Data { block_id, data });
            }
            AgentEventType::DataBlockDelta => {
                let block_id = required_block_id(&event)?;
                if let AgentEventPayload::DataDelta { data } = event.payload {
                    match self.find_block_mut(&block_id)? {
                        BlockDraft::Data {
                            data: stored_data, ..
                        } => *stored_data = data,
                        _ => warn!(block_id = %block_id, "data delta targeted a non-data block"),
                    }
                }
            }
            _ => {}
        }

        self.accepted_event_count += 1;
        Ok(())
    }

    /// Build the final assistant message from accepted content blocks.
    pub fn finish(self) -> Msg {
        let blocks = self
            .blocks
            .into_iter()
            .map(BlockDraft::into_content_block)
            .collect::<Vec<_>>();
        let mut message = Msg::assistant(self.agent_name, MsgContent::Blocks(blocks));
        message.id = self.reply_id;
        message.invocation_id = self.trace_id;
        if self.usage.is_some() || self.generate_reason.is_some() {
            message.metadata = serde_json::json!({
                "usage": self.usage,
                "generate_reason": self.generate_reason,
            });
        }
        message
    }

    fn find_block_mut(
        &mut self,
        block_id: &str,
    ) -> Result<&mut BlockDraft, AgentEventAccumulatorError> {
        self.blocks
            .iter_mut()
            .find(|block| block.block_id() == block_id)
            .ok_or_else(|| {
                warn!(block_id = %block_id, "agent event referenced a missing block");
                AgentEventAccumulatorError::MissingBlock {
                    block_id: block_id.to_string(),
                }
            })
    }
}

fn required_block_id(event: &AgentEvent) -> Result<String, AgentEventAccumulatorError> {
    event
        .block_id
        .clone()
        .ok_or(AgentEventAccumulatorError::MissingBlockId {
            event_type: event.event_type,
        })
}

#[cfg(test)]
#[path = "event_contract_tests.rs"]
mod event_contract_tests;
