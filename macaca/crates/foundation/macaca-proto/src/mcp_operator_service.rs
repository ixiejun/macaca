//! Operator-grade MCP lifecycle DTOs shared by SDK and runtime-host.
//!
//! The base MCP service DTOs cover registration, catalog, invocation, status,
//! and cleanup.  This module adds operator lifecycle commands that are useful
//! for shells, applications, and automation controllers: reload, OAuth state,
//! resource access, diagnostics, watched-change reporting, and per-thread
//! exposure refresh.  The DTOs stay provider-neutral and carry only bounded,
//! sanitized metadata so service snapshots do not leak secrets or raw payloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{MacacaError, MacacaResult, McpServicePolicyHints, McpServiceScope, TraceContext};

pub const MCP_SERVER_STATUS_LIST_COMMAND: &str = "mcp.server.status.list";
pub const MCP_RELOAD_COMMAND: &str = "mcp.server.reload";
pub const MCP_RESOURCE_LIST_COMMAND: &str = "mcp.resource.list";
pub const MCP_RESOURCE_TEMPLATE_LIST_COMMAND: &str = "mcp.resource_template.list";
pub const MCP_RESOURCE_READ_COMMAND: &str = "mcp.resource.read";
pub const MCP_OAUTH_LOGIN_COMMAND: &str = "mcp.oauth.login";
pub const MCP_OAUTH_STATUS_COMMAND: &str = "mcp.oauth.status";
pub const MCP_DIAGNOSTICS_SNAPSHOT_COMMAND: &str = "mcp.diagnostics.snapshot";
pub const MCP_EXPOSURE_REFRESH_COMMAND: &str = "mcp.exposure.refresh";

/// OAuth state is explicit so callers never infer auth from transport errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpOAuthState {
    NotRequired,
    Required,
    LoginStarted,
    LoginCompleted,
    Failed,
    Unavailable,
}

impl Default for McpOAuthState {
    fn default() -> Self {
        Self::NotRequired
    }
}

/// Redacted status record used by operator dashboards and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOperatorServerStatus {
    pub server_id: String,
    pub state: String,
    pub auth_state: McpOAuthState,
    pub tool_count: usize,
    pub resource_count: usize,
    pub resource_template_count: usize,
    pub exposure_generation: u64,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerStatusListCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub policy: McpServicePolicyHints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerStatusListResult {
    pub statuses: Vec<McpOperatorServerStatus>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpReloadCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    #[serde(default)]
    pub definitions: Vec<Value>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpWatchedChange {
    pub server_id: String,
    pub change_kind: String,
    pub sanitized_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpReloadResult {
    pub reloaded: usize,
    pub exposure_generation: u64,
    pub pending_thread_ids: Vec<String>,
    pub watched_changes: Vec<McpWatchedChange>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExposureRefreshCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub thread_id: String,
    #[serde(default)]
    pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpExposureRefreshResult {
    pub thread_id: String,
    pub refreshed: bool,
    pub exposure_generation: u64,
    pub visible_tool_count: usize,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthLoginCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub server_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOAuthStatusResult {
    pub server_id: String,
    pub state: McpOAuthState,
    pub login_flow_ref: Option<String>,
    pub sanitized_message: Option<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthStatusCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceListCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub policy: McpServicePolicyHints,
    #[serde(default)]
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceDescriptor {
    pub server_id: String,
    pub uri: String,
    pub name: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceListResult {
    pub resources: Vec<McpResourceDescriptor>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceTemplateDescriptor {
    pub server_id: String,
    pub uri_template: String,
    pub name: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceTemplateListResult {
    pub templates: Vec<McpResourceTemplateDescriptor>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceReadCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
    pub server_id: String,
    pub uri: String,
    #[serde(default = "default_resource_limit")]
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceReadResult {
    pub server_id: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob_base64: Option<String>,
    pub truncated: bool,
    pub error_summary: Option<String>,
    pub captured_at: DateTime<Utc>,
}

impl McpResourceReadResult {
    pub fn denied(
        server_id: impl Into<String>,
        uri: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            uri: uri.into(),
            mime_type: None,
            text: None,
            blob_base64: None,
            truncated: false,
            error_summary: Some(sanitize(reason.into())),
            captured_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiagnosticsSnapshotCommand {
    pub trace: TraceContext,
    pub scope: McpServiceScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpDiagnosticsSnapshot {
    pub service_id: String,
    pub exposure_generation: u64,
    pub server_count: usize,
    pub auth_required: usize,
    pub ready: usize,
    pub failed: usize,
    pub captured_at: DateTime<Utc>,
}

pub fn validate_operator_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

pub fn sanitize(value: String) -> String {
    let mut sanitized = value.replace('\n', " ").replace('\r', " ");
    sanitized.truncate(240);
    sanitized
}

fn default_resource_limit() -> usize {
    64 * 1024
}
