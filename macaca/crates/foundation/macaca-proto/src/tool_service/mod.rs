//! Provider-neutral industrial Tool Capability Plane contracts.
//!
//! The Tool service defines catalog planning, invocation, artifact access, and audit query
//! vocabulary for the industrial capability plane. This facade module keeps command DTOs in
//! `mod.rs` while descriptor/plan vocabulary lives in `descriptor_vocabulary.rs` so each file
//! stays under the OS 500-line constitution without changing public exports.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ApplicationId, CapabilityId, CapabilityToolInvocationScope, KernelServiceId, MacacaError,
    MacacaResult, ServiceCapability, ServiceDescriptor, ServiceHealth, ServiceScope, ServiceType,
    TraceContext, TraceSchemaRef,
};

pub const TOOL_SERVICE_ID: &str = "service.tool";
pub const TOOL_CATALOG_PLAN_COMMAND: &str = "tool.catalog.plan";
pub const TOOL_CATALOG_SNAPSHOT_COMMAND: &str = "tool.catalog.snapshot";
pub const TOOL_TOOLSET_RESOLVE_COMMAND: &str = "tool.toolset.resolve";
pub const TOOL_INVOKE_COMMAND: &str = "tool.invoke";
pub const TOOL_INVOKE_CANCEL_COMMAND: &str = "tool.invoke.cancel";
pub const TOOL_INVOCATION_STATUS_COMMAND: &str = "tool.invocation.status";
pub const TOOL_RESULT_GET_COMMAND: &str = "tool.result.get";
pub const TOOL_ARTIFACT_OPEN_COMMAND: &str = "tool.artifact.open";
pub const TOOL_PROVIDER_STATUS_COMMAND: &str = "tool.provider.status";
pub const TOOL_PROVIDER_HEALTH_COMMAND: &str = "tool.provider.health";
pub const TOOL_POLICY_EXPLAIN_COMMAND: &str = "tool.policy.explain";
pub const TOOL_AUDIT_QUERY_COMMAND: &str = "tool.audit.query";

mod descriptor_vocabulary;

pub use descriptor_vocabulary::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolAuditRef(String);

impl ToolAuditRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolPolicyRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolApprovalRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolArtifactRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolInvocationRef(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCatalogPlanCommand {
    pub trace: TraceContext,
    pub application_id: Option<ApplicationId>,
    pub agent_name: Option<String>,
    pub requested_toolsets: Vec<ToolsetRef>,
    pub requested_families: Vec<ToolFamilyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub denied_families: Vec<ToolFamilyRef>,
    pub include_hidden: bool,
    pub metadata: BTreeMap<String, String>,
}

impl ToolCatalogPlanCommand {
    pub fn new(trace: TraceContext) -> MacacaResult<Self> {
        validate_trace(&trace)?;
        Ok(Self {
            trace,
            application_id: None,
            agent_name: None,
            requested_toolsets: Vec::new(),
            requested_families: Vec::new(),
            allowed_tools: Vec::new(),
            denied_families: Vec::new(),
            include_hidden: false,
            metadata: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCatalogPlanResult {
    pub trace: TraceContext,
    pub visible: Vec<ToolPlanEntry>,
    pub hidden: Vec<ToolPlanEntry>,
    pub conflicts: Vec<ToolConflictDiagnostic>,
    pub audit_refs: Vec<ToolAuditRef>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl ToolCatalogPlanResult {
    pub fn empty(trace: TraceContext) -> Self {
        Self {
            trace,
            visible: Vec::new(),
            hidden: Vec::new(),
            conflicts: Vec::new(),
            audit_refs: Vec::new(),
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolServiceSnapshotResult {
    pub trace: TraceContext,
    pub health: ServiceHealth,
    pub plan: ToolPlan,
    pub provider_count: usize,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl ToolServiceSnapshotResult {
    pub fn unavailable(trace: TraceContext, reason: impl Into<String>) -> Self {
        Self {
            trace,
            health: ServiceHealth::Unavailable {
                reason: reason.into(),
            },
            plan: ToolPlan {
                visible: Vec::new(),
                hidden: Vec::new(),
                conflicts: Vec::new(),
            },
            provider_count: 0,
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolServiceHealthResult {
    pub trace: TraceContext,
    pub health: ServiceHealth,
    pub provider_count: usize,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolGenericTraceCommand {
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl ToolGenericTraceCommand {
    pub fn new(trace: TraceContext) -> MacacaResult<Self> {
        validate_trace(&trace)?;
        Ok(Self {
            trace,
            metadata: BTreeMap::new(),
        })
    }
}

pub type ToolCatalogSnapshotCommand = ToolGenericTraceCommand;
pub type ToolToolsetResolveCommand = ToolGenericTraceCommand;
pub type ToolProviderStatusCommand = ToolGenericTraceCommand;
pub type ToolProviderHealthCommand = ToolGenericTraceCommand;
pub type ToolPolicyExplainCommand = ToolGenericTraceCommand;
pub type ToolAuditQueryCommand = ToolGenericTraceCommand;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvokeCommand {
    pub trace: TraceContext,
    pub scope: CapabilityToolInvocationScope,
    pub tool_id: String,
    pub descriptor: Option<IndustrialToolDescriptor>,
    pub input: Value,
    pub policy_ref: Option<ToolPolicyRef>,
    pub approval_ref: Option<ToolApprovalRef>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocationControlCommand {
    pub trace: TraceContext,
    pub invocation_ref: ToolInvocationRef,
    pub metadata: BTreeMap<String, String>,
}

pub type ToolInvokeCancelCommand = ToolInvocationControlCommand;
pub type ToolInvocationStatusCommand = ToolInvocationControlCommand;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultGetCommand {
    pub trace: TraceContext,
    pub invocation_ref: ToolInvocationRef,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolArtifactOpenCommand {
    pub trace: TraceContext,
    pub artifact_ref: ToolArtifactRef,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCommandResult {
    pub trace: TraceContext,
    pub status: String,
    pub result_class: ToolResultClass,
    pub inline_output: Option<Value>,
    pub artifact_refs: Vec<ToolArtifactRef>,
    pub invocation_ref: Option<ToolInvocationRef>,
    pub audit_refs: Vec<ToolAuditRef>,
    pub error_summary: Option<String>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

pub type ToolCatalogSnapshotResult = ToolServiceSnapshotResult;
pub type ToolToolsetResolveResult = ToolCatalogPlanResult;
pub type ToolInvokeResult = ToolCommandResult;
pub type ToolInvokeCancelResult = ToolCommandResult;
pub type ToolInvocationStatusResult = ToolCommandResult;
pub type ToolResultGetResult = ToolCommandResult;
pub type ToolArtifactOpenResult = ToolCommandResult;
pub type ToolProviderStatusResult = ToolServiceSnapshotResult;
pub type ToolProviderHealthResult = ToolServiceHealthResult;
pub type ToolPolicyExplainResult = ToolCommandResult;
pub type ToolAuditQueryResult = ToolCommandResult;

pub fn tool_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(TOOL_SERVICE_ID),
        ServiceType::new("tool.capability"),
        TraceSchemaRef::new("trace.service.tool.v1"),
    );
    descriptor.capabilities.push(ServiceCapability::new(
        CapabilityId::new("capability.tool.capability_plane"),
        "Provider-neutral industrial tool capability contracts",
    ));
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor
}

fn validate_trace(trace: &TraceContext) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "tool service command requires trace_id".into(),
        ));
    }
    Ok(())
}

pub(crate) fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
