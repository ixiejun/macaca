//! Protocol-level Plugin Runtime v0 contracts.
//!
//! Plugin Runtime v0 is intentionally manifest-first.  These types describe a
//! plugin, the services it provides, the permissions/resources it needs, and
//! the lifecycle events the OS must audit.  They do not execute plugin code,
//! store credentials, or bind Macaca to any concrete gateway, driver, memory,
//! skill, MCP, payment, chain, or application implementation.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CapabilityId, DeveloperId, KernelServiceId, ServiceDescriptor, TraceContext};

/// Stable plugin identity.
///
/// The value is string-backed so local development ids, Store ids, reverse-DNS
/// ids, and future decentralized ids can share the same protocol contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PluginId(String);

impl PluginId {
    /// Create a plugin id after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw plugin id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable semantic or Store-assigned plugin version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PluginVersion(String);

impl PluginVersion {
    /// Create a plugin version after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw version string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Runtime family requested by a plugin manifest.
///
/// `DescriptorOnly` and `BuiltInAdapter` are the only executable-safe Phase 07
/// paths.  Future execution runtimes are represented as structured data so the
/// runtime-host can reject them without losing manifest information.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeKind {
    DescriptorOnly,
    BuiltInAdapter,
    Wasm,
    Native,
    Process,
    RemoteProxy,
    Custom(String),
}

impl PluginRuntimeKind {
    /// Return true when Phase 07 can activate this runtime without launching
    /// arbitrary third-party code.
    pub fn is_supported_v0(&self) -> bool {
        matches!(self, Self::DescriptorOnly | Self::BuiltInAdapter)
    }
}

impl fmt::Display for PluginRuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptorOnly => f.write_str("descriptor_only"),
            Self::BuiltInAdapter => f.write_str("built_in_adapter"),
            Self::Wasm => f.write_str("wasm"),
            Self::Native => f.write_str("native"),
            Self::Process => f.write_str("process"),
            Self::RemoteProxy => f.write_str("remote_proxy"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Runtime declaration attached to a plugin manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRuntimeDeclaration {
    pub kind: PluginRuntimeKind,
    pub abi_version: String,
    pub metadata: BTreeMap<String, String>,
}

impl PluginRuntimeDeclaration {
    /// Build a runtime declaration for a known runtime kind.
    pub fn new(kind: PluginRuntimeKind, abi_version: impl Into<String>) -> Self {
        Self {
            kind,
            abi_version: abi_version.into(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Entry point declaration preserved for future executable plugin hosts.
///
/// Phase 07 validates and stores this metadata only.  It never shells out,
/// loads a dynamic library, starts a process, or executes WASM based on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginEntryDeclaration {
    pub kind: String,
    pub value: String,
    pub metadata: BTreeMap<String, String>,
}

/// Extensible plugin-provided service category.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginServiceCategory {
    Gateway,
    Driver,
    Memory,
    Context,
    Skill,
    Mcp,
    Payment,
    Compliance,
    Custom(String),
}

impl fmt::Display for PluginServiceCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gateway => f.write_str("gateway"),
            Self::Driver => f.write_str("driver"),
            Self::Memory => f.write_str("memory"),
            Self::Context => f.write_str("context"),
            Self::Skill => f.write_str("skill"),
            Self::Mcp => f.write_str("mcp"),
            Self::Payment => f.write_str("payment"),
            Self::Compliance => f.write_str("compliance"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Service descriptor provided by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginProvidedService {
    pub service: ServiceDescriptor,
    pub category: PluginServiceCategory,
    pub required_permissions: Vec<String>,
    pub lifecycle_coupled: bool,
    pub metadata: BTreeMap<String, String>,
}

impl PluginProvidedService {
    /// Build a plugin-provided service descriptor from a system service descriptor.
    pub fn new(service: ServiceDescriptor, category: PluginServiceCategory) -> Self {
        Self {
            service,
            category,
            required_permissions: Vec::new(),
            lifecycle_coupled: true,
            metadata: BTreeMap::new(),
        }
    }
}

/// Capability descriptor provided by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginProvidedCapability {
    pub id: CapabilityId,
    pub service: Option<KernelServiceId>,
    pub category: PluginServiceCategory,
    pub description: String,
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Service dependency required by a plugin before activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRequiredService {
    pub service: KernelServiceId,
    pub capability: Option<CapabilityId>,
    pub required: bool,
    pub degraded_behavior: Option<String>,
    pub reason: String,
}

/// Permission requested by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPermission {
    pub id: String,
    pub reason: String,
    pub optional: bool,
    pub metadata: BTreeMap<String, String>,
}

/// Resource requested by a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginResource {
    pub id: String,
    pub kind: String,
    pub reason: String,
    pub optional: bool,
    pub metadata: BTreeMap<String, String>,
}

/// Signature metadata carried by plugin manifests.
///
/// Phase 07 requires metadata presence for installable plugin manifests but
/// leaves trust root and cryptographic verification to Store/entitlement phases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSignature {
    pub algorithm: String,
    pub key_id: String,
    pub digest: String,
    pub verification_status: String,
}

/// Canonical Plugin Manifest v0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: PluginId,
    pub version: PluginVersion,
    pub developer_id: DeveloperId,
    pub runtime: PluginRuntimeDeclaration,
    pub provides_services: Vec<PluginProvidedService>,
    pub provides_capabilities: Vec<PluginProvidedCapability>,
    pub requires_services: Vec<PluginRequiredService>,
    pub permissions: Vec<PluginPermission>,
    pub resources: Vec<PluginResource>,
    pub entry: Option<PluginEntryDeclaration>,
    pub signature: Option<PluginSignature>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginManifest {
    /// Build a minimal descriptor-only manifest for tests and built-in adapters.
    pub fn new(
        plugin_id: PluginId,
        version: PluginVersion,
        developer_id: DeveloperId,
        runtime: PluginRuntimeDeclaration,
    ) -> Self {
        Self {
            plugin_id,
            version,
            developer_id,
            runtime,
            provides_services: Vec::new(),
            provides_capabilities: Vec::new(),
            requires_services: Vec::new(),
            permissions: Vec::new(),
            resources: Vec::new(),
            entry: None,
            signature: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Lifecycle state owned by the plugin registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginLifecycleState {
    Installed,
    Registered,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Uninstalled,
}

/// Auditable lifecycle event emitted by Plugin Runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLifecycleEvent {
    pub plugin_id: PluginId,
    pub developer_id: DeveloperId,
    pub version: PluginVersion,
    pub runtime_kind: PluginRuntimeKind,
    pub previous_state: Option<PluginLifecycleState>,
    pub next_state: PluginLifecycleState,
    pub operation: String,
    pub service_ids: Vec<KernelServiceId>,
    pub capability_ids: Vec<CapabilityId>,
    pub status: String,
    pub error_code: Option<String>,
    pub trace: Option<TraceContext>,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginLifecycleEvent {
    /// Create a lifecycle event with the current UTC timestamp.
    pub fn new(
        manifest: &PluginManifest,
        previous_state: Option<PluginLifecycleState>,
        next_state: PluginLifecycleState,
        operation: impl Into<String>,
        status: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            plugin_id: manifest.plugin_id.clone(),
            developer_id: manifest.developer_id.clone(),
            version: manifest.version.clone(),
            runtime_kind: manifest.runtime.kind.clone(),
            previous_state,
            next_state,
            operation: operation.into(),
            service_ids: manifest
                .provides_services
                .iter()
                .map(|service| service.service.id.clone())
                .collect(),
            capability_ids: manifest
                .provides_capabilities
                .iter()
                .map(|capability| capability.id.clone())
                .collect(),
            status: status.into(),
            error_code: None,
            trace,
            timestamp: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Health reported by plugin runtime diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginHealth {
    Healthy,
    Degraded { reason: String },
    Unavailable { reason: String },
    DisabledByPolicy { reason: String },
    Unknown,
}

/// Structured errors used at the plugin runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum PluginError {
    #[error("missing plugin id")]
    MissingPluginId,

    #[error("missing plugin version")]
    MissingPluginVersion,

    #[error("missing developer id")]
    MissingDeveloperId,

    #[error("unsupported plugin runtime: {0}")]
    UnsupportedRuntime(String),

    #[error("missing plugin permission declaration: {0}")]
    MissingPermission(String),

    #[error("missing plugin resource declaration: {0}")]
    MissingResource(String),

    #[error("missing plugin signature metadata")]
    MissingSignature,

    #[error("duplicate plugin id: {0}")]
    DuplicatePlugin(String),

    #[error("invalid plugin lifecycle transition: {from:?} -> {to:?}")]
    InvalidLifecycleTransition {
        from: PluginLifecycleState,
        to: PluginLifecycleState,
    },

    #[error("plugin unavailable: {0}")]
    Unavailable(String),
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;
