//! Provider-neutral industrial Tool Capability Plane contracts.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ApplicationId, CapabilityId, CapabilityToolDescriptor, CapabilityToolInvocationScope,
    KernelServiceId, MacacaError, MacacaResult, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolFamilyRef(String);

impl ToolFamilyRef {
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(value.into(), "tool family requires value")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolsetRef(String);

impl ToolsetRef {
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(value.into(), "toolset requires value")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AvailabilityExpression {
    Config { key: String },
    Secret { key_ref: String },
    Auth { provider_ref: String },
    Env { key: String },
    Binary { binary: String },
    ServiceHealth { service_id: String },
    Platform { os: String },
    Resource { resource: String },
    Entitlement { entitlement: String },
    Plugin { plugin_id: String },
    Manifest { capability: String },
    AgentPolicy { policy_ref: String },
    SessionContext { key: String },
    All { items: Vec<AvailabilityExpression> },
    Any { items: Vec<AvailabilityExpression> },
    Not { item: Box<AvailabilityExpression> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSideEffectClass {
    ReadOnly,
    Write,
    Network,
    Process,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultClass {
    Text,
    StructuredJson,
    BinaryArtifact,
    Multimodal,
    BackgroundHandle,
    ApprovalRequest,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolArtifactPolicy {
    InlineOnly,
    PersistOversized,
    AlwaysPersist,
    NeverPersist,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolTrustLevel {
    BuiltIn,
    Plugin,
    Remote,
    Gateway,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolLifecycleScope {
    Global,
    Application,
    Session,
    AgentSession,
    Call,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutorRoute {
    pub service_id: String,
    pub provider_id: String,
    pub capability_id: String,
    pub command_name: Option<String>,
}

impl ToolExecutorRoute {
    fn from_descriptor(base: &CapabilityToolDescriptor) -> Self {
        Self {
            service_id: base.service_id.clone(),
            provider_id: base.provider_id.clone(),
            capability_id: base.capability_id.clone(),
            command_name: None,
        }
    }
}

/// Descriptor snapshot; owning services still control lifecycle and invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialToolDescriptor {
    pub stable_tool_id: String,
    pub visible_name: String,
    pub title: String,
    pub family: ToolFamilyRef,
    pub toolsets: Vec<ToolsetRef>,
    pub base_descriptor: CapabilityToolDescriptor,
    pub executor_route: ToolExecutorRoute,
    pub lifecycle_scope: ToolLifecycleScope,
    pub side_effect_class: ToolSideEffectClass,
    pub approval_profile: Option<String>,
    pub result_budget_profile: Option<String>,
    pub result_class: ToolResultClass,
    pub artifact_policy: ToolArtifactPolicy,
    pub trust_level: ToolTrustLevel,
    pub availability: Vec<AvailabilityExpression>,
    pub telemetry_labels: BTreeMap<String, String>,
    pub sanitized_metadata: BTreeMap<String, String>,
}

impl IndustrialToolDescriptor {
    pub fn new(
        stable_tool_id: impl Into<String>,
        visible_name: impl Into<String>,
        title: impl Into<String>,
        family: ToolFamilyRef,
        base_descriptor: CapabilityToolDescriptor,
    ) -> MacacaResult<Self> {
        Ok(Self {
            stable_tool_id: non_empty(stable_tool_id.into(), "tool requires stable_tool_id")?,
            visible_name: non_empty(visible_name.into(), "tool requires visible_name")?,
            title: non_empty(title.into(), "tool requires title")?,
            family,
            toolsets: Vec::new(),
            executor_route: ToolExecutorRoute::from_descriptor(&base_descriptor),
            base_descriptor,
            lifecycle_scope: ToolLifecycleScope::Call,
            side_effect_class: ToolSideEffectClass::ReadOnly,
            approval_profile: None,
            result_budget_profile: None,
            result_class: ToolResultClass::Text,
            artifact_policy: ToolArtifactPolicy::PersistOversized,
            trust_level: ToolTrustLevel::Unavailable,
            availability: Vec::new(),
            telemetry_labels: BTreeMap::new(),
            sanitized_metadata: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolHiddenReason {
    MissingConfig,
    MissingSecret,
    MissingAuth,
    MissingBinary,
    ServiceUnavailable,
    PolicyDenied,
    ResourceUnavailable,
    EntitlementMissing,
    PluginUnavailable,
    ManifestDenied,
    Conflict,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPlanEntry {
    pub descriptor: IndustrialToolDescriptor,
    pub hidden_reason: Option<ToolHiddenReason>,
    pub reason_refs: Vec<String>,
    pub selection_reasons: Vec<String>,
}

impl ToolPlanEntry {
    pub fn visible(descriptor: IndustrialToolDescriptor, selection_reasons: Vec<String>) -> Self {
        Self {
            descriptor,
            hidden_reason: None,
            reason_refs: Vec::new(),
            selection_reasons,
        }
    }

    pub fn hidden(
        descriptor: IndustrialToolDescriptor,
        hidden_reason: ToolHiddenReason,
        reason_refs: Vec<String>,
    ) -> Self {
        Self {
            descriptor,
            hidden_reason: Some(hidden_reason),
            reason_refs,
            selection_reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HiddenToolPlanEntry {
    pub entry: ToolPlanEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolConflictDiagnostic {
    pub namespace: String,
    pub tool_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPlan {
    pub visible: Vec<ToolPlanEntry>,
    pub hidden: Vec<ToolPlanEntry>,
    pub conflicts: Vec<ToolConflictDiagnostic>,
}

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

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
