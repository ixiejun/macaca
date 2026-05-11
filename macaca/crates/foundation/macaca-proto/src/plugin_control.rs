//! Protocol-level Plugin Control Plane v1 contracts.
//!
//! This module contains data-only DTOs for managing plugins as OS resources.
//! The contracts deliberately avoid marketplace, package-manager, execution
//! host, or provider-specific assumptions.  Runtime-host owns orchestration and
//! shells talk through SDK/service calls, so Web and CLI never need to inspect
//! plugin directories or execute plugin code directly.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    KernelServiceId, PluginHealth, PluginId, PluginLifecycleState, PluginManifest, PluginVersion,
    TraceContext,
};

/// Stable service id used by SDK clients and shells to reach Plugin Control.
///
/// The id is a provider-neutral OS service address.  It names the capability,
/// not a concrete implementation, so tests, local runtime-host, and future
/// remote service runtimes can expose the same contract.
pub const PLUGIN_CONTROL_SERVICE_ID: &str = "macaca.plugin.control";

/// Command names accepted by Plugin Control Service.
///
/// These constants keep SDK, Web, CLI, and runtime-host dispatch aligned while
/// still using the generic Route C service-call transport.
pub const PLUGIN_CONTROL_LIST_COMMAND: &str = "plugin.list";
pub const PLUGIN_CONTROL_INSPECT_COMMAND: &str = "plugin.inspect";
pub const PLUGIN_CONTROL_INSTALL_COMMAND: &str = "plugin.install";
pub const PLUGIN_CONTROL_ENABLE_COMMAND: &str = "plugin.enable";
pub const PLUGIN_CONTROL_DISABLE_COMMAND: &str = "plugin.disable";
pub const PLUGIN_CONTROL_START_COMMAND: &str = "plugin.start";
pub const PLUGIN_CONTROL_STOP_COMMAND: &str = "plugin.stop";
pub const PLUGIN_CONTROL_UNINSTALL_COMMAND: &str = "plugin.uninstall";
pub const PLUGIN_CONTROL_HEALTH_COMMAND: &str = "plugin.health";
pub const PLUGIN_CONTROL_DIAGNOSTICS_COMMAND: &str = "plugin.diagnostics";

/// Repository category used to group installed and discoverable plugins.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginRepositoryKind {
    Bundled,
    UserLocal,
    ProjectLocal,
    DevLink,
    Archive,
    StoreCache,
    Git,
    Custom(String),
}

/// Install-source kind selected by a control-plane install command.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginInstallSourceKind {
    Bundled,
    UserLocal,
    ProjectLocal,
    DevLink,
    Archive,
    StoreCache,
    Git,
    Custom(String),
}

/// Provider-neutral package location.
///
/// `uri` can represent a local path, package cache key, store package id, or
/// future git URL.  Runtime-host strategies validate source-specific safety
/// rules before they read any bytes from this location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPackageLocation {
    pub source_kind: PluginInstallSourceKind,
    pub uri: String,
    pub metadata: BTreeMap<String, String>,
}

impl PluginPackageLocation {
    /// Build a package location with no provider-specific metadata.
    pub fn new(source_kind: PluginInstallSourceKind, uri: impl Into<String>) -> Self {
        Self {
            source_kind,
            uri: uri.into().trim().to_string(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Repository descriptor returned by diagnostics and shell views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRepositoryDescriptor {
    pub repository_id: String,
    pub kind: PluginRepositoryKind,
    pub enabled: bool,
    pub writable: bool,
    pub root_hint: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Declarative config requirement exposed by a plugin manifest or package.
///
/// The control plane reports keys and status only.  It never returns raw config
/// values because diagnostics are often rendered in Web/CLI and audit logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigRequirement {
    pub key: String,
    pub required: bool,
    pub present: bool,
    pub description: String,
}

/// Secret or environment requirement exposed without leaking values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSecretRequirement {
    pub name: String,
    pub required: bool,
    pub present: bool,
    pub source_hint: Option<String>,
}

/// Activation state tracked by the control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginActivationState {
    Disabled,
    Enabled,
    Starting,
    Running,
    Stopped,
    Failed,
    Uninstalled,
}

impl PluginActivationState {
    /// Convert kernel lifecycle state into control-plane activation state.
    pub fn from_lifecycle(state: &PluginLifecycleState) -> Self {
        match state {
            PluginLifecycleState::Installed | PluginLifecycleState::Registered => Self::Enabled,
            PluginLifecycleState::Starting => Self::Starting,
            PluginLifecycleState::Running => Self::Running,
            PluginLifecycleState::Stopping | PluginLifecycleState::Stopped => Self::Stopped,
            PluginLifecycleState::Failed => Self::Failed,
            PluginLifecycleState::Uninstalled => Self::Uninstalled,
        }
    }
}

/// Read model for one plugin managed by the control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginControlRecord {
    pub plugin_id: PluginId,
    pub version: PluginVersion,
    pub repository_id: String,
    pub source_kind: PluginInstallSourceKind,
    pub activation_state: PluginActivationState,
    pub lifecycle_state: PluginLifecycleState,
    pub health: PluginHealth,
    pub enabled: bool,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub manifest: PluginManifest,
    pub config_requirements: Vec<PluginConfigRequirement>,
    pub secret_requirements: Vec<PluginSecretRequirement>,
    pub metadata: BTreeMap<String, String>,
}

/// Install request accepted by the runtime-host control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInstallRequest {
    pub location: PluginPackageLocation,
    pub manifest: Option<PluginManifest>,
    pub enable_after_install: bool,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Generic target command for plugin lifecycle and inspection operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginTargetCommand {
    pub plugin_id: PluginId,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl PluginTargetCommand {
    /// Build a traced command against one plugin id.
    pub fn new(plugin_id: PluginId, trace: TraceContext) -> Self {
        Self {
            plugin_id,
            trace,
            metadata: BTreeMap::new(),
        }
    }
}

/// List command for repository-filtered control-plane views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginListCommand {
    pub repository_id: Option<String>,
    pub include_disabled: bool,
    pub trace: TraceContext,
}

/// Install result containing the sanitized installed record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInstallResult {
    pub record: PluginControlRecord,
    pub events: Vec<PluginControlEvent>,
}

/// Health snapshot used by Web/CLI and tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHealthSnapshot {
    pub plugin_id: PluginId,
    pub health: PluginHealth,
    pub activation_state: PluginActivationState,
    pub checked_at: DateTime<Utc>,
    pub reason: Option<String>,
}

/// Deterministic diagnostics bundle for one plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDiagnostics {
    pub plugin_id: PluginId,
    pub repository_id: String,
    pub source_kind: PluginInstallSourceKind,
    pub activation_state: PluginActivationState,
    pub lifecycle_state: PluginLifecycleState,
    pub health: PluginHealth,
    pub config_requirements: Vec<PluginConfigRequirement>,
    pub secret_requirements: Vec<PluginSecretRequirement>,
    pub generated_at: DateTime<Utc>,
    pub warnings: Vec<String>,
}

/// Control-plane operation event used for trace and audit sinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginControlEvent {
    pub plugin_id: Option<PluginId>,
    pub operation: String,
    pub status: String,
    pub activation_state: Option<PluginActivationState>,
    pub trace: TraceContext,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginControlEvent {
    /// Build an event without embedding config, secret, or package payloads.
    pub fn new(
        plugin_id: Option<PluginId>,
        operation: impl Into<String>,
        status: impl Into<String>,
        activation_state: Option<PluginActivationState>,
        trace: TraceContext,
    ) -> Self {
        Self {
            plugin_id,
            operation: operation.into(),
            status: status.into(),
            activation_state,
            trace,
            timestamp: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Typed response envelope used by SDK clients that call through ServiceRuntime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum PluginControlResponse {
    List(Vec<PluginControlRecord>),
    Inspect(Option<PluginControlRecord>),
    Install(PluginInstallResult),
    State(PluginControlRecord),
    Health(PluginHealthSnapshot),
    Diagnostics(PluginDiagnostics),
}

/// Return the service id as a value object for service-runtime registration.
pub fn plugin_control_service_id() -> KernelServiceId {
    KernelServiceId::new(PLUGIN_CONTROL_SERVICE_ID)
}

#[cfg(test)]
#[path = "plugin_control_tests.rs"]
mod plugin_control_tests;
