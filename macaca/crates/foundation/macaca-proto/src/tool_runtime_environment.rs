//! Provider-neutral contracts for tool runtime environments and managed gateways.
//!
//! These DTOs are intentionally data-only.  Runtime-host providers, plugins, and
//! remote services can exchange them without giving the kernel, SDK, or shells
//! knowledge of concrete environment backends or gateway products.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ServiceHealth, ToolArtifactRef, ToolAuditRef, ToolFamilyRef, TraceContext};

pub const TOOL_ENVIRONMENT_HEALTH_COMMAND: &str = "tool.environment.health";
pub const TOOL_ENVIRONMENT_CLEANUP_COMMAND: &str = "tool.environment.cleanup";
pub const TOOL_GATEWAY_HEALTH_COMMAND: &str = "tool.gateway.health";
pub const TOOL_GATEWAY_METER_COMMAND: &str = "tool.gateway.meter";
pub const TOOL_GATEWAY_AUDIT_COMMAND: &str = "tool.gateway.audit";

/// Stable category for an environment provider.
///
/// `Custom` keeps future provider categories data-driven.  Callers match on
/// policy capabilities, not on vendor or application names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRuntimeEnvironmentKind {
    LocalWorkspace,
    LocalSandbox,
    Docker,
    SshRemote,
    WasmHostImport,
    BrowserSandbox,
    ManagedSandbox,
    Custom(String),
}

/// Explicit lifecycle state for environment resources.
///
/// Tool callers can distinguish absence, startup, readiness, cleanup, and
/// failure without inferring state from logs or provider-specific messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRuntimeEnvironmentState {
    Unavailable,
    Starting,
    Ready,
    Busy,
    Degraded,
    Cleaning,
    Failed,
    Stopped,
}

/// Lifetime strategy requested by a tool invocation or session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRuntimeEnvironmentScope {
    PerCall,
    Session,
    Application,
}

/// Resource guardrails applied before an environment performs side effects.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolEnvironmentResourcePolicy {
    pub cpu_millis_limit: Option<u64>,
    pub memory_bytes_limit: Option<u64>,
    pub wall_time_millis_limit: Option<u64>,
    pub process_limit: Option<u32>,
    pub metadata: BTreeMap<String, String>,
}

/// Network policy is expressed as bounded descriptors, never raw credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentNetworkPolicy {
    pub egress_allowed: bool,
    pub allowed_host_refs: Vec<String>,
    pub denied_host_refs: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ToolEnvironmentNetworkPolicy {
    pub fn denied() -> Self {
        Self {
            egress_allowed: false,
            allowed_host_refs: Vec::new(),
            denied_host_refs: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Filesystem access is represented by root references controlled by policy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ToolEnvironmentFilesystemPolicy {
    pub readable_root_refs: Vec<String>,
    pub writable_root_refs: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Secret injection policy avoids carrying secret values in descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSecretInjectionMode {
    Disabled,
    ReferenceOnly,
    ProviderInjected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentSecretPolicy {
    pub mode: ToolSecretInjectionMode,
    pub allowed_secret_refs: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl ToolEnvironmentSecretPolicy {
    pub fn disabled() -> Self {
        Self {
            mode: ToolSecretInjectionMode::Disabled,
            allowed_secret_refs: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Sanitized process handle.  The handle reference is stable audit data; raw
/// command lines, environment values, and provider payloads are intentionally
/// excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentProcessHandle {
    pub handle_ref: String,
    pub state: ToolRuntimeEnvironmentState,
    pub pid_hint: Option<u32>,
    pub metadata: BTreeMap<String, String>,
}

/// Artifact root exposed by an environment provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentArtifactRoot {
    pub root_ref: ToolArtifactRef,
    pub scope: ToolRuntimeEnvironmentScope,
    pub metadata: BTreeMap<String, String>,
}

/// Descriptor exported by a runtime environment provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRuntimeEnvironmentDescriptor {
    pub environment_id: String,
    pub provider_id: String,
    pub kind: ToolRuntimeEnvironmentKind,
    pub scope: ToolRuntimeEnvironmentScope,
    pub state: ToolRuntimeEnvironmentState,
    pub health: ServiceHealth,
    pub resource_policy: ToolEnvironmentResourcePolicy,
    pub network_policy: ToolEnvironmentNetworkPolicy,
    pub filesystem_policy: ToolEnvironmentFilesystemPolicy,
    pub secret_policy: ToolEnvironmentSecretPolicy,
    pub artifact_roots: Vec<ToolEnvironmentArtifactRoot>,
    pub process_handles: Vec<ToolEnvironmentProcessHandle>,
    pub metadata: BTreeMap<String, String>,
}

impl ToolRuntimeEnvironmentDescriptor {
    pub fn unavailable(
        environment_id: impl Into<String>,
        provider_id: impl Into<String>,
        kind: ToolRuntimeEnvironmentKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            environment_id: environment_id.into(),
            provider_id: provider_id.into(),
            kind,
            scope: ToolRuntimeEnvironmentScope::PerCall,
            state: ToolRuntimeEnvironmentState::Unavailable,
            health: ServiceHealth::Unavailable {
                reason: reason.into(),
            },
            resource_policy: ToolEnvironmentResourcePolicy::default(),
            network_policy: ToolEnvironmentNetworkPolicy::denied(),
            filesystem_policy: ToolEnvironmentFilesystemPolicy::default(),
            secret_policy: ToolEnvironmentSecretPolicy::disabled(),
            artifact_roots: Vec::new(),
            process_handles: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolEnvironmentHealthResult {
    pub trace: TraceContext,
    pub descriptors: Vec<ToolRuntimeEnvironmentDescriptor>,
    pub captured_at: DateTime<Utc>,
    pub audit_refs: Vec<ToolAuditRef>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentCleanupCommand {
    pub trace: TraceContext,
    pub environment_id: String,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnvironmentCleanupResult {
    pub trace: TraceContext,
    pub environment_id: String,
    pub status: String,
    pub released_process_handles: Vec<String>,
    pub released_artifact_roots: Vec<ToolArtifactRef>,
    pub audit_refs: Vec<ToolAuditRef>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Generic route category exposed by optional managed gateway providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolManagedGatewayRouteKind {
    Web,
    Browser,
    Media,
    Document,
    RemoteSandbox,
    EnterpriseConnector,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolManagedGatewayDescriptor {
    pub gateway_id: String,
    pub provider_id: String,
    pub route_kind: ToolManagedGatewayRouteKind,
    pub family: ToolFamilyRef,
    pub health: ServiceHealth,
    pub metering_required: bool,
    pub audit_required: bool,
    pub metadata: BTreeMap<String, String>,
}

impl ToolManagedGatewayDescriptor {
    pub fn unavailable(
        gateway_id: impl Into<String>,
        provider_id: impl Into<String>,
        route_kind: ToolManagedGatewayRouteKind,
        family: ToolFamilyRef,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            gateway_id: gateway_id.into(),
            provider_id: provider_id.into(),
            route_kind,
            family,
            health: ServiceHealth::Unavailable {
                reason: reason.into(),
            },
            metering_required: true,
            audit_required: true,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolGatewayHealthResult {
    pub trace: TraceContext,
    pub gateways: Vec<ToolManagedGatewayDescriptor>,
    pub captured_at: DateTime<Utc>,
    pub audit_refs: Vec<ToolAuditRef>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGatewayMeteringEvent {
    pub trace: TraceContext,
    pub gateway_id: String,
    pub provider_id: String,
    pub tool_id: String,
    pub metering_ref: String,
    pub units: u64,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGatewayAuditEvent {
    pub trace: TraceContext,
    pub gateway_id: String,
    pub provider_id: String,
    pub tool_id: String,
    pub status: String,
    pub latency_millis: u64,
    pub input_hash: String,
    pub output_hash: String,
    pub artifact_refs: Vec<ToolArtifactRef>,
    pub metering_ref: Option<String>,
    pub metadata: BTreeMap<String, String>,
}
