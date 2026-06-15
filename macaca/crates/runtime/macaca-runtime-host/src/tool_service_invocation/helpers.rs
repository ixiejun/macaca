//! Shared helpers for invocation routing, reply normalization, and reference generation.
//!
//! These pure functions adapt owner-service replies and descriptor metadata into the
//! uniform `CapabilityToolInvocationResult` shape consumed by result budgeting and audit.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use macaca_proto::{
    CapabilityToolInvocationResult, CapabilityToolOriginKind, IndustrialToolDescriptor,
    MacacaError, MacacaResult, McpServiceLifecycleScope, ToolInvocationRef, ToolInvokeCommand,
    TraceContext, MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_DESCRIPTOR_LIFECYCLE_SCOPE,
};

use crate::tool_service_result::bounded_summary;
use crate::ServiceRuntimeError;

/// Monotonic sequence used to mint unique invocation refs per trace.
static INVOCATION_SEQ: AtomicU64 = AtomicU64::new(1);

/// Allocate the next invocation reference for audit correlation within a trace.
pub(crate) fn next_invocation_ref(trace: &TraceContext) -> ToolInvocationRef {
    ToolInvocationRef(format!(
        "tool-invocation-{}-{}",
        trace.trace_id,
        INVOCATION_SEQ.fetch_add(1, Ordering::SeqCst)
    ))
}

/// Resolve inline JSON budget from caller metadata with a conservative default.
pub(crate) fn inline_budget_bytes(metadata: &BTreeMap<String, String>) -> usize {
    metadata
        .get("result.inline_budget_bytes")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(16 * 1024)
}

/// Normalize heterogeneous owner-service replies into `CapabilityToolInvocationResult`.
pub(crate) fn wrap_service_command_reply(
    service_id: String,
    command_name: &str,
    output: serde_json::Value,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    // Workbench-family services return their own typed WorkbenchCommandResult envelopes.
    // service.tool normalizes those replies into the historical CapabilityToolInvocationResult
    // shape so result budgeting, artifact storage, and audit recording stay uniform.
    let mut result = if let Some(reason) = workbench_unavailable_reason(&output) {
        CapabilityToolInvocationResult::failed(
            service_id,
            CapabilityToolOriginKind::Mcp,
            command_name,
            reason,
            trace,
        )
    } else {
        CapabilityToolInvocationResult::ok(
            service_id,
            CapabilityToolOriginKind::Mcp,
            command_name,
            output,
            trace,
        )
    };
    result.metadata.insert(
        "service_tool.normalized_reply".into(),
        "workbench_command".into(),
    );
    result
}

/// Extract unavailable reason from workbench-style command result envelopes.
fn workbench_unavailable_reason(output: &serde_json::Value) -> Option<String> {
    output
        .get("status")
        .and_then(|status| status.get("Unavailable"))
        .and_then(|unavailable| unavailable.get("reason"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Build the JSON payload forwarded to an owning service command.
///
/// Family-specific `*.tool.invoke` commands receive the full envelope; typed service
/// commands receive only the caller input DTO so routing metadata stays authoritative.
pub(crate) fn owning_service_payload(
    command_name: &str,
    command: &ToolInvokeCommand,
) -> MacacaResult<serde_json::Value> {
    if command_name.ends_with(".tool.invoke") || command_name == "tool.invoke" {
        serde_json::to_value(command).map_err(MacacaError::from)
    } else {
        Ok(command.input.clone())
    }
}

/// Resolve MCP backend tool name from descriptor metadata with visible-name fallback.
pub(crate) fn backend_tool_name(descriptor: &IndustrialToolDescriptor) -> MacacaResult<String> {
    descriptor
        .base_descriptor
        .metadata
        .get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME)
        .cloned()
        .unwrap_or_else(|| descriptor.visible_name.clone())
        .trim()
        .to_string()
        .pipe_non_empty("mcp tool invocation requires backend tool name")
}

/// Parse MCP lifecycle scope from descriptor metadata.
pub(crate) fn lifecycle(
    descriptor: &IndustrialToolDescriptor,
) -> MacacaResult<McpServiceLifecycleScope> {
    match descriptor
        .base_descriptor
        .metadata
        .get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE)
        .map(String::as_str)
        .unwrap_or("agent_session")
    {
        "global" => Ok(McpServiceLifecycleScope::Global),
        "app" => Ok(McpServiceLifecycleScope::App),
        "session" => Ok(McpServiceLifecycleScope::Session),
        "agent_session" => Ok(McpServiceLifecycleScope::AgentSession),
        "call" => Ok(McpServiceLifecycleScope::Call),
        other => Err(MacacaError::Config(format!(
            "unsupported MCP descriptor lifecycle '{}'",
            bounded_summary(other.into())
        ))),
    }
}

/// Map service runtime failures into configuration errors for tool command results.
pub(crate) fn runtime_error(error: ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

trait NonEmptyString {
    fn pipe_non_empty(self, message: &'static str) -> MacacaResult<String>;
}

impl NonEmptyString for String {
    fn pipe_non_empty(self, message: &'static str) -> MacacaResult<String> {
        if self.is_empty() {
            Err(MacacaError::Config(message.into()))
        } else {
            Ok(self)
        }
    }
}
