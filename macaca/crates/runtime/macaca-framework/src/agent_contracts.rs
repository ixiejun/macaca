// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Event-stream-first agent contracts.
//!
//! This module closes the AgentScope 2.0 parity gap where `reply` must be a
//! projection over canonical events rather than a separate execution path. The
//! contracts use small traits so implementations can choose a local, remote,
//! plugin, mock, or unavailable provider without changing consumer code.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::agent::{AgentError, AgentResult};
use crate::event_contract::{AgentEvent, AgentEventAccumulator};
use crate::message::Msg;
use crate::runtime_context::{PermissionState, RuntimeContext};

/// Streaming behavior requested by one agent call.
///
/// The fields are data-only Strategy inputs. Concrete model clients decide how
/// to translate them into provider-specific transport flags outside framework
/// ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamOptions {
    pub emit_lifecycle_events: bool,
    pub emit_model_events: bool,
    pub emit_tool_events: bool,
    pub emit_partial_text: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            emit_lifecycle_events: true,
            emit_model_events: true,
            emit_tool_events: true,
            emit_partial_text: true,
        }
    }
}

/// Provider-neutral structured output request.
///
/// The schema is deliberately generic JSON. Framework code validates that a
/// request exists and records failure policy; formatter and model services own
/// provider-specific schema encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredOutputRequest {
    pub schema: serde_json::Value,
    pub strict: bool,
    pub failure_policy: StructuredOutputFailurePolicy,
}

/// Failure behavior when a model response cannot satisfy the requested schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredOutputFailurePolicy {
    ReturnError,
    EmitValidationEvent,
    RetryThroughServicePolicy,
}

/// Canonical event-stream command for one framework agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamCommand {
    pub runtime: RuntimeContext,
    pub input: Vec<Msg>,
    pub options: StreamOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<StructuredOutputRequest>,
}

impl AgentStreamCommand {
    /// Create a command with default stream behavior.
    pub fn new(runtime: RuntimeContext, input: Vec<Msg>) -> Self {
        Self {
            runtime,
            input,
            options: StreamOptions::default(),
            structured_output: None,
        }
    }
}

/// Canonical stream result plus deterministic final-message projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamResult {
    pub events: Vec<AgentEvent>,
    pub final_message: Option<Msg>,
}

/// Trait for providers that can execute and return a canonical event stream.
#[async_trait]
pub trait StreamableAgent: Send + Sync {
    async fn stream_events(&self, command: AgentStreamCommand) -> AgentResult<AgentStreamResult>;
}

/// Trait for providers that can observe messages without owning a reply path.
#[async_trait]
pub trait ObservableAgent: Send + Sync {
    async fn observe_events(
        &self,
        runtime: RuntimeContext,
        input: Vec<Msg>,
    ) -> AgentResult<Vec<AgentEvent>>;
}

/// Compatibility call trait. Implementors SHOULD use `reply_from_stream` so
/// final replies never drift from event replay behavior.
#[async_trait]
pub trait CallableAgent: StreamableAgent {
    async fn reply(&self, command: AgentStreamCommand) -> AgentResult<Option<Msg>> {
        reply_from_stream(self, command).await
    }
}

#[async_trait]
impl<T> CallableAgent for T where T: StreamableAgent + Send + Sync {}

/// Project a final reply by replaying canonical events.
///
/// This helper is the Facade that keeps older `reply` callers aligned while
/// preserving one execution truth. If a provider already supplied a projection,
/// the replay still acts as a contract check when events are available.
pub async fn reply_from_stream<A>(
    agent: &A,
    command: AgentStreamCommand,
) -> AgentResult<Option<Msg>>
where
    A: StreamableAgent + ?Sized,
{
    debug!(
        trace_id = %command.runtime.trace_id,
        input_count = command.input.len(),
        "projecting agent reply from canonical event stream"
    );
    let result = agent.stream_events(command).await?;
    if let Some(final_message) = result.final_message {
        return Ok(Some(final_message));
    }
    let Some(first_event) = result.events.first() else {
        return Ok(None);
    };
    let mut accumulator = AgentEventAccumulator::new(&first_event.reply_id, "agent");
    for event in result.events {
        accumulator
            .accept(event)
            .map_err(|error| AgentError::Other(format!("event projection failed: {error}")))?;
    }
    Ok(Some(accumulator.finish()))
}

/// Human-in-the-loop input kind requested by an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitlInputKind {
    UserMessage,
    UserConfirmation,
    ExternalExecutionResult,
}

/// Durable pending input state used by UserAgent and StreamUserInput adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingInputState {
    pub request_id: String,
    pub expected_kind: HitlInputKind,
    pub trace_id: String,
    pub permission: PermissionState,
    pub idempotency_key: String,
}

/// User-provided resume input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitlResumeInput {
    pub request_id: String,
    pub kind: HitlInputKind,
    pub idempotency_key: String,
    pub payload: serde_json::Value,
}

/// Result of applying a HITL resume input to pending state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HitlResumeDecision {
    Accepted { payload: serde_json::Value },
    Rejected { reason: String },
}

/// Validate resume correlation and idempotency before any suspended side effect
/// is allowed to continue. This is a State-pattern guard shared by user-input
/// agents, tool confirmation, and external execution resumes.
pub fn validate_hitl_resume(
    pending: &PendingInputState,
    input: HitlResumeInput,
) -> HitlResumeDecision {
    if pending.request_id != input.request_id {
        warn!(
            expected_request_id = %pending.request_id,
            actual_request_id = %input.request_id,
            trace_id = %pending.trace_id,
            "hitl resume rejected because request id does not match"
        );
        return HitlResumeDecision::Rejected {
            reason: "resume request id does not match pending input".to_string(),
        };
    }
    if pending.expected_kind != input.kind {
        warn!(
            request_id = %pending.request_id,
            trace_id = %pending.trace_id,
            "hitl resume rejected because input kind does not match"
        );
        return HitlResumeDecision::Rejected {
            reason: "resume input kind does not match pending input".to_string(),
        };
    }
    if pending.idempotency_key != input.idempotency_key {
        warn!(
            request_id = %pending.request_id,
            trace_id = %pending.trace_id,
            "hitl resume rejected because idempotency key does not match"
        );
        return HitlResumeDecision::Rejected {
            reason: "resume idempotency key does not match pending input".to_string(),
        };
    }
    HitlResumeDecision::Accepted {
        payload: input.payload,
    }
}

#[cfg(test)]
#[path = "agent_contracts_tests.rs"]
mod agent_contracts_tests;
