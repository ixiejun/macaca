//! Industrial tool descriptor, availability, routing, and plan vocabulary.
//!
//! **Pattern:** Value Object cluster — groups stable catalog semantics separately from command
//! envelopes so tool families can evolve descriptor fields without touching invoke/audit DTOs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CapabilityToolDescriptor, MacacaResult};

use super::non_empty;

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

/// Application-neutral execution route selected by the Tool Capability Plane.
///
/// This route kind is the critical ownership boundary for industrial tools. It
/// tells `service.tool` how to dispatch after policy admission without forcing
/// non-MCP families through MCP semantics and without branching on concrete
/// application, provider, or product names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutorRouteKind {
    Driver,
    Skill,
    Mcp,
    OwningServiceCommand,
    RuntimeEnvironment,
    ManagedGateway,
    Plugin,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutorRoute {
    pub route_kind: ToolExecutorRouteKind,
    pub service_id: String,
    pub provider_id: String,
    pub capability_id: String,
    pub command_name: Option<String>,
}

impl ToolExecutorRoute {
    fn from_descriptor(base: &CapabilityToolDescriptor) -> Self {
        let route_kind = match base.origin_kind {
            crate::CapabilityToolOriginKind::Driver => ToolExecutorRouteKind::Driver,
            crate::CapabilityToolOriginKind::Skill => ToolExecutorRouteKind::Skill,
            crate::CapabilityToolOriginKind::Mcp => ToolExecutorRouteKind::Mcp,
        };
        Self {
            route_kind,
            service_id: base.service_id.clone(),
            provider_id: base.provider_id.clone(),
            capability_id: base.capability_id.clone(),
            command_name: None,
        }
    }

    /// Override the route kind and optional command after descriptor creation.
    ///
    /// Family contributors use this Builder-style method to keep base
    /// descriptor continuity while carrying industrial route metadata for
    /// runtime environments, gateways, plugins, owning-service commands, and
    /// unavailable providers.
    pub fn with_route_kind(
        mut self,
        route_kind: ToolExecutorRouteKind,
        command_name: Option<String>,
    ) -> Self {
        self.route_kind = route_kind;
        self.command_name = command_name;
        self
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
