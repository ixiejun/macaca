//! Provider-neutral MCP service DTOs shared by SDK and runtime-host.
//!
//! These DTOs live in `macaca-proto` to avoid an SDK -> runtime-host dependency
//! cycle.  Runtime-host remains the owner of MCP protocol lifecycle and adapts
//! the JSON definition payloads into its host-local `McpServerDefinition`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocation,
    CapabilityToolInvocationResult, MacacaError, MacacaResult, TraceContext,
};

pub const MCP_SERVICE_ID: &str = "service.mcp";
pub const MCP_REGISTER_COMMAND: &str = "mcp.register";
pub const MCP_PROBE_COMMAND: &str = "mcp.probe";
pub const MCP_TOOL_CATALOG_COMMAND: &str = "mcp.tool.catalog";
pub const MCP_TOOL_ATTACH_COMMAND: &str = "mcp.tool.attach";
pub const MCP_TOOL_INVOKE_COMMAND: &str = "mcp.tool.invoke";
pub const MCP_STATUS_COMMAND: &str = "mcp.status";
pub const MCP_SNAPSHOT_COMMAND: &str = "mcp.service.snapshot";
pub const MCP_CLEANUP_COMMAND: &str = "mcp.cleanup";

pub const MCP_DESCRIPTOR_BACKEND_TOOL_NAME: &str = "mcp.backend_tool_name";
pub const MCP_DESCRIPTOR_LIFECYCLE_SCOPE: &str = "mcp.lifecycle_scope";
pub const MCP_DESCRIPTOR_DEFINITION_SOURCE: &str = "mcp.definition_source";

/// Provider-neutral lifecycle scope for MCP resources.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServiceLifecycleScope {
    Global,
    App,
    Session,
    AgentSession,
    Call,
}

impl Default for McpServiceLifecycleScope {
    fn default() -> Self {
        Self::Session
    }
}

/// Explicit scope for MCP service commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServiceScope {
    pub application_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
    pub lifecycle: McpServiceLifecycleScope,
}

impl McpServiceScope {
    /// Build agent-session scoped metadata for MCP attachment and cleanup.
    pub fn agent_session(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            session_id: Some(non_empty(
                session_id.into(),
                "mcp service requires session_id",
            )?),
            agent_name: Some(non_empty(
                agent_name.into(),
                "mcp service requires agent_name",
            )?),
            lifecycle: McpServiceLifecycleScope::AgentSession,
        })
    }
}

impl Default for McpServiceScope {
    fn default() -> Self {
        Self {
            application_id: None,
            session_id: None,
            agent_name: None,
            lifecycle: McpServiceLifecycleScope::Session,
        }
    }
}

/// Serializable policy subset used at the MCP service boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolPolicySnapshot {
    pub allow_servers: Option<Vec<String>>,
    pub deny_servers: Vec<String>,
    pub allow_tools: Option<Vec<String>>,
    pub deny_tools: Vec<String>,
}

/// Policy hints interpreted by MCP service adapters and runtime decorators.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServicePolicyHints {
    pub tool_policy: McpToolPolicySnapshot,
    pub metadata: BTreeMap<String, String>,
}

/// Command for registering MCP definitions.
///
/// Definitions are JSON payloads so SDK can stay independent from host-local
/// transport structs. Runtime-host validates and converts them before use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRegisterCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub definitions: Vec<Value>,
    pub policy: McpServicePolicyHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProbeCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub policy: McpServicePolicyHints,
}

impl McpProbeCommand {
    pub fn new(trace: TraceContext, policy: McpToolPolicySnapshot) -> MacacaResult<Self> {
        validate_trace(&trace, "mcp probe command requires trace_id")?;
        Ok(Self {
            trace,
            scope: McpServiceScope::default(),
            policy: McpServicePolicyHints {
                tool_policy: policy,
                metadata: BTreeMap::new(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCatalogCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub policy: McpServicePolicyHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolAttachCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub definitions: Vec<Value>,
    pub policy: McpServicePolicyHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInvokeCommand {
    pub invocation: CapabilityToolInvocation,
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub backend_tool_name: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<McpServiceLifecycleScope>,
}

impl McpToolInvokeCommand {
    /// Build an invocation command from explicit descriptor routing metadata.
    ///
    /// The visible tool name remains in `invocation.tool_name` because it is
    /// what the model called.  `server_id` and `backend_tool_name` are the
    /// service-owned routing facts that prevent production callers from
    /// guessing server identity by parsing a display name.
    pub fn routed(
        invocation: CapabilityToolInvocation,
        server_id: impl Into<String>,
        backend_tool_name: impl Into<String>,
        lifecycle: McpServiceLifecycleScope,
    ) -> MacacaResult<Self> {
        Ok(Self {
            invocation,
            server_id: Some(non_empty(
                server_id.into(),
                "mcp tool invocation requires server_id",
            )?),
            backend_tool_name: Some(non_empty(
                backend_tool_name.into(),
                "mcp tool invocation requires backend_tool_name",
            )?),
            lifecycle: Some(lifecycle),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatusCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub policy: McpServicePolicyHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceSnapshotCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub include_definitions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCleanupCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRegisterResult {
    pub registered: usize,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRuntimeStatusView {
    pub server_id: String,
    pub transport: String,
    pub lifecycle: McpServiceLifecycleScope,
    pub session_mode: String,
    pub state: String,
    pub exposed_tools: Vec<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpStatusResult {
    pub statuses: Vec<McpRuntimeStatusView>,
    pub captured_at: DateTime<Utc>,
}

impl McpStatusResult {
    pub fn new(statuses: Vec<McpRuntimeStatusView>) -> Self {
        Self {
            statuses,
            captured_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolCatalogResult {
    pub tools: Vec<CapabilityToolDescriptor>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolAttachResult {
    pub statuses: Vec<McpRuntimeStatusView>,
    pub conflicts: Vec<String>,
    pub applied_prefixes: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub registered_definitions: usize,
    pub ready: usize,
    pub failed: usize,
    pub dependency_missing: usize,
    pub disabled: usize,
    pub exposed_tool_count: usize,
    pub lifecycle_scopes: Vec<McpServiceLifecycleScope>,
    pub failure_reasons: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl McpServiceSnapshot {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: MCP_SERVICE_ID.into(),
            healthy: false,
            registered_definitions: 0,
            ready: 0,
            failed: 0,
            dependency_missing: 0,
            disabled: 0,
            exposed_tool_count: 0,
            lifecycle_scopes: Vec::new(),
            failure_reasons: vec![reason.into()],
            captured_at: Utc::now(),
        }
    }
}

pub type McpToolInvokeResult = CapabilityToolInvocationResult;

fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
