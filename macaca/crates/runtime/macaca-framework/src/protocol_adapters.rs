// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Protocol adapters from typed `AgentEvent` streams.
//!
//! The framework keeps protocol conversion provider-neutral. A2A, AG-UI, and
//! Chat Completions streaming adapters project the same typed event evidence
//! without owning transport servers, UI shells, telemetry backends, schedulers,
//! RAG providers, or extension-specific services.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::event_contract::{AgentEvent, AgentEventPayload, AgentEventType};

/// Supported projection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolAdapterKind {
    A2a,
    AgUi,
    ChatCompletions,
}

/// Generic projected event. Protocol servers can wrap this DTO into their
/// concrete wire transport while preserving trace id, sequence, and source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolStreamFrame {
    pub adapter: ProtocolAdapterKind,
    pub event: String,
    pub sequence: u64,
    pub trace_id: Option<String>,
    pub payload: serde_json::Value,
}

/// Optional AgentScope extension descriptor. Concrete adapters remain service
/// or plugin backed and can be absent without affecting base framework calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalProtocolExtensionDescriptor {
    pub extension_id: String,
    pub service_backed: bool,
    pub available: bool,
    pub reason: Option<String>,
}

pub fn project_a2a_event(event: &AgentEvent) -> ProtocolStreamFrame {
    project_event(ProtocolAdapterKind::A2a, event, "a2a.agent_event")
}

pub fn project_ag_ui_event(event: &AgentEvent) -> ProtocolStreamFrame {
    project_event(ProtocolAdapterKind::AgUi, event, "ag_ui.agent_event")
}

pub fn project_chat_completion_event(event: &AgentEvent) -> ProtocolStreamFrame {
    let event_name = match event.event_type {
        AgentEventType::TextBlockDelta => "chat.completion.chunk",
        AgentEventType::ToolCallStart | AgentEventType::ToolCallEnd => "chat.tool_call.chunk",
        AgentEventType::ToolResultTextDelta => "chat.tool_result.chunk",
        AgentEventType::AgentEnd => "chat.completion.done",
        _ => "chat.completion.event",
    };
    project_event(ProtocolAdapterKind::ChatCompletions, event, event_name)
}

pub fn optional_extension_descriptors() -> Vec<OptionalProtocolExtensionDescriptor> {
    [
        "agent_protocol",
        "studio_telemetry",
        "scheduler",
        "rocketmq",
        "nacos",
        "higress",
        "training",
        "rag",
        "memory_repository",
        "session_repository",
        "skill_repository",
    ]
    .into_iter()
    .map(|extension_id| OptionalProtocolExtensionDescriptor {
        extension_id: extension_id.to_string(),
        service_backed: true,
        available: false,
        reason: Some("optional extension must be provided by a service or plugin".to_string()),
    })
    .collect()
}

fn project_event(
    adapter: ProtocolAdapterKind,
    event: &AgentEvent,
    event_name: &str,
) -> ProtocolStreamFrame {
    debug!(
        adapter = ?adapter,
        event_type = ?event.event_type,
        sequence = event.sequence,
        trace_id = ?event.trace_id,
        "projecting typed agent event to protocol frame"
    );
    ProtocolStreamFrame {
        adapter,
        event: event_name.to_string(),
        sequence: event.sequence,
        trace_id: event.trace_id.clone(),
        payload: payload_for_event(event),
    }
}

fn payload_for_event(event: &AgentEvent) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "event_type": format!("{:?}", event.event_type),
        "reply_id": event.reply_id,
        "block_id": event.block_id,
        "payload": event.payload,
    });
    if let AgentEventPayload::TextDelta { delta } = &event.payload {
        payload["delta"] = serde_json::json!(delta);
    }
    payload
}

#[cfg(test)]
#[path = "protocol_adapters_tests.rs"]
mod protocol_adapters_tests;
