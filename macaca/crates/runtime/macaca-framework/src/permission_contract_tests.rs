// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::message::ToolUseBlock;

fn tool_call() -> ToolUseBlock {
    ToolUseBlock {
        id: "tool-call-1".into(),
        name: "generic_tool".into(),
        input: serde_json::json!({}),
        raw_input: None,
    }
}

#[test]
fn bridge_maps_allow_to_immediate_execution() {
    let outcome =
        ToolPermissionEngineBridge::apply_decision(ToolPermissionDecision::Allow, &tool_call());

    assert!(matches!(outcome, PermissionGateOutcome::Allow));
}

#[test]
fn bridge_maps_ask_user_to_durable_suspend_state() {
    let outcome = ToolPermissionEngineBridge::apply_decision(
        ToolPermissionDecision::AskUser {
            request_id: "confirm-1".into(),
            prompt: "approve".into(),
        },
        &tool_call(),
    );

    assert!(matches!(
        outcome,
        PermissionGateOutcome::Suspend {
            permission: PermissionState::WaitingForUserConfirm { .. },
            result: ToolExecutionResult::Suspended {
                external: false,
                ..
            }
        }
    ));
}

#[test]
fn bridge_maps_external_to_durable_suspend_state() {
    let outcome = ToolPermissionEngineBridge::apply_decision(
        ToolPermissionDecision::RequireExternalExecution {
            request_id: "external-1".into(),
        },
        &tool_call(),
    );

    assert!(matches!(
        outcome,
        PermissionGateOutcome::Suspend {
            permission: PermissionState::WaitingForExternalExecution { .. },
            result: ToolExecutionResult::Suspended { external: true, .. }
        }
    ));
}

#[test]
fn bridge_maps_user_confirm_result_to_allow_or_deny() {
    let current = PermissionState::WaitingForUserConfirm {
        request_id: "confirm-1".into(),
        tool_call_id: Some("tool-call-1".into()),
        prompt: "approve".into(),
    };

    let approved = ToolPermissionEngineBridge::apply_user_confirm_result(
        &current,
        UserConfirmResultCommand {
            request_id: "confirm-1".into(),
            approved: true,
            reason: None,
        },
    );
    let denied = ToolPermissionEngineBridge::apply_user_confirm_result(
        &current,
        UserConfirmResultCommand {
            request_id: "confirm-1".into(),
            approved: false,
            reason: Some("no".into()),
        },
    );

    assert!(matches!(approved, PermissionGateOutcome::Allow));
    assert!(matches!(denied, PermissionGateOutcome::Deny { .. }));
}

#[test]
fn bridge_maps_external_result_to_allow_or_deny() {
    let current = PermissionState::WaitingForExternalExecution {
        request_id: "external-1".into(),
        tool_call_id: Some("tool-call-1".into()),
    };

    let success = ToolPermissionEngineBridge::apply_external_execution_result(
        &current,
        ExternalExecutionResultCommand {
            request_id: "external-1".into(),
            success: true,
            output: Some("ok".into()),
            reason: None,
        },
    );
    let failed = ToolPermissionEngineBridge::apply_external_execution_result(
        &current,
        ExternalExecutionResultCommand {
            request_id: "external-1".into(),
            success: false,
            output: None,
            reason: Some("failed".into()),
        },
    );

    assert!(matches!(success, PermissionGateOutcome::Allow));
    assert!(matches!(failed, PermissionGateOutcome::Deny { .. }));
}
