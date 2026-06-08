//! Shared DTO shaping, policy translation, and admission helpers for MCP commands.
//!
//! **Pattern:** Adapter support — pure transformations between proto DTOs and the
//! private `McpRuntimeFacade` without embedding application-specific behavior.

use macaca_proto::{
    CapabilityToolInvocationResult, CapabilityToolOriginKind, CapabilityToolPolicyHints,
    McpRuntimeStatusView, McpServiceLifecycleScope, McpServiceSnapshot, McpStatusResult,
    McpToolInvokeCommand, ServiceError, ServiceResult, TraceContext, MCP_SERVICE_ID,
};

use crate::mcp_runtime::{McpRuntimeStatus, McpToolPolicy};

pub(super) fn status_view(status: McpRuntimeStatus) -> McpRuntimeStatusView {
    McpRuntimeStatusView {
        server_id: status.server_id,
        transport: status.transport,
        lifecycle: service_lifecycle(status.lifecycle),
        session_mode: format!("{:?}", status.session_mode),
        state: format!("{:?}", status.state),
        exposed_tools: status.exposed_tools,
        failure_reason: status.failure_reason,
    }
}

pub(super) fn status_result(statuses: Vec<McpRuntimeStatus>) -> McpStatusResult {
    McpStatusResult::new(statuses.into_iter().map(status_view).collect())
}

pub(super) fn validate_invocation_command(command: &McpToolInvokeCommand) -> Option<&'static str> {
    if command.invocation.trace.trace_id.trim().is_empty() {
        return Some("missing_trace");
    }
    if command.invocation.scope.session_id.trim().is_empty()
        || command.invocation.scope.agent_name.trim().is_empty()
    {
        return Some("scope_missing");
    }
    if command.invocation.tool_name.trim().is_empty() {
        return Some("tool_missing");
    }
    if command
        .server_id
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Some("server_missing");
    }
    if command
        .backend_tool_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Some("backend_tool_missing");
    }
    if command.lifecycle.is_none() {
        return Some("lifecycle_missing");
    }
    None
}

pub(super) fn failed_invocation_result(
    visible_tool_name: &str,
    reason: impl Into<String>,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    let reason = sanitize_reason(reason);
    let mut result = CapabilityToolInvocationResult::failed(
        MCP_SERVICE_ID,
        CapabilityToolOriginKind::Mcp,
        visible_tool_name,
        reason.clone(),
        trace,
    );
    result
        .metadata
        .insert("mcp.policy_decision".into(), "deny".into());
    result.metadata.insert("mcp.reason_code".into(), reason);
    result
}

pub(super) fn runtime_policy_from_capability_hints(hints: &CapabilityToolPolicyHints) -> McpToolPolicy {
    let mut policy = McpToolPolicy::default();
    if let Some(deny_tools) = hints.metadata.get("mcp.deny_tools") {
        policy.deny_tools = deny_tools
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if let Some(deny_servers) = hints.metadata.get("mcp.deny_servers") {
        policy.deny_servers = deny_servers
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    policy
}

pub(super) fn sanitize_reason(reason: impl Into<String>) -> String {
    let mut value = reason.into().replace('\n', " ").replace('\r', " ");
    value.truncate(240);
    value
}

pub(super) fn snapshot_from_statuses(
    registered_definitions: usize,
    definitions: Vec<serde_json::Value>,
    statuses: Vec<McpRuntimeStatus>,
) -> McpServiceSnapshot {
    let mut ready = 0usize;
    let mut failed = 0usize;
    let mut dependency_missing = 0usize;
    let mut disabled = 0usize;
    let mut exposed_tool_count = 0usize;
    let mut lifecycle_scopes = Vec::new();
    let mut failure_reasons = Vec::new();
    for status in statuses {
        match status.state {
            crate::mcp_runtime::McpRuntimeStatusState::Ready => ready += 1,
            crate::mcp_runtime::McpRuntimeStatusState::Failed => failed += 1,
            crate::mcp_runtime::McpRuntimeStatusState::DependencyMissing => dependency_missing += 1,
            crate::mcp_runtime::McpRuntimeStatusState::Disabled => disabled += 1,
        }
        exposed_tool_count += status.exposed_tools.len();
        lifecycle_scopes.push(service_lifecycle(status.lifecycle));
        if let Some(reason) = status.failure_reason {
            failure_reasons.push(reason);
        }
    }
    lifecycle_scopes.sort();
    lifecycle_scopes.dedup();
    McpServiceSnapshot {
        service_id: MCP_SERVICE_ID.into(),
        healthy: failed == 0,
        registered_definitions,
        definitions,
        ready,
        failed,
        dependency_missing,
        disabled,
        exposed_tool_count,
        lifecycle_scopes,
        failure_reasons,
        captured_at: chrono::Utc::now(),
    }
}

/// Convert runtime-owned server definitions into snapshot DTO payloads.
///
/// The provider deliberately serializes definitions at the service edge instead
/// of exposing `McpRuntimeFacade` to Web.  Serialization failures are logged and
/// skipped so one malformed extension cannot block the entire diagnostic
/// snapshot; the registered count still reports the full runtime inventory for
/// operator visibility.
pub(super) fn service_definition_payloads(
    include_definitions: bool,
    definitions: Vec<crate::McpServerDefinition>,
) -> Vec<serde_json::Value> {
    if !include_definitions {
        return Vec::new();
    }

    definitions
        .into_iter()
        .filter_map(|definition| match serde_json::to_value(&definition) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(
                    server_id = %definition.id,
                    error = %error,
                    "mcp service snapshot skipped unserializable definition"
                );
                None
            }
        })
        .collect()
}

pub(super) fn service_lifecycle(lifecycle: crate::mcp_runtime::McpLifecycleScope) -> McpServiceLifecycleScope {
    match lifecycle {
        crate::mcp_runtime::McpLifecycleScope::Global => McpServiceLifecycleScope::Global,
        crate::mcp_runtime::McpLifecycleScope::App => McpServiceLifecycleScope::App,
        crate::mcp_runtime::McpLifecycleScope::Session => McpServiceLifecycleScope::Session,
        crate::mcp_runtime::McpLifecycleScope::AgentSession => {
            McpServiceLifecycleScope::AgentSession
        }
        crate::mcp_runtime::McpLifecycleScope::Call => McpServiceLifecycleScope::Call,
    }
}

pub(super) fn runtime_policy(snapshot: macaca_proto::McpToolPolicySnapshot) -> McpToolPolicy {
    McpToolPolicy {
        allow_servers: snapshot
            .allow_servers
            .map(|items| items.into_iter().collect()),
        deny_servers: snapshot.deny_servers.into_iter().collect(),
        allow_tools: snapshot
            .allow_tools
            .map(|items| items.into_iter().collect()),
        deny_tools: snapshot.deny_tools.into_iter().collect(),
    }
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))
}

pub(super) fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}
