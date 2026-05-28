//! Provider-neutral contracts for `service.config`.
//!
//! The config service owns layered configuration, executable requirements,
//! permission profile visibility, feature flags, schema reads, hot reload
//! notifications, and sanitized audit mementos.  The contracts intentionally
//! describe generic OS policy surfaces rather than application workflows.  Raw
//! secret values must never appear in snapshots, logs, events, or audit records.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::sandbox::{SandboxNetworkMode, SandboxPermissionProfile, SandboxRuntimeKind};
use super::WorkbenchServiceDescriptor;

pub const SERVICE_ID: &str = "service.config";
pub const CAPABILITY_ID: &str = "capability.config";
pub const READ_COMMAND: &str = "config.read";
pub const VALUE_WRITE_COMMAND: &str = "config.value.write";
pub const BATCH_WRITE_COMMAND: &str = "config.batch.write";
pub const SCHEMA_READ_COMMAND: &str = "config.schema.read";
pub const RELOAD_COMMAND: &str = "config.reload";
pub const REQUIREMENTS_READ_COMMAND: &str = "config.requirements.read";
pub const PERMISSION_PROFILE_LIST_COMMAND: &str = "permission_profile.list";
pub const FEATURE_LIST_COMMAND: &str = "feature.list";
pub const FEATURE_ENABLEMENT_SET_COMMAND: &str = "feature.enablement.set";

pub const COMMANDS: &[&str] = &[
    READ_COMMAND,
    VALUE_WRITE_COMMAND,
    BATCH_WRITE_COMMAND,
    SCHEMA_READ_COMMAND,
    RELOAD_COMMAND,
    REQUIREMENTS_READ_COMMAND,
    PERMISSION_PROFILE_LIST_COMMAND,
    FEATURE_LIST_COMMAND,
    FEATURE_ENABLEMENT_SET_COMMAND,
];

/// Ordered config layers used by effective-config resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLayer {
    Default,
    User,
    Project,
    Application,
    Session,
    Requirements,
    Managed,
}

/// Writer/source class used in audit and requirement precedence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSourceClass {
    BuiltIn,
    User,
    Project,
    Application,
    Session,
    Requirements,
    Managed,
}

/// Scope used for layered reads and reload notifications.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConfigScope {
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub project_ref: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigReadRequest {
    pub scope: ConfigScope,
    pub key_prefix: Option<String>,
    pub include_sources: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigValueWrite {
    pub layer: ConfigLayer,
    pub key_path: String,
    pub value: Value,
    pub writer_ref: String,
    pub reload: bool,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigBatchWrite {
    pub writes: Vec<ConfigValueWrite>,
    pub reload: bool,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSchemaRead {
    pub include_internal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigReloadRequest {
    pub scope: ConfigScope,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigRequirementsRead {
    pub scope: ConfigScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionProfileListRequest {
    pub scope: ConfigScope,
    pub runtime_kind: Option<SandboxRuntimeKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureListRequest {
    pub include_disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureEnablementSet {
    pub feature_key: String,
    pub enabled: bool,
    pub layer: ConfigLayer,
    pub writer_ref: String,
    pub reload: bool,
    pub metadata: BTreeMap<String, String>,
}

/// Sanitized value view returned to callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigValueView {
    pub key_path: String,
    pub layer: ConfigLayer,
    pub value: Value,
    pub redacted: bool,
    pub version: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveConfig {
    pub scope: ConfigScope,
    pub values: BTreeMap<String, ConfigValueView>,
    pub layer_order: Vec<ConfigLayer>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSchema {
    pub supported_layers: Vec<ConfigLayer>,
    pub supported_commands: Vec<String>,
    pub secret_key_fragments: Vec<String>,
    pub reload_supported: bool,
}

/// Executable requirements exposed to policy decorators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigRequirements {
    pub source_class: ConfigSourceClass,
    pub allowed_approval_profiles: Vec<String>,
    pub allowed_sandbox_profiles: Vec<String>,
    pub allowed_network_modes: Vec<SandboxNetworkMode>,
    pub allowed_permissions: Vec<String>,
    pub managed_hooks_only: bool,
    pub residency: Option<String>,
    pub feature_requirements: BTreeMap<String, bool>,
    pub network_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlagRecord {
    pub feature_key: String,
    pub enabled: bool,
    pub layer: ConfigLayer,
    pub source_class: ConfigSourceClass,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigAuditRecord {
    pub audit_ref: String,
    pub operation: String,
    pub key_paths: Vec<String>,
    pub writer_ref: Option<String>,
    pub reload: bool,
    pub trace_id: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigChangedEvent {
    pub event_ref: String,
    pub key_paths: Vec<String>,
    pub reload: bool,
    pub trace_id: String,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigServiceResponse {
    Read(EffectiveConfig),
    Written {
        values: Vec<ConfigValueView>,
        audit_ref: String,
        reload_emitted: bool,
    },
    Schema(ConfigSchema),
    Reloaded(ConfigChangedEvent),
    Requirements(ConfigRequirements),
    PermissionProfiles(Vec<SandboxPermissionProfile>),
    Features(Vec<FeatureFlagRecord>),
    Audit(ConfigAuditRecord),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.config.events.v1",
        "service.config.audit.v1",
        "service.config.snapshot.v1",
    )
}
