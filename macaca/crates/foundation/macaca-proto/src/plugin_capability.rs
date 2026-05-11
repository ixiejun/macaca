//! Protocol-level Plugin Capability Registry v1 contracts.
//!
//! These DTOs describe plugin-owned capabilities as data.  They deliberately
//! avoid provider, application, workflow, model, driver, gateway, chain, or
//! business-specific names so Macaca can discover, conflict-check, route, and
//! audit plugin capabilities without loading plugin code.  Runtime-host owns
//! registry orchestration; concrete services own capability behavior.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CapabilityId, KernelServiceId, PluginId, ServiceScope, TraceContext, TraceSchemaRef};

/// Stable service id used for the Plugin Capability Registry service.
pub const PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID: &str = "macaca.plugin.capability_registry";

/// Command names accepted by the Plugin Capability Registry service.
pub const PLUGIN_CAPABILITY_DISCOVER_COMMAND: &str = "plugin.capability.discover";
pub const PLUGIN_CAPABILITY_REGISTER_COMMAND: &str = "plugin.capability.register";
pub const PLUGIN_CAPABILITY_DEACTIVATE_COMMAND: &str = "plugin.capability.deactivate";
pub const PLUGIN_CAPABILITY_QUERY_COMMAND: &str = "plugin.capability.query";
pub const PLUGIN_CAPABILITY_INSPECT_COMMAND: &str = "plugin.capability.inspect";
pub const PLUGIN_CAPABILITY_CALL_COMMAND: &str = "plugin.capability.call";

/// Return the canonical service id as a typed kernel service id.
pub fn plugin_capability_registry_service_id() -> KernelServiceId {
    KernelServiceId::new(PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID)
}

/// Provider-neutral capability category exposed by plugins and built-ins.
///
/// The enum is open through `Custom` because Macaca is an OS-style platform:
/// third-party packages must be able to introduce new extension categories
/// without requiring a core source edit.  Closed variants are only the common
/// built-in categories that need first-class conflict policy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapabilityKind {
    Tool,
    Hook,
    Driver,
    Gateway,
    Skill,
    Mcp,
    Memory,
    Context,
    LlmProvider,
    Observability,
    HttpRoute,
    CliCommand,
    Custom(String),
}

impl fmt::Display for PluginCapabilityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tool => f.write_str("tool"),
            Self::Hook => f.write_str("hook"),
            Self::Driver => f.write_str("driver"),
            Self::Gateway => f.write_str("gateway"),
            Self::Skill => f.write_str("skill"),
            Self::Mcp => f.write_str("mcp"),
            Self::Memory => f.write_str("memory"),
            Self::Context => f.write_str("context"),
            Self::LlmProvider => f.write_str("llm_provider"),
            Self::Observability => f.write_str("observability"),
            Self::HttpRoute => f.write_str("http_route"),
            Self::CliCommand => f.write_str("cli_command"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Visibility policy attached to a capability descriptor.
///
/// Visibility is descriptive only.  Admission and entitlement strategies decide
/// whether a caller may see or invoke a descriptor in a concrete deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapabilityVisibility {
    Public,
    Workspace,
    Application,
    Session,
    Private,
}

/// Provider-neutral JSON schema reference or inline schema.
///
/// The registry stores schema metadata for contract-first discovery, but it
/// does not interpret business fields.  Concrete services or future SDK test
/// kits can validate payloads against these schema hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilitySchema {
    pub name: String,
    pub schema_ref: Option<String>,
    pub inline_schema: Option<serde_json::Value>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginCapabilitySchema {
    /// Build a schema descriptor from a stable name and optional reference.
    pub fn referenced(name: impl Into<String>, schema_ref: impl Into<String>) -> Self {
        Self {
            name: name.into().trim().to_string(),
            schema_ref: Some(schema_ref.into().trim().to_string()),
            inline_schema: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Permission hint required to discover or invoke a capability.
///
/// This is a hint rather than a decision.  The service runtime, entitlement,
/// and policy layers evaluate the actual request using deployment-specific
/// strategies while preserving the same descriptor shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityPermissionHint {
    pub permission: String,
    pub required: bool,
    pub reason: String,
    pub metadata: BTreeMap<String, String>,
}

/// Resource hint required by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityResourceHint {
    pub resource: String,
    pub kind: String,
    pub required: bool,
    pub reason: String,
    pub metadata: BTreeMap<String, String>,
}

/// Slot metadata used by conflict policies.
///
/// Slot namespace and key are the registry-level contract for exclusive or
/// shared ownership.  Examples include tool names, CLI commands, HTTP routes,
/// gateway routes, and default provider slots.  The registry never hardcodes a
/// concrete application name or provider identity inside these fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PluginCapabilitySlot {
    pub namespace: String,
    pub key: String,
    pub exclusive: bool,
    pub metadata: BTreeMap<String, String>,
}

impl PluginCapabilitySlot {
    /// Build a slot used for conflict detection.
    pub fn new(namespace: impl Into<String>, key: impl Into<String>, exclusive: bool) -> Self {
        Self {
            namespace: namespace.into().trim().to_string(),
            key: key.into().trim().to_string(),
            exclusive,
            metadata: BTreeMap::new(),
        }
    }
}

/// Provider-neutral plugin capability descriptor.
///
/// This is the core data contract for Plugin Capability Registry v1.  It can
/// be loaded from manifests, built-in adapters, repository snapshots, or future
/// SDK registration fixtures without starting plugin runtime code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityDescriptor {
    pub capability_id: CapabilityId,
    pub plugin_id: PluginId,
    pub kind: PluginCapabilityKind,
    pub service: Option<KernelServiceId>,
    pub name: String,
    pub description: String,
    pub visibility: PluginCapabilityVisibility,
    pub scopes: Vec<ServiceScope>,
    pub input_schema: Option<PluginCapabilitySchema>,
    pub output_schema: Option<PluginCapabilitySchema>,
    pub permission_hints: Vec<PluginCapabilityPermissionHint>,
    pub resource_hints: Vec<PluginCapabilityResourceHint>,
    pub trace_schema: TraceSchemaRef,
    pub slots: Vec<PluginCapabilitySlot>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginCapabilityDescriptor {
    /// Build a minimal descriptor with provider-neutral defaults.
    pub fn new(
        capability_id: CapabilityId,
        plugin_id: PluginId,
        kind: PluginCapabilityKind,
        name: impl Into<String>,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            capability_id,
            plugin_id,
            kind,
            service: None,
            name: name.into().trim().to_string(),
            description: String::new(),
            visibility: PluginCapabilityVisibility::Workspace,
            scopes: vec![ServiceScope::Global],
            input_schema: None,
            output_schema: None,
            permission_hints: Vec::new(),
            resource_hints: Vec::new(),
            trace_schema,
            slots: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Active ownership record stored by the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityOwnership {
    pub capability_id: CapabilityId,
    pub plugin_id: PluginId,
    pub kind: PluginCapabilityKind,
    pub service: Option<KernelServiceId>,
    pub active: bool,
    pub registered_at: DateTime<Utc>,
    pub descriptor: PluginCapabilityDescriptor,
}

/// Conflict report returned by registry Strategy implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityConflictReport {
    pub namespace: String,
    pub key: String,
    pub rejected_plugin_id: PluginId,
    pub rejected_capability_id: CapabilityId,
    pub existing_plugin_id: PluginId,
    pub existing_capability_id: CapabilityId,
    pub reason: String,
}

/// Deterministic registry snapshot used for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityRegistrySnapshot {
    pub generated_at: DateTime<Utc>,
    pub capabilities: Vec<PluginCapabilityOwnership>,
    pub conflicts: Vec<PluginCapabilityConflictReport>,
}

/// Command for discovering descriptors from manifest or repository data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityDiscoverCommand {
    pub plugin_id: Option<PluginId>,
    pub include_inactive: bool,
    pub trace: TraceContext,
}

/// Command for registering one plugin's descriptor set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityRegisterCommand {
    pub plugin_id: PluginId,
    pub descriptors: Vec<PluginCapabilityDescriptor>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Command for deactivating all capabilities owned by one plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityDeactivateCommand {
    pub plugin_id: PluginId,
    pub trace: TraceContext,
    pub reason: String,
}

/// Query command for deterministic capability lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityQueryCommand {
    pub kind: Option<PluginCapabilityKind>,
    pub plugin_id: Option<PluginId>,
    pub slot_namespace: Option<String>,
    pub slot_key: Option<String>,
    pub include_inactive: bool,
    pub trace: TraceContext,
}

/// Inspect command for a single capability id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityInspectCommand {
    pub capability_id: CapabilityId,
    pub trace: TraceContext,
}

/// Provider-neutral call envelope for future plugin execution hosts.
///
/// The envelope requires trace context at the protocol layer.  Runtime-host
/// still validates trace before dispatch so malformed service commands are
/// rejected before reaching any implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityCallCommand {
    pub capability_id: CapabilityId,
    pub plugin_id: Option<PluginId>,
    pub input: serde_json::Value,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Structured result for capability call skeletons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityCallResult {
    pub capability_id: CapabilityId,
    pub plugin_id: PluginId,
    pub status: String,
    pub output: serde_json::Value,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Audit event emitted by registry and call attempts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilityEvent {
    pub plugin_id: Option<PluginId>,
    pub capability_id: Option<CapabilityId>,
    pub kind: Option<PluginCapabilityKind>,
    pub operation: String,
    pub decision: String,
    pub trace: TraceContext,
    pub timestamp: DateTime<Utc>,
    pub error_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginCapabilityEvent {
    /// Build an audit-safe event without embedding capability inputs, secrets,
    /// or provider configuration values.
    pub fn new(
        plugin_id: Option<PluginId>,
        capability_id: Option<CapabilityId>,
        kind: Option<PluginCapabilityKind>,
        operation: impl Into<String>,
        decision: impl Into<String>,
        trace: TraceContext,
    ) -> Self {
        Self {
            plugin_id,
            capability_id,
            kind,
            operation: operation.into(),
            decision: decision.into(),
            trace,
            timestamp: Utc::now(),
            error_code: None,
            metadata: BTreeMap::new(),
        }
    }
}
