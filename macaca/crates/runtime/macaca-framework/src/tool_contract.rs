// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! AgentScope 2.0-style provider-neutral tool contracts.
//!
//! Existing `Toolkit` remains available as a concrete registry, while this
//! module defines the future-stable tool ABI used by framework providers. The design
//! uses Command, Adapter, Strategy, State, and Observer patterns:
//!
//! - `ToolInvocation` is the command that carries trace-required runtime scope.
//! - `ToolSpec` describes base, meta, and schema-only tools without executing
//!   provider-specific code.
//! - `ToolExecutionResult` models completion, streaming, suspension, denial,
//!   and unavailability explicitly.
//! - `ToolHandlerAdapter` bridges current `ToolHandler` implementations
//!   without changing their public trait.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::agent::{AgentError, AgentResult};
use crate::message::ContentBlock;
use crate::runtime_context::RuntimeContext;
use crate::tool::{ToolHandler, ToolResponse};

/// Tool category used by model-visible descriptors and routing policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Base,
    Meta,
    SchemaOnly,
}

/// Tool capability flags understood by policy and runtime adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapability {
    Streaming,
    ContextInjection,
    PermissionSuspend,
    ExternalExecutionSuspend,
}

/// Provider-neutral group descriptor. Groups are data; activation and policy
/// decisions remain owned by runtime/service layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGroupSpec {
    pub name: String,
    pub active_by_default: bool,
}

impl ToolGroupSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            active_by_default: true,
        }
    }
}

/// Model-visible and policy-visible tool descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub kind: ToolKind,
    pub group: ToolGroupSpec,
    #[serde(default)]
    pub capabilities: Vec<ToolCapability>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl ToolSpec {
    pub fn base(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            kind: ToolKind::Base,
            group: ToolGroupSpec::new("basic"),
            capabilities: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn schema_only(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            kind: ToolKind::SchemaOnly,
            ..Self::base(name, description, input_schema)
        }
    }

    pub fn from_handler(handler: &dyn ToolHandler, group: impl Into<String>) -> Self {
        let mut spec = Self::base(handler.name(), handler.description(), handler.schema());
        spec.group = ToolGroupSpec::new(group);
        spec
    }

    pub fn with_kind(mut self, kind: ToolKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_capability(mut self, capability: ToolCapability) -> Self {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
        self
    }
}

/// Runtime context injected into a tool call. This is deliberately scoped and
/// sanitized; raw prompts, provider payloads, credentials, and file contents
/// must not be copied into the context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRuntimeContext {
    pub runtime: RuntimeContext,
    #[serde(default)]
    pub injected: BTreeMap<String, serde_json::Value>,
}

impl ToolRuntimeContext {
    pub fn new(runtime: RuntimeContext) -> Self {
        Self {
            runtime,
            injected: BTreeMap::new(),
        }
    }

    pub fn with_injected(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.injected.insert(key.into(), value);
        self
    }
}

/// Command object for one tool execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub context: ToolRuntimeContext,
}

impl ToolInvocation {
    pub fn new(
        tool_call_id: impl Into<String>,
        tool_name: impl Into<String>,
        input: serde_json::Value,
        context: ToolRuntimeContext,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
            input,
            context,
        }
    }
}

/// Streaming event emitted by tool adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ToolExecutionEvent {
    Started,
    Delta { content: Vec<ContentBlock> },
    Completed,
    Suspended { request_id: String, reason: String },
}

/// Result of a tool execution or policy gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolExecutionResult {
    Completed {
        content: Vec<ContentBlock>,
        metadata: Option<serde_json::Value>,
        is_error: bool,
    },
    Streaming {
        events: Vec<ToolExecutionEvent>,
        metadata: Option<serde_json::Value>,
    },
    Suspended {
        request_id: String,
        reason: String,
        external: bool,
    },
    Denied {
        reason: String,
    },
    Unavailable {
        reason: String,
    },
}

impl ToolExecutionResult {
    pub fn from_tool_response(response: ToolResponse) -> Self {
        if response.is_stream {
            ToolExecutionResult::Streaming {
                events: vec![
                    ToolExecutionEvent::Started,
                    ToolExecutionEvent::Delta {
                        content: response.content,
                    },
                    ToolExecutionEvent::Completed,
                ],
                metadata: response.metadata,
            }
        } else {
            ToolExecutionResult::Completed {
                content: response.content,
                metadata: response.metadata,
                is_error: response.is_interrupted,
            }
        }
    }
}

/// Guard decision returned before a tool side effect is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolGuardDecision {
    Allow,
    Deny { reason: String },
}

/// Decorator hook for trace, policy, resource, entitlement, and metering gates.
///
/// Concrete service adapters can implement this trait for real OS policy and
/// resource services. The framework owns only the generic sequencing contract:
/// every guard runs before the tool side effect, and denial fails closed.
#[async_trait]
pub trait ToolExecutionGuard: Send + Sync {
    fn name(&self) -> &str;
    async fn check(&self, invocation: &ToolInvocation) -> AgentResult<ToolGuardDecision>;
}

/// Minimal guard that enforces trace-required side effects.
pub struct TraceRequiredToolGuard;

#[async_trait]
impl ToolExecutionGuard for TraceRequiredToolGuard {
    fn name(&self) -> &str {
        "trace_required"
    }

    async fn check(&self, invocation: &ToolInvocation) -> AgentResult<ToolGuardDecision> {
        if invocation.context.runtime.trace_id.trim().is_empty() {
            warn!(
                tool_name = %invocation.tool_name,
                tool_call_id = %invocation.tool_call_id,
                "tool execution denied because trace id is missing"
            );
            return Ok(ToolGuardDecision::Deny {
                reason: "tool execution requires trace_id".to_string(),
            });
        }
        Ok(ToolGuardDecision::Allow)
    }
}

/// Decorated executor that runs all guards before invoking a tool.
pub struct GuardedToolExecutor {
    guards: Vec<Arc<dyn ToolExecutionGuard>>,
}

impl GuardedToolExecutor {
    pub fn new() -> Self {
        Self {
            guards: vec![Arc::new(TraceRequiredToolGuard)],
        }
    }

    pub fn with_guard(mut self, guard: Arc<dyn ToolExecutionGuard>) -> Self {
        self.guards.push(guard);
        self
    }

    pub async fn execute(
        &self,
        tool: &dyn ToolBase,
        invocation: ToolInvocation,
    ) -> AgentResult<ToolExecutionResult> {
        for guard in &self.guards {
            debug!(
                trace_id = %invocation.context.runtime.trace_id,
                tool_name = %invocation.tool_name,
                guard = %guard.name(),
                "tool execution guard evaluating side effect"
            );
            match guard.check(&invocation).await? {
                ToolGuardDecision::Allow => {}
                ToolGuardDecision::Deny { reason } => {
                    warn!(
                        trace_id = %invocation.context.runtime.trace_id,
                        tool_name = %invocation.tool_name,
                        guard = %guard.name(),
                        reason = %reason,
                        "tool execution denied before side effect"
                    );
                    return Ok(ToolExecutionResult::Denied { reason });
                }
            }
        }
        tool.call(invocation).await
    }
}

impl Default for GuardedToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// AgentScope 2.0-equivalent tool trait.
#[async_trait]
pub trait ToolBase: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn call(&self, invocation: ToolInvocation) -> AgentResult<ToolExecutionResult>;
}

/// Schema-only tool descriptor. It can be exposed to model planning/catalogs
/// but fails closed when execution is attempted.
pub struct SchemaOnlyTool {
    spec: ToolSpec,
}

impl SchemaOnlyTool {
    pub fn new(spec: ToolSpec) -> Self {
        Self {
            spec: spec.with_kind(ToolKind::SchemaOnly),
        }
    }
}

#[async_trait]
impl ToolBase for SchemaOnlyTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn call(&self, invocation: ToolInvocation) -> AgentResult<ToolExecutionResult> {
        warn!(
            trace_id = %invocation.context.runtime.trace_id,
            tool_name = %self.spec.name,
            "schema-only tool execution denied"
        );
        Ok(ToolExecutionResult::Unavailable {
            reason: "schema-only tool cannot execute".to_string(),
        })
    }
}

/// Adapter that presents an existing `ToolHandler` as the new `ToolBase` contract.
pub struct ToolHandlerAdapter {
    spec: ToolSpec,
    handler: Arc<dyn ToolHandler>,
}

impl ToolHandlerAdapter {
    pub fn new(handler: Arc<dyn ToolHandler>, group: impl Into<String>) -> Self {
        let spec = ToolSpec::from_handler(handler.as_ref(), group);
        info!(tool_name = %spec.name, "tool handler adapted to ToolBase contract");
        Self { spec, handler }
    }
}

#[async_trait]
impl ToolBase for ToolHandlerAdapter {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn call(&self, invocation: ToolInvocation) -> AgentResult<ToolExecutionResult> {
        debug!(
            trace_id = %invocation.context.runtime.trace_id,
            tool_call_id = %invocation.tool_call_id,
            tool_name = %invocation.tool_name,
            "tool handler adapter executing tool call"
        );
        let response = self
            .handler
            .execute(invocation.input)
            .await
            .map_err(|error| AgentError::Tool(error.to_string()))?;
        Ok(ToolExecutionResult::from_tool_response(response))
    }
}

#[cfg(test)]
#[path = "tool_contract_tests.rs"]
mod tool_contract_tests;
