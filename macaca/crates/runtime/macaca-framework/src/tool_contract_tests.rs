// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use async_trait::async_trait;

use super::*;
use crate::message::TextBlock;
use crate::tool::{ToolError, ToolResponse};

struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::json(args))
    }

    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo input"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }
}

fn invocation(tool_name: &str) -> ToolInvocation {
    ToolInvocation::new(
        "call-1",
        tool_name,
        serde_json::json!({"value": 1}),
        ToolRuntimeContext::new(RuntimeContext::new("trace-tool-contract"))
            .with_injected("session_ref", serde_json::json!("session")),
    )
}

#[tokio::test]
async fn tool_handler_projects_to_tool_base_result() {
    let adapter = ToolHandlerAdapter::new(Arc::new(EchoTool), "basic");
    let result = adapter.call(invocation("echo")).await.unwrap();

    assert_eq!(adapter.spec().kind, ToolKind::Base);
    assert_eq!(adapter.spec().name, "echo");
    assert!(matches!(result, ToolExecutionResult::Completed { .. }));
}

#[tokio::test]
async fn schema_only_tool_fails_closed() {
    let tool = SchemaOnlyTool::new(ToolSpec::schema_only(
        "catalog_only",
        "Catalog only",
        serde_json::json!({"type": "object"}),
    ));
    let result = tool.call(invocation("catalog_only")).await.unwrap();

    assert_eq!(tool.spec().kind, ToolKind::SchemaOnly);
    assert!(matches!(result, ToolExecutionResult::Unavailable { .. }));
}

#[test]
fn streaming_tool_response_projects_to_events() {
    let result = ToolExecutionResult::from_tool_response(ToolResponse {
        content: vec![ContentBlock::Text(TextBlock {
            text: "chunk".into(),
        })],
        metadata: None,
        is_stream: true,
        is_last: true,
        is_interrupted: false,
    });

    assert!(matches!(
        result,
        ToolExecutionResult::Streaming { ref events, .. } if events.len() == 3
    ));
}

#[test]
fn tool_spec_supports_meta_tool_and_suspend_capabilities() {
    let spec = ToolSpec::base(
        "delegate",
        "Delegate to another tool",
        serde_json::json!({}),
    )
    .with_kind(ToolKind::Meta)
    .with_capability(ToolCapability::ContextInjection)
    .with_capability(ToolCapability::PermissionSuspend)
    .with_capability(ToolCapability::ExternalExecutionSuspend);

    assert_eq!(spec.kind, ToolKind::Meta);
    assert!(spec
        .capabilities
        .contains(&ToolCapability::PermissionSuspend));
    assert!(spec
        .capabilities
        .contains(&ToolCapability::ExternalExecutionSuspend));
}

#[test]
fn tool_runtime_context_carries_trace_and_injected_values() {
    let invocation = invocation("echo");

    assert_eq!(invocation.context.runtime.trace_id, "trace-tool-contract");
    assert_eq!(
        invocation.context.injected["session_ref"],
        serde_json::json!("session")
    );
}

struct DenyGuard;

#[async_trait]
impl ToolExecutionGuard for DenyGuard {
    fn name(&self) -> &str {
        "deny_guard"
    }

    async fn check(&self, _invocation: &ToolInvocation) -> AgentResult<ToolGuardDecision> {
        Ok(ToolGuardDecision::Deny {
            reason: "policy denied".into(),
        })
    }
}

#[tokio::test]
async fn guarded_executor_denies_before_tool_side_effect() {
    let adapter = ToolHandlerAdapter::new(Arc::new(EchoTool), "basic");
    let executor = GuardedToolExecutor::new().with_guard(Arc::new(DenyGuard));
    let result = executor
        .execute(&adapter, invocation("echo"))
        .await
        .unwrap();

    assert!(matches!(result, ToolExecutionResult::Denied { .. }));
}

#[tokio::test]
async fn guarded_executor_requires_trace() {
    let adapter = ToolHandlerAdapter::new(Arc::new(EchoTool), "basic");
    let invocation = ToolInvocation::new(
        "call-1",
        "echo",
        serde_json::json!({}),
        ToolRuntimeContext::new(RuntimeContext::new("")),
    );
    let result = GuardedToolExecutor::new()
        .execute(&adapter, invocation)
        .await
        .unwrap();

    assert!(matches!(result, ToolExecutionResult::Denied { .. }));
}
