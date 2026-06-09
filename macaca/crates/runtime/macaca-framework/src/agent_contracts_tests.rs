// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::runtime_context::PermissionState;

#[test]
fn stream_options_default_emits_core_event_groups() {
    let options = StreamOptions::default();

    assert!(options.emit_lifecycle_events);
    assert!(options.emit_model_events);
    assert!(options.emit_tool_events);
    assert!(options.emit_partial_text);
}

#[test]
fn hitl_resume_accepts_matching_correlation() {
    let pending = PendingInputState {
        request_id: "request-1".to_string(),
        expected_kind: HitlInputKind::UserConfirmation,
        trace_id: "trace-hitl".to_string(),
        permission: PermissionState::Clear,
        idempotency_key: "idem-1".to_string(),
    };

    let decision = validate_hitl_resume(
        &pending,
        HitlResumeInput {
            request_id: "request-1".to_string(),
            kind: HitlInputKind::UserConfirmation,
            idempotency_key: "idem-1".to_string(),
            payload: serde_json::json!({"approved": true}),
        },
    );

    assert!(matches!(decision, HitlResumeDecision::Accepted { .. }));
}

#[test]
fn hitl_resume_rejects_stale_or_mismatched_input() {
    let pending = PendingInputState {
        request_id: "request-1".to_string(),
        expected_kind: HitlInputKind::ExternalExecutionResult,
        trace_id: "trace-hitl".to_string(),
        permission: PermissionState::Clear,
        idempotency_key: "idem-1".to_string(),
    };

    let decision = validate_hitl_resume(
        &pending,
        HitlResumeInput {
            request_id: "request-2".to_string(),
            kind: HitlInputKind::ExternalExecutionResult,
            idempotency_key: "idem-1".to_string(),
            payload: serde_json::json!({}),
        },
    );

    assert!(matches!(decision, HitlResumeDecision::Rejected { .. }));
}
