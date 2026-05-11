//! Protocol-level Plugin SDK / ABI / Host contracts.
//!
//! These DTOs are the stable data boundary between plugin authors, SDK helpers,
//! runtime-host factories, and future execution hosts. The module deliberately
//! stores ABI and host information as provider-neutral protocol data. It does
//! not load WASM, spawn processes, open sockets, read plugin package bytes, or
//! expose kernel/runtime-host internals to plugin code.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{PluginHealth, PluginId, PluginRuntimeKind, PluginVersion, TraceContext};

/// Canonical Plugin ABI version used by SDK/host skeleton v1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PluginAbiVersion(String);

impl PluginAbiVersion {
    /// Create a trimmed ABI version identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw ABI version string used in manifests and diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PluginAbiVersion {
    fn default() -> Self {
        Self::new("plugin_abi_v1")
    }
}

impl fmt::Display for PluginAbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Declares how a runtime-host should locate an execution entry point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHostEntryPoint {
    pub kind: String,
    pub value: String,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHostEntryPoint {
    /// Build an entry point descriptor with no provider-specific metadata.
    pub fn new(kind: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind: kind.into().trim().to_string(),
            value: value.into().trim().to_string(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Bounded timeout policy attached to host lifecycle and invocation commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHostTimeoutPolicy {
    Default,
    Millis(u64),
    Disabled,
}

impl PluginHostTimeoutPolicy {
    /// Convert a timeout policy into a concrete bounded millisecond value.
    pub fn as_millis(&self, default_millis: u64) -> Option<u64> {
        match self {
            Self::Default => Some(default_millis),
            Self::Millis(value) => Some(*value),
            Self::Disabled => None,
        }
    }
}

/// Resource lease requested for one host operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHostResourceLease {
    pub resource_id: String,
    pub kind: String,
    pub required: bool,
    pub quantity: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHostResourceLease {
    /// Build a resource lease declaration for host admission and diagnostics.
    pub fn new(resource_id: impl Into<String>, kind: impl Into<String>, required: bool) -> Self {
        Self {
            resource_id: resource_id.into().trim().to_string(),
            kind: kind.into().trim().to_string(),
            required,
            quantity: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Host metadata derived from a plugin manifest and SDK registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHostDescriptor {
    pub plugin_id: PluginId,
    pub plugin_version: PluginVersion,
    pub runtime_kind: PluginRuntimeKind,
    pub abi_version: PluginAbiVersion,
    pub entry_point: Option<PluginHostEntryPoint>,
    pub timeout_policy: PluginHostTimeoutPolicy,
    pub resource_leases: Vec<PluginHostResourceLease>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHostDescriptor {
    /// Build host metadata with conservative defaults.
    pub fn new(
        plugin_id: PluginId,
        plugin_version: PluginVersion,
        runtime_kind: PluginRuntimeKind,
        abi_version: PluginAbiVersion,
    ) -> Self {
        Self {
            plugin_id,
            plugin_version,
            runtime_kind,
            abi_version,
            entry_point: None,
            timeout_policy: PluginHostTimeoutPolicy::Default,
            resource_leases: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Lifecycle and invocation operations understood by host skeletons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHostOperation {
    Select,
    Prepare,
    Start,
    Call,
    InvokeHook,
    HealthProbe,
    Stop,
    Cleanup,
}

impl fmt::Display for PluginHostOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Select => f.write_str("select"),
            Self::Prepare => f.write_str("prepare"),
            Self::Start => f.write_str("start"),
            Self::Call => f.write_str("call"),
            Self::InvokeHook => f.write_str("invoke_hook"),
            Self::HealthProbe => f.write_str("health_probe"),
            Self::Stop => f.write_str("stop"),
            Self::Cleanup => f.write_str("cleanup"),
        }
    }
}

/// Typed command sent to a plugin host skeleton.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHostCommand {
    pub descriptor: PluginHostDescriptor,
    pub operation: PluginHostOperation,
    pub payload: serde_json::Value,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHostCommand {
    /// Build a host command with an empty metadata map.
    pub fn new(
        descriptor: PluginHostDescriptor,
        operation: PluginHostOperation,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            descriptor,
            operation,
            payload,
            trace,
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured status produced by host skeletons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHostStatus {
    Accepted,
    Unavailable,
    Denied,
    Timeout,
    Error,
}

/// Host operation result returned across runtime boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHostResult {
    pub plugin_id: PluginId,
    pub runtime_kind: PluginRuntimeKind,
    pub operation: PluginHostOperation,
    pub status: PluginHostStatus,
    pub output: serde_json::Value,
    pub error_code: Option<String>,
    pub message: String,
    pub trace: TraceContext,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHostResult {
    /// Build a successful result for descriptor and built-in adapter hosts.
    pub fn accepted(command: &PluginHostCommand, message: impl Into<String>) -> Self {
        Self::new(command, PluginHostStatus::Accepted, message, None)
    }

    /// Build a structured unavailable result for unimplemented execution hosts.
    pub fn unavailable(command: &PluginHostCommand, message: impl Into<String>) -> Self {
        Self::new(
            command,
            PluginHostStatus::Unavailable,
            message,
            Some("plugin_host_unavailable".into()),
        )
    }

    fn new(
        command: &PluginHostCommand,
        status: PluginHostStatus,
        message: impl Into<String>,
        error_code: Option<String>,
    ) -> Self {
        Self {
            plugin_id: command.descriptor.plugin_id.clone(),
            runtime_kind: command.descriptor.runtime_kind.clone(),
            operation: command.operation.clone(),
            status,
            output: serde_json::json!({}),
            error_code,
            message: message.into(),
            trace: command.trace.clone(),
            timestamp: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Health snapshot returned by plugin host probes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHostHealthSnapshot {
    pub plugin_id: PluginId,
    pub runtime_kind: PluginRuntimeKind,
    pub health: PluginHealth,
    pub checked_at: DateTime<Utc>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}
