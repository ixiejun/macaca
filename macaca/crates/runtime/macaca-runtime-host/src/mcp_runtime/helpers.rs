//! Shared pure helpers for MCP runtime modules.
//!
//! Keying rules, status builders, invocation failure DTOs, and hash utilities.

use std::path::PathBuf;
use std::time::Duration;

use macaca_framework::mcp::McpTransportConfig;
use macaca_proto::{
    CapabilityToolInvocationResult, CapabilityToolOriginKind, CapabilityToolResourceScope,
    TraceContext, MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_SERVICE_ID,
};

use crate::mcp_descriptor_index::McpToolDescriptorRoute;

use super::types::{
    McpLifecycleScope, McpRuntimeContext, McpRuntimeKey, McpRuntimeStatus, McpRuntimeStatusState,
    McpServerDefinition, McpToolPolicy,
};

pub(crate) fn missing_required_bin(definition: &McpServerDefinition) -> Option<String> {
    let mut required = definition.required_bins.clone();
    if let McpTransportConfig::Stdio { command, .. } = &definition.transport {
        required.push(command.clone());
    }
    required.into_iter().find(|bin| !command_exists(bin))
}

pub(crate) fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return PathBuf::from(command).is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

pub(crate) fn status_for_definition(
    definition: &McpServerDefinition,
    state: McpRuntimeStatusState,
    exposed_tools: Vec<String>,
    failure_reason: Option<String>,
) -> McpRuntimeStatus {
    McpRuntimeStatus {
        server_id: definition.id.clone(),
        transport: transport_name(&definition.transport).to_string(),
        lifecycle: definition.lifecycle.clone(),
        session_mode: definition.session_mode,
        state,
        exposed_tools,
        failure_reason,
    }
}

pub(crate) fn transport_name(config: &McpTransportConfig) -> &'static str {
    match config {
        McpTransportConfig::Stdio { .. } => "stdio",
        McpTransportConfig::Sse { .. } => "sse",
        McpTransportConfig::StreamableHttp { .. } => "streamable_http",
    }
}

impl McpRuntimeKey {
    /// Build the service-owned lifecycle key for an MCP server and caller scope.
    pub(crate) fn from_definition(
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> Self {
        runtime_key(definition, context)
    }
}

pub(crate) fn runtime_key(
    definition: &McpServerDefinition,
    context: &McpRuntimeContext,
) -> McpRuntimeKey {
    let app_id = context.app_id.as_ref().map(|id| id.0.to_string());
    let session_id = context.session_id.clone();
    let agent_name = context.agent_name.clone();
    match definition.lifecycle {
        McpLifecycleScope::Global => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Global,
            app_id: None,
            session_id: None,
            agent_name: None,
        },
        McpLifecycleScope::App => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::App,
            app_id,
            session_id: None,
            agent_name: None,
        },
        McpLifecycleScope::Session => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Session,
            app_id,
            session_id,
            agent_name: None,
        },
        McpLifecycleScope::AgentSession => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::AgentSession,
            app_id,
            session_id,
            agent_name,
        },
        McpLifecycleScope::Call => McpRuntimeKey {
            server_id: definition.id.clone(),
            scope: McpLifecycleScope::Call,
            app_id,
            session_id,
            agent_name,
        },
    }
}

pub(crate) fn prefixed_tool_name(definition: &McpServerDefinition, tool_name: &str) -> String {
    definition
        .tool_prefix
        .as_ref()
        .map(|prefix| format!("{prefix}{tool_name}"))
        .unwrap_or_else(|| tool_name.to_string())
}

pub(crate) fn failed_invocation_result(
    visible_tool_name: &str,
    error_summary: impl Into<String>,
    trace: TraceContext,
) -> CapabilityToolInvocationResult {
    CapabilityToolInvocationResult::failed(
        MCP_SERVICE_ID,
        CapabilityToolOriginKind::Mcp,
        visible_tool_name,
        sanitize_error(error_summary),
        trace,
    )
}

pub(crate) fn failed_invocation_result_with_metadata(
    visible_tool_name: &str,
    error_summary: impl Into<String>,
    trace: TraceContext,
    server_id: &str,
    backend_tool_name: &str,
    lifecycle: &McpLifecycleScope,
    input_hash: String,
    elapsed: Duration,
) -> CapabilityToolInvocationResult {
    let reason = sanitize_error(error_summary);
    let mut result = failed_invocation_result(visible_tool_name, reason.clone(), trace);
    result
        .metadata
        .insert("mcp.server_id".into(), server_id.into());
    result.metadata.insert(
        MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(),
        backend_tool_name.into(),
    );
    result.metadata.insert(
        "mcp.lifecycle_scope".into(),
        lifecycle_scope_name(lifecycle).into(),
    );
    result
        .metadata
        .insert("mcp.policy_decision".into(), "allow".into());
    result.metadata.insert("mcp.reason_code".into(), reason);
    result.metadata.insert("mcp.input_hash".into(), input_hash);
    result.metadata.insert(
        "mcp.latency_ms".into(),
        (elapsed.as_millis() as u64).to_string(),
    );
    result
}

pub(crate) fn validate_descriptor_route(
    route: &McpToolDescriptorRoute,
    server_id: &str,
    backend_tool_name: &str,
    visible_tool_name: &str,
) -> Option<&'static str> {
    if route.server_id != server_id {
        return Some("mcp_descriptor_server_mismatch");
    }
    if route.backend_tool_name != backend_tool_name {
        return Some("mcp_descriptor_backend_tool_mismatch");
    }
    if route.visible_tool_name != visible_tool_name {
        return Some("mcp_descriptor_visible_tool_mismatch");
    }
    None
}

pub(crate) fn stable_json_hash(value: &serde_json::Value) -> String {
    let serialized = serde_json::to_string(value).unwrap_or_else(|_| "unserializable".into());
    stable_text_hash(&serialized)
}

pub(crate) fn stable_text_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

pub(crate) fn sanitize_error(error: impl Into<String>) -> String {
    let value = error.into();
    let mut sanitized = value.replace('\n', " ").replace('\r', " ");
    sanitized.truncate(240);
    sanitized
}

/// Map MCP lifecycle scope to capability-tool resource scope for policy hints.
pub(crate) fn resource_scope_for_lifecycle(
    lifecycle: &McpLifecycleScope,
) -> CapabilityToolResourceScope {
    match lifecycle {
        McpLifecycleScope::Global => CapabilityToolResourceScope::Global,
        McpLifecycleScope::App => CapabilityToolResourceScope::Application,
        McpLifecycleScope::Session => CapabilityToolResourceScope::Session,
        McpLifecycleScope::AgentSession => CapabilityToolResourceScope::AgentSession,
        McpLifecycleScope::Call => CapabilityToolResourceScope::Call,
    }
}

/// Stable string label for lifecycle scope used in audit metadata.
pub(crate) fn lifecycle_scope_name(lifecycle: &McpLifecycleScope) -> &'static str {
    match lifecycle {
        McpLifecycleScope::Global => "global",
        McpLifecycleScope::App => "app",
        McpLifecycleScope::Session => "session",
        McpLifecycleScope::AgentSession => "agent_session",
        McpLifecycleScope::Call => "call",
    }
}

/// Pre-flight policy/dependency check before resource or descriptor access.
pub(crate) fn resource_access_error(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> Option<String> {
    if !definition.enabled || !policy.allows_server(&definition.id) {
        return Some("mcp_server_denied".into());
    }
    missing_required_bin(definition).map(|missing| format!("missing dependency: {missing}"))
}

/// Collapse tokio timeout + MCP protocol errors into a single string for status reporting.
pub(crate) fn flatten_timeout_result<T>(
    result: Result<Result<T, macaca_framework::mcp::McpError>, tokio::time::error::Elapsed>,
) -> Result<T, String> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("connect_timeout".to_string()),
    }
}
