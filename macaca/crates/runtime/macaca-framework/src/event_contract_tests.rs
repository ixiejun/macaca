// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::message::{ContentBlock, MsgContent};

#[test]
fn text_events_reconstruct_final_assistant_message() {
    let mut accumulator = AgentEventAccumulator::new("reply-1", "assistant");

    accumulator
        .accept(
            AgentEvent::new(
                "reply-1",
                1,
                AgentEventType::TextBlockStart,
                AgentEventPayload::Empty,
            )
            .with_block_id("text-1")
            .with_trace_id("trace-1"),
        )
        .unwrap();
    accumulator
        .accept(
            AgentEvent::new(
                "reply-1",
                2,
                AgentEventType::TextBlockDelta,
                AgentEventPayload::TextDelta {
                    delta: "hello ".to_string(),
                },
            )
            .with_block_id("text-1"),
        )
        .unwrap();
    accumulator
        .accept(
            AgentEvent::new(
                "reply-1",
                3,
                AgentEventType::TextBlockDelta,
                AgentEventPayload::TextDelta {
                    delta: "world".to_string(),
                },
            )
            .with_block_id("text-1"),
        )
        .unwrap();

    let message = accumulator.finish();
    assert_eq!(message.id, "reply-1");
    assert_eq!(message.invocation_id.as_deref(), Some("trace-1"));
    assert_eq!(message.get_text(), "hello world");
}

#[test]
fn tool_events_reconstruct_tool_blocks() {
    let mut accumulator = AgentEventAccumulator::new("reply-2", "assistant");

    accumulator
        .accept(
            AgentEvent::new(
                "reply-2",
                1,
                AgentEventType::ToolCallStart,
                AgentEventPayload::ToolCall {
                    tool_call_id: "tool-call-1".to_string(),
                    name: "generic_tool".to_string(),
                    input: serde_json::json!({"query": "value"}),
                    raw_delta: Some("{\"query\":\"value\"}".to_string()),
                },
            )
            .with_block_id("tool-use-1")
            .with_tool_call_id("tool-call-1"),
        )
        .unwrap();
    accumulator
        .accept(
            AgentEvent::new(
                "reply-2",
                2,
                AgentEventType::ToolResultStart,
                AgentEventPayload::ToolResult {
                    tool_call_id: "tool-call-1".to_string(),
                    name: Some("generic_tool".to_string()),
                    delta: "ok".to_string(),
                    is_error: false,
                },
            )
            .with_block_id("tool-result-1")
            .with_tool_call_id("tool-call-1"),
        )
        .unwrap();
    accumulator
        .accept(
            AgentEvent::new(
                "reply-2",
                3,
                AgentEventType::ToolResultTextDelta,
                AgentEventPayload::ToolResult {
                    tool_call_id: "tool-call-1".to_string(),
                    name: Some("generic_tool".to_string()),
                    delta: " done".to_string(),
                    is_error: false,
                },
            )
            .with_block_id("tool-result-1")
            .with_tool_call_id("tool-call-1"),
        )
        .unwrap();

    let message = accumulator.finish();
    let MsgContent::Blocks(blocks) = message.content else {
        panic!("expected block content");
    };
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], ContentBlock::ToolUse(block) if block.id == "tool-call-1"));
    assert!(matches!(&blocks[1], ContentBlock::ToolResult(block) if block.output == "ok done"));
}

#[test]
fn data_and_model_end_events_preserve_metadata() {
    let mut accumulator = AgentEventAccumulator::new("reply-3", "assistant");

    accumulator
        .accept(
            AgentEvent::new(
                "reply-3",
                1,
                AgentEventType::DataBlockStart,
                AgentEventPayload::DataDelta {
                    data: serde_json::json!({"partial": true}),
                },
            )
            .with_block_id("data-1"),
        )
        .unwrap();
    accumulator
        .accept(
            AgentEvent::new(
                "reply-3",
                2,
                AgentEventType::DataBlockDelta,
                AgentEventPayload::DataDelta {
                    data: serde_json::json!({"answer": 42}),
                },
            )
            .with_block_id("data-1"),
        )
        .unwrap();
    accumulator
        .accept(AgentEvent::new(
            "reply-3",
            3,
            AgentEventType::ModelCallEnd,
            AgentEventPayload::ModelCall {
                model_id: Some("provider-neutral-model".to_string()),
                request_id: Some("request-1".to_string()),
                usage: Some(serde_json::json!({"input_tokens": 1, "output_tokens": 2})),
                reason: Some("end_turn".to_string()),
            },
        ))
        .unwrap();

    let message = accumulator.finish();
    assert_eq!(message.metadata["generate_reason"], "end_turn");
    assert_eq!(message.metadata["usage"]["output_tokens"], 2);
    let MsgContent::Blocks(blocks) = message.content else {
        panic!("expected block content");
    };
    assert!(matches!(&blocks[0], ContentBlock::Data(block) if block.data["answer"] == 42));
}

#[test]
fn mismatched_reply_id_is_structured_error() {
    let mut accumulator = AgentEventAccumulator::new("reply-expected", "assistant");
    let error = accumulator
        .accept(
            AgentEvent::new(
                "reply-other",
                1,
                AgentEventType::TextBlockStart,
                AgentEventPayload::Empty,
            )
            .with_block_id("text-1"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        AgentEventAccumulatorError::ReplyIdMismatch { .. }
    ));
    assert_eq!(accumulator.accepted_event_count(), 0);
}

#[test]
fn sequence_regression_is_structured_error() {
    let mut accumulator = AgentEventAccumulator::new("reply-sequence", "assistant");
    accumulator
        .accept(
            AgentEvent::new(
                "reply-sequence",
                2,
                AgentEventType::TextBlockStart,
                AgentEventPayload::Empty,
            )
            .with_block_id("text-1"),
        )
        .unwrap();

    let error = accumulator
        .accept(
            AgentEvent::new(
                "reply-sequence",
                2,
                AgentEventType::TextBlockDelta,
                AgentEventPayload::TextDelta {
                    delta: "duplicate".into(),
                },
            )
            .with_block_id("text-1"),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        AgentEventAccumulatorError::SequenceRegression { last: 2, actual: 2 }
    ));
    assert_eq!(accumulator.accepted_event_count(), 1);
}
