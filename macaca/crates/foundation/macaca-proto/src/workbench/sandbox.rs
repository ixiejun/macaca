//! Provider-neutral contracts for `service.sandbox`.
//!
//! The sandbox service owns permission-profile resolution, runtime environment
//! lifecycle records, and policy explanations for privileged workbench calls.
//! DTOs are intentionally data-only: runtime-host providers decide how to map
//! these records to local, container, remote, browser, or WASM isolation while
//! callers continue to depend on stable profile and environment semantics.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::WorkbenchServiceDescriptor;

pub const SERVICE_ID: &str = "service.sandbox";
pub const CAPABILITY_ID: &str = "capability.sandbox";
pub const PROFILE_LIST_COMMAND: &str = "sandbox.profile.list";
pub const PROFILE_RESOLVE_COMMAND: &str = "sandbox.profile.resolve";
pub const ENVIRONMENT_PREPARE_COMMAND: &str = "sandbox.environment.prepare";
pub const ENVIRONMENT_HEALTH_COMMAND: &str = "sandbox.environment.health";
pub const ENVIRONMENT_CLEANUP_COMMAND: &str = "sandbox.environment.cleanup";
pub const POLICY_EXPLAIN_COMMAND: &str = "sandbox.policy.explain";
pub const TOOL_INVOKE_COMMAND: &str = "sandbox.tool.invoke";

pub const READ_ONLY_PROFILE: &str = "permission.read_only";
pub const WORKSPACE_WRITE_PROFILE: &str = "permission.workspace_write";
pub const FULL_ACCESS_PROFILE: &str = "permission.full_access";
pub const NETWORK_DENIED_PROFILE: &str = "permission.network.denied";
pub const NETWORK_ALLOWED_PROFILE: &str = "permission.network.allowed";
pub const REMOTE_ENVIRONMENT_PROFILE: &str = "permission.remote_environment";
pub const LEGACY_WORKSPACE_PROCESS_PROFILE: &str = "permission.workspace.process";
pub const LEGACY_DENY_PROCESS_PROFILE: &str = "deny-process";

pub const COMMANDS: &[&str] = &[
    PROFILE_LIST_COMMAND,
    PROFILE_RESOLVE_COMMAND,
    ENVIRONMENT_PREPARE_COMMAND,
    ENVIRONMENT_HEALTH_COMMAND,
    ENVIRONMENT_CLEANUP_COMMAND,
    POLICY_EXPLAIN_COMMAND,
    TOOL_INVOKE_COMMAND,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRuntimeKind {
    LocalWorkspace,
    LocalSandbox,
    Docker,
    SshRemote,
    OsSpecific,
    Browser,
    Wasm,
    RemoteEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEnvironmentState {
    Unavailable,
    Preparing,
    Ready,
    Degraded,
    Cleaning,
    Cleaned,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEnvironmentScope {
    PerCall,
    Thread,
    Session,
    Application,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxNetworkMode {
    Denied,
    Restricted,
    Allowed,
    RemoteOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxWriteScope {
    None,
    Workspace,
    FullAccess,
    RemoteEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPermissionProfile {
    pub profile_ref: String,
    pub label: String,
    pub can_read_workspace: bool,
    pub can_write_workspace: bool,
    pub can_spawn_process: bool,
    pub network_mode: SandboxNetworkMode,
    pub write_scope: SandboxWriteScope,
    pub remote_environment_allowed: bool,
    pub audit_tags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl SandboxPermissionProfile {
    /// Return the built-in provider-neutral permission profiles.
    ///
    /// These profiles are OS capability classes, not application workflows.  A
    /// future config or policy service can add tenant/application constrained
    /// profiles without changing process, file, or tool contracts.
    pub fn catalog() -> Vec<Self> {
        vec![
            Self {
                profile_ref: READ_ONLY_PROFILE.into(),
                label: "Read-only workspace".into(),
                can_read_workspace: true,
                can_write_workspace: false,
                can_spawn_process: true,
                network_mode: SandboxNetworkMode::Denied,
                write_scope: SandboxWriteScope::None,
                remote_environment_allowed: false,
                audit_tags: vec!["read_only".into(), "network_denied".into()],
                metadata: BTreeMap::new(),
            },
            Self {
                profile_ref: WORKSPACE_WRITE_PROFILE.into(),
                label: "Workspace write".into(),
                can_read_workspace: true,
                can_write_workspace: true,
                can_spawn_process: true,
                network_mode: SandboxNetworkMode::Denied,
                write_scope: SandboxWriteScope::Workspace,
                remote_environment_allowed: false,
                audit_tags: vec!["workspace_write".into(), "network_denied".into()],
                metadata: BTreeMap::new(),
            },
            Self {
                profile_ref: FULL_ACCESS_PROFILE.into(),
                label: "Full local access".into(),
                can_read_workspace: true,
                can_write_workspace: true,
                can_spawn_process: true,
                network_mode: SandboxNetworkMode::Allowed,
                write_scope: SandboxWriteScope::FullAccess,
                remote_environment_allowed: false,
                audit_tags: vec!["full_access".into(), "network_allowed".into()],
                metadata: BTreeMap::new(),
            },
            Self {
                profile_ref: NETWORK_DENIED_PROFILE.into(),
                label: "Network denied".into(),
                can_read_workspace: true,
                can_write_workspace: false,
                can_spawn_process: true,
                network_mode: SandboxNetworkMode::Denied,
                write_scope: SandboxWriteScope::None,
                remote_environment_allowed: false,
                audit_tags: vec!["network_denied".into()],
                metadata: BTreeMap::new(),
            },
            Self {
                profile_ref: NETWORK_ALLOWED_PROFILE.into(),
                label: "Network allowed".into(),
                can_read_workspace: true,
                can_write_workspace: false,
                can_spawn_process: true,
                network_mode: SandboxNetworkMode::Allowed,
                write_scope: SandboxWriteScope::None,
                remote_environment_allowed: false,
                audit_tags: vec!["network_allowed".into()],
                metadata: BTreeMap::new(),
            },
            Self {
                profile_ref: REMOTE_ENVIRONMENT_PROFILE.into(),
                label: "Remote environment".into(),
                can_read_workspace: false,
                can_write_workspace: false,
                can_spawn_process: false,
                network_mode: SandboxNetworkMode::RemoteOnly,
                write_scope: SandboxWriteScope::RemoteEnvironment,
                remote_environment_allowed: true,
                audit_tags: vec!["remote_environment".into()],
                metadata: BTreeMap::new(),
            },
        ]
    }
}

pub fn resolve_builtin_permission_profile(profile_ref: &str) -> Option<SandboxPermissionProfile> {
    let normalized = match profile_ref {
        LEGACY_WORKSPACE_PROCESS_PROFILE => WORKSPACE_WRITE_PROFILE,
        LEGACY_DENY_PROCESS_PROFILE => READ_ONLY_PROFILE,
        other => other,
    };
    let mut profile = SandboxPermissionProfile::catalog()
        .into_iter()
        .find(|profile| profile.profile_ref == normalized)?;
    if profile_ref == LEGACY_DENY_PROCESS_PROFILE {
        profile.profile_ref = LEGACY_DENY_PROCESS_PROFILE.into();
        profile.can_spawn_process = false;
        profile.audit_tags.push("process_denied".into());
    }
    Some(profile)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfileListRequest {
    pub application_id: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfileResolveRequest {
    pub requested_profile_ref: String,
    pub application_id: Option<String>,
    pub workspace_root: Option<String>,
    pub runtime_kind: SandboxRuntimeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxEnvironmentPrepareRequest {
    pub requested_profile_ref: String,
    pub runtime_kind: SandboxRuntimeKind,
    pub scope: SandboxEnvironmentScope,
    pub workspace_root: Option<String>,
    pub cwd: Option<String>,
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub thread_ref: Option<String>,
    pub resource_request_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxEnvironmentHealthRequest {
    pub environment_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxEnvironmentCleanupRequest {
    pub environment_id: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicyExplainRequest {
    pub requested_profile_ref: String,
    pub runtime_kind: SandboxRuntimeKind,
    pub workspace_root: Option<String>,
    pub path: Option<String>,
    pub network_host_ref: Option<String>,
    pub env_var_key: Option<String>,
    pub write_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxResourceLease {
    pub lease_ref: String,
    pub resource_class: String,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxEnvironmentRecord {
    pub environment_id: String,
    pub runtime_kind: SandboxRuntimeKind,
    pub state: SandboxEnvironmentState,
    pub permission_profile: SandboxPermissionProfile,
    pub scope: SandboxEnvironmentScope,
    pub workspace_root_ref: Option<String>,
    pub cwd_ref: Option<String>,
    pub resource_leases: Vec<SandboxResourceLease>,
    pub trace_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicyDecision {
    pub rule: String,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxServiceResponse {
    ProfileList {
        profiles: Vec<SandboxPermissionProfile>,
    },
    ProfileResolved {
        profile: SandboxPermissionProfile,
    },
    EnvironmentPrepared {
        record: SandboxEnvironmentRecord,
    },
    EnvironmentHealth {
        records: Vec<SandboxEnvironmentRecord>,
    },
    EnvironmentCleaned {
        record: SandboxEnvironmentRecord,
        released_lease_refs: Vec<String>,
    },
    PolicyExplanation {
        decisions: Vec<SandboxPolicyDecision>,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.sandbox.events.v1",
        "service.sandbox.audit.v1",
        "service.sandbox.snapshot.v1",
    )
}
