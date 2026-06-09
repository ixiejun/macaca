// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Toolkit bridge for `ReActAgent`.
//!
//! The bridge is intentionally narrow: it adapts one `Toolkit` call to
//! the AgentScope 2.0-style `ToolBase` contract so side effects can be decorated by
//! trace, policy, resource, entitlement, and service guards. Provider creation
//! remains outside the framework; runtime-host or service adapters can replace
//! the executor or add guards at composition roots.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use super::*;
use crate::message::{ContentBlock, TextBlock};
use crate::react_agent::react_agent_steps::tool_response_text;
use crate::tool_contract::{
    ToolBase, ToolExecutionEvent, ToolExecutionResult, ToolInvocation, ToolKind,
    ToolRuntimeContext, ToolSpec,
};

/// Adapter that exposes `Toolkit::call_tool` as one `ToolBase`.
///
/// The adapter lets the guarded executor own pre-side-effect checks while the
/// Toolkit remains a provider-neutral command registry. Tool lookup and
/// execution failures are returned as error content so the event stream can
/// preserve the full reasoning/action history instead of losing audit evidence
/// by aborting the entire run.
pub(super) struct ToolkitToolAdapter {
    toolkit: Arc<Mutex<Toolkit>>,
    spec: ToolSpec,
}

impl ToolkitToolAdapter {
    pub(super) fn new(toolkit: Arc<Mutex<Toolkit>>, tool_name: &str) -> Self {
        Self {
            toolkit,
            spec: ToolSpec::base(
                tool_name,
                "Toolkit tool adapted through the AgentScope 2.0 ToolBase contract.",
                serde_json::json!({ "type": "object" }),
            )
            .with_kind(ToolKind::Base),
        }
    }
}

#[async_trait]
impl ToolBase for ToolkitToolAdapter {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn call(&self, invocation: ToolInvocation) -> AgentResult<ToolExecutionResult> {
        debug!(
            trace_id = %invocation.context.runtime.trace_id,
            tool_call_id = %invocation.tool_call_id,
            tool_name = %invocation.tool_name,
            "react_agent toolkit adapter executing guarded tool call"
        );
        let result = {
            let toolkit = self.toolkit.lock().await;
            toolkit
                .call_tool(&invocation.tool_name, invocation.input)
                .await
        };
        match result {
            Ok(response) => Ok(ToolExecutionResult::from_tool_response(response)),
            Err(error) => {
                warn!(
                    trace_id = %invocation.context.runtime.trace_id,
                    tool_name = %invocation.tool_name,
                    error = %error,
                    "react_agent toolkit tool returned an execution error"
                );
                Ok(ToolExecutionResult::Completed {
                    content: vec![ContentBlock::Text(TextBlock {
                        text: format!("Error: {error}"),
                    })],
                    metadata: None,
                    is_error: true,
                })
            }
        }
    }
}

impl ReActAgent {
    pub(super) async fn execute_tool_via_bridge(
        &self,
        runtime: &RuntimeContext,
        tool_call: &ToolUseBlock,
    ) -> AgentResult<ToolExecutionResult> {
        let tool = ToolkitToolAdapter::new(self.toolkit.clone(), &tool_call.name);
        let invocation = ToolInvocation::new(
            tool_call.id.clone(),
            tool_call.name.clone(),
            tool_call.input.clone(),
            ToolRuntimeContext::new(runtime.clone()),
        );
        self.tool_executor.execute(&tool, invocation).await
    }
}

pub(super) fn tool_execution_result_to_block(
    tool_call: &ToolUseBlock,
    result: ToolExecutionResult,
) -> ToolResultBlock {
    let (output, is_error) = match result {
        ToolExecutionResult::Completed {
            content, is_error, ..
        } => (tool_response_text(&content), is_error),
        ToolExecutionResult::Streaming { events, .. } => (streaming_tool_text(&events), false),
        ToolExecutionResult::Suspended { reason, .. } => (reason, true),
        ToolExecutionResult::Denied { reason } => (format!("Error: {reason}"), true),
        ToolExecutionResult::Unavailable { reason } => (format!("Error: {reason}"), true),
    };
    ToolResultBlock {
        tool_use_id: tool_call.id.clone(),
        output,
        name: Some(tool_call.name.clone()),
        is_error,
    }
}

fn streaming_tool_text(events: &[ToolExecutionEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            ToolExecutionEvent::Delta { content } => Some(tool_response_text(content)),
            ToolExecutionEvent::Suspended { reason, .. } => Some(reason.clone()),
            ToolExecutionEvent::Started | ToolExecutionEvent::Completed => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
