// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::event_contract::{AgentEvent, AgentEventPayload, AgentEventType};

fn text_delta_event() -> AgentEvent {
    AgentEvent::new(
        "reply-1",
        7,
        AgentEventType::TextBlockDelta,
        AgentEventPayload::TextDelta {
            delta: "hello".to_string(),
        },
    )
    .with_trace_id("trace-protocol")
    .with_block_id("block-1")
}

#[test]
fn a2a_projection_preserves_event_sequence_trace_and_payload() {
    let frame = project_a2a_event(&text_delta_event());

    assert_eq!(frame.adapter, ProtocolAdapterKind::A2a);
    assert_eq!(frame.event, "a2a.agent_event");
    assert_eq!(frame.sequence, 7);
    assert_eq!(frame.trace_id.as_deref(), Some("trace-protocol"));
    assert_eq!(frame.payload["delta"], "hello");
}

#[test]
fn ag_ui_projection_uses_same_typed_event_evidence() {
    let frame = project_ag_ui_event(&text_delta_event());

    assert_eq!(frame.adapter, ProtocolAdapterKind::AgUi);
    assert_eq!(frame.event, "ag_ui.agent_event");
    assert_eq!(frame.payload["block_id"], "block-1");
}

#[test]
fn chat_completion_projection_maps_text_delta_to_chunk() {
    let frame = project_chat_completion_event(&text_delta_event());

    assert_eq!(frame.adapter, ProtocolAdapterKind::ChatCompletions);
    assert_eq!(frame.event, "chat.completion.chunk");
    assert_eq!(frame.payload["payload"]["delta"], "hello");
}

#[test]
fn optional_extensions_are_service_backed_and_unavailable_by_default() {
    let descriptors = optional_extension_descriptors();

    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor.extension_id == "agent_protocol"));
    assert!(descriptors
        .iter()
        .all(|descriptor| descriptor.service_backed));
    assert!(descriptors.iter().all(|descriptor| !descriptor.available));
}
