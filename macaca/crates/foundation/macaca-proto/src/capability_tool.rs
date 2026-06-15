//! Provider-neutral tool metadata shared by Driver, Skill, and MCP services.
//!
//! Protocol serviceization keeps each capability service responsible for its
//! own lifecycle and invocation, but upper layers still need one safe shape for
//! model-visible tool metadata.  The DTOs in this module are that common
//! boundary: they describe a tool without exposing provider credentials,
//! environment, headers, command secrets, or unsanitized payloads.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApplicationId, MacacaError, MacacaResult, TraceContext};

/// Service family that produced a tool descriptor.
///
/// The enum is intentionally small because it models S6 ownership boundaries:
/// Driver, Skill, and MCP keep separate lifecycle semantics even though their
/// tools share one metadata envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityToolOriginKind {
    Driver,
    Skill,
    Mcp,
}

/// Provider-neutral resource scope hint used by tool policy and cleanup.
///
/// This is a hint, not an authorization decision.  Runtime decorators and
/// service providers still validate scope before dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityToolResourceScope {
    Global,
    Application,
    Session,
    AgentSession,
    Call,
}

/// Sanitized model-visible metadata for one capability tool.
///
/// The descriptor is a Memento snapshot of safe metadata.  It never owns the
/// tool runtime and never transfers invocation authority away from the service
/// named by `service_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityToolDescriptor {
    pub service_id: String,
    pub provider_id: String,
    pub capability_id: String,
    pub tool_name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub parameters_schema: Value,
    pub origin_kind: CapabilityToolOriginKind,
    pub required_permission_hints: Vec<String>,
    pub resource_scope_hints: Vec<CapabilityToolResourceScope>,
    pub conflict_namespace: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl CapabilityToolDescriptor {
    /// Build a sanitized descriptor for a service-owned tool.
    pub fn new(
        service_id: impl Into<String>,
        provider_id: impl Into<String>,
        capability_id: impl Into<String>,
        tool_name: impl Into<String>,
        description: impl Into<String>,
        parameters_schema: Value,
        origin_kind: CapabilityToolOriginKind,
    ) -> MacacaResult<Self> {
        let service_id = non_empty(service_id.into(), "tool descriptor requires service_id")?;
        let provider_id = non_empty(provider_id.into(), "tool descriptor requires provider_id")?;
        let capability_id = non_empty(
            capability_id.into(),
            "tool descriptor requires capability_id",
        )?;
        let tool_name = non_empty(tool_name.into(), "tool descriptor requires tool_name")?;
        Ok(Self {
            service_id,
            provider_id,
            capability_id,
            tool_name,
            display_name: None,
            description: description.into(),
            parameters_schema,
            origin_kind,
            required_permission_hints: Vec::new(),
            resource_scope_hints: Vec::new(),
            conflict_namespace: None,
            metadata: BTreeMap::new(),
        })
    }

    /// Attach presentation metadata that helps conflict policies avoid
    /// hardcoded service/provider/tool names.
    pub fn with_display(mut self, display_name: Option<String>, namespace: Option<String>) -> Self {
        self.display_name = display_name;
        self.conflict_namespace = namespace;
        self
    }

    /// Attach permission and resource-scope hints for policy strategies.
    pub fn with_policy_hints(
        mut self,
        permissions: Vec<String>,
        scopes: Vec<CapabilityToolResourceScope>,
    ) -> Self {
        self.required_permission_hints = permissions;
        self.resource_scope_hints = scopes;
        self
    }
}

/// Provider-neutral policy hints attached to a tool invocation.
///
/// These hints are plain data so different hosts can plug in their own policy
/// engine without changing Driver, Skill, or MCP service command schemas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToolPolicyHints {
    pub permission_hints: Vec<String>,
    pub resource_scope: Option<CapabilityToolResourceScope>,
    pub metadata: BTreeMap<String, String>,
}

/// Shared scope required by all Driver/Skill/MCP tool invocations.
///
/// Explicit scope makes audit and cleanup deterministic.  Services must not
/// infer application/session/agent ownership from raw tool input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToolInvocationScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
}

impl CapabilityToolInvocationScope {
    /// Build a validated invocation scope.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id,
            session_id: non_empty(session_id.into(), "tool invocation requires session_id")?,
            agent_name: non_empty(agent_name.into(), "tool invocation requires agent_name")?,
        })
    }
}

/// Shared invocation envelope used by service-specific tool commands.
///
/// The envelope carries raw JSON input because tools are schema-driven, but the
/// envelope itself does not permit secrets to be mirrored into descriptors,
/// snapshots, or logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityToolInvocation {
    pub trace: TraceContext,
    pub scope: CapabilityToolInvocationScope,
    pub tool_name: String,
    pub input: Value,
    pub policy: CapabilityToolPolicyHints,
}

impl CapabilityToolInvocation {
    /// Build an invocation and reject untraceable or unscoped calls before they
    /// enter service dispatch.
    pub fn new(
        trace: TraceContext,
        scope: CapabilityToolInvocationScope,
        tool_name: impl Into<String>,
        input: Value,
    ) -> MacacaResult<Self> {
        if trace.trace_id.trim().is_empty() {
            return Err(MacacaError::Config(
                "tool invocation requires trace_id".into(),
            ));
        }
        Ok(Self {
            trace,
            scope,
            tool_name: non_empty(tool_name.into(), "tool invocation requires tool_name")?,
            input,
            policy: CapabilityToolPolicyHints::default(),
        })
    }
}

/// Structured result returned by capability tool services.
///
/// `output` may contain tool output when it is safe for the caller.  Logs and
/// snapshots should use `error_summary`, `status`, and origin metadata rather
/// than duplicating raw payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityToolInvocationResult {
    pub service_id: String,
    pub origin_kind: CapabilityToolOriginKind,
    pub tool_name: String,
    pub status: String,
    pub output: Option<Value>,
    pub error_summary: Option<String>,
    pub trace: TraceContext,
    pub completed_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl CapabilityToolInvocationResult {
    /// Build a successful tool result with service origin metadata.
    pub fn ok(
        service_id: impl Into<String>,
        origin_kind: CapabilityToolOriginKind,
        tool_name: impl Into<String>,
        output: Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            origin_kind,
            tool_name: tool_name.into(),
            status: "ok".into(),
            output: Some(output),
            error_summary: None,
            trace,
            completed_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a failed tool result with a sanitized error summary.
    pub fn failed(
        service_id: impl Into<String>,
        origin_kind: CapabilityToolOriginKind,
        tool_name: impl Into<String>,
        error_summary: impl Into<String>,
        trace: TraceContext,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            origin_kind,
            tool_name: tool_name.into(),
            status: "failed".into(),
            output: None,
            error_summary: Some(error_summary.into()),
            trace,
            completed_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
