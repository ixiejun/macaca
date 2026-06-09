// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It defines a generic
// permission strategy boundary and does not encode application-specific tool,
// workflow, provider, or business rules.

//! Permission, HITL, and external-execution strategy contracts.
//!
//! The framework must ask policy before tool side effects. This module keeps
//! the decision provider-neutral:
//!
//! - **Strategy**: callers can provide built-in, plugin, remote, mock, or
//!   unavailable permission policies.
//! - **Command**: the policy receives a bounded command object.
//! - **State**: ask/external decisions become durable `PermissionState` values.
//! - **Null Object**: `AllowAllToolPermissionPolicy` is explicit permissive
//!   behavior for focused tests and optional service absence.

use async_trait::async_trait;
use tracing::debug;

use crate::agent::AgentResult;
use crate::message::ToolUseBlock;
use crate::runtime_context::{AgentState, PermissionState, RuntimeContext};
use crate::tool_contract::ToolExecutionResult;

/// Command object evaluated before a tool side effect.
pub struct ToolPermissionCommand<'a> {
    pub runtime: &'a RuntimeContext,
    pub state: &'a AgentState,
    pub tool_call: &'a ToolUseBlock,
}

/// Permission decision for one tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPermissionDecision {
    Allow,
    AskUser { request_id: String, prompt: String },
    RequireExternalExecution { request_id: String },
    Deny { reason: String },
}

/// Provider-neutral policy trait for tool side effects.
#[async_trait]
pub trait ToolPermissionPolicy: Send + Sync {
    /// Stable policy name used in trace logs and provider snapshots.
    fn name(&self) -> &str;

    /// Decide whether a tool call may execute now, must suspend, or is denied.
    async fn decide(
        &self,
        command: ToolPermissionCommand<'_>,
    ) -> AgentResult<ToolPermissionDecision>;
}

/// Explicit permissive policy for focused tests.
pub struct AllowAllToolPermissionPolicy;

#[async_trait]
impl ToolPermissionPolicy for AllowAllToolPermissionPolicy {
    fn name(&self) -> &str {
        "allow_all"
    }

    async fn decide(
        &self,
        command: ToolPermissionCommand<'_>,
    ) -> AgentResult<ToolPermissionDecision> {
        debug!(
            policy = self.name(),
            trace_id = %command.runtime.trace_id,
            tool_name = %command.tool_call.name,
            "tool permission policy allowed tool execution"
        );
        let _ = command.state;
        Ok(ToolPermissionDecision::Allow)
    }
}

/// Result of applying a permission decision or resume event.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionGateOutcome {
    /// Tool execution may continue immediately.
    Allow,
    /// Tool execution must pause and persist the returned permission state.
    Suspend {
        permission: PermissionState,
        result: ToolExecutionResult,
    },
    /// Tool execution is denied and must not run side effects.
    Deny {
        permission: PermissionState,
        result: ToolExecutionResult,
    },
}

/// Provider-neutral user confirmation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserConfirmResultCommand {
    pub request_id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

/// Provider-neutral external execution result.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalExecutionResultCommand {
    pub request_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub reason: Option<String>,
}

/// Stateless permission bridge used by ReActAgent, future tool services, and
/// protocol adapters. It translates policy decisions and HITL/external resume
/// events into durable `PermissionState` values without owning application
/// policy, UI behavior, or tool side effects.
pub struct ToolPermissionEngineBridge;

impl ToolPermissionEngineBridge {
    pub fn apply_decision(
        decision: ToolPermissionDecision,
        tool_call: &ToolUseBlock,
    ) -> PermissionGateOutcome {
        match decision {
            ToolPermissionDecision::Allow => PermissionGateOutcome::Allow,
            ToolPermissionDecision::AskUser { request_id, prompt } => {
                let permission = PermissionState::WaitingForUserConfirm {
                    request_id: request_id.clone(),
                    tool_call_id: Some(tool_call.id.clone()),
                    prompt,
                };
                PermissionGateOutcome::Suspend {
                    permission,
                    result: ToolExecutionResult::Suspended {
                        request_id,
                        reason: "user confirmation required".to_string(),
                        external: false,
                    },
                }
            }
            ToolPermissionDecision::RequireExternalExecution { request_id } => {
                let permission = PermissionState::WaitingForExternalExecution {
                    request_id: request_id.clone(),
                    tool_call_id: Some(tool_call.id.clone()),
                };
                PermissionGateOutcome::Suspend {
                    permission,
                    result: ToolExecutionResult::Suspended {
                        request_id,
                        reason: "external execution required".to_string(),
                        external: true,
                    },
                }
            }
            ToolPermissionDecision::Deny { reason } => {
                let permission = PermissionState::Denied {
                    reason: reason.clone(),
                };
                PermissionGateOutcome::Deny {
                    permission,
                    result: ToolExecutionResult::Denied { reason },
                }
            }
        }
    }

    pub fn apply_user_confirm_result(
        current: &PermissionState,
        command: UserConfirmResultCommand,
    ) -> PermissionGateOutcome {
        match current {
            PermissionState::WaitingForUserConfirm { request_id, .. }
                if request_id == &command.request_id && command.approved =>
            {
                PermissionGateOutcome::Allow
            }
            PermissionState::WaitingForUserConfirm { request_id, .. }
                if request_id == &command.request_id =>
            {
                let reason = command
                    .reason
                    .unwrap_or_else(|| "user confirmation denied".to_string());
                PermissionGateOutcome::Deny {
                    permission: PermissionState::Denied {
                        reason: reason.clone(),
                    },
                    result: ToolExecutionResult::Denied { reason },
                }
            }
            _ => PermissionGateOutcome::Suspend {
                permission: current.clone(),
                result: ToolExecutionResult::Suspended {
                    request_id: command.request_id,
                    reason: "permission result does not match pending request".to_string(),
                    external: false,
                },
            },
        }
    }

    pub fn apply_external_execution_result(
        current: &PermissionState,
        command: ExternalExecutionResultCommand,
    ) -> PermissionGateOutcome {
        match current {
            PermissionState::WaitingForExternalExecution { request_id, .. }
                if request_id == &command.request_id && command.success =>
            {
                PermissionGateOutcome::Allow
            }
            PermissionState::WaitingForExternalExecution { request_id, .. }
                if request_id == &command.request_id =>
            {
                let reason = command
                    .reason
                    .or(command.output)
                    .unwrap_or_else(|| "external execution failed".to_string());
                PermissionGateOutcome::Deny {
                    permission: PermissionState::Denied {
                        reason: reason.clone(),
                    },
                    result: ToolExecutionResult::Denied { reason },
                }
            }
            _ => PermissionGateOutcome::Suspend {
                permission: current.clone(),
                result: ToolExecutionResult::Suspended {
                    request_id: command.request_id,
                    reason: "external execution result does not match pending request".to_string(),
                    external: true,
                },
            },
        }
    }
}

#[cfg(test)]
#[path = "permission_contract_tests.rs"]
mod permission_contract_tests;
