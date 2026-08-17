use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for provider-neutral filesystem access.
pub const FOUNDATION_FILESYSTEM_PACK_ID: &str = "pack.foundation.filesystem.v1";
/// Stable service id used by future filesystem providers.
pub const FOUNDATION_FILESYSTEM_SERVICE_ID: &str = "service.foundation.filesystem";

/// Canonical command names described by `pack.foundation.filesystem.v1`.
///
/// The command surface is intentionally provider-neutral: commands use logical
/// roots, paths, handles, and content references instead of host paths or raw
/// OS file descriptors.
pub const FOUNDATION_FILESYSTEM_COMMANDS: &[&str] = &[
    "filesystem.open_handle",
    "filesystem.close_handle",
    "filesystem.read_file",
    "filesystem.write_file",
    "filesystem.append_file",
    "filesystem.list_directory",
    "filesystem.stat_path",
    "filesystem.create_directory",
    "filesystem.copy_path",
    "filesystem.move_path",
    "filesystem.delete_path",
    "filesystem.create_temp",
    "filesystem.watch_path",
    "filesystem.snapshot_tree",
    "filesystem.restore_snapshot",
];

/// Build the descriptor-only catalog entry for `pack.foundation.filesystem.v1`.
///
/// The descriptor exposes discovery, policy, command schema, provider class, and
/// unavailable diagnostics metadata. It does not construct a local workspace,
/// WASI preopen, remote artifact, mock, or unavailable provider; those remain
/// service-runtime responsibilities.
pub fn foundation_filesystem_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_FILESYSTEM_COMMANDS);
    let result_schemas = FOUNDATION_FILESYSTEM_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_FILESYSTEM_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_FILESYSTEM_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_FILESYSTEM_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "filesystem.read",
                "filesystem.write",
                "filesystem.append",
                "filesystem.list",
                "filesystem.metadata",
                "filesystem.copy",
                "filesystem.move",
                "filesystem.delete",
                "filesystem.watch",
                "filesystem.temp",
                "filesystem.snapshot",
                "filesystem.restore",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-filesystem",
            ]),
            migration_notes: vec![
                "The filesystem pack is discoverable as an industrial descriptor and becomes callable only after an approved filesystem provider registers.".into(),
                "Applications receive logical root/path/handle references; raw host paths and provider handles remain private to providers.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(30_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "filesystem_metadata".into(),
                retention_policy: "content_by_reference_redacted_snapshot_metadata_only".into(),
                redaction_policy: "raw_host_paths_file_bytes_secrets_and_provider_payloads_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.filesystem".into(),
                docs_url: "docs://macaca/developer-packs/foundation/filesystem".into(),
                examples: vec![
                    "Declare `pack.foundation.filesystem.v1` as optional until a scoped filesystem provider is installed.".into(),
                    "Use logical roots, handles, and content references rather than raw host paths.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "filesystem.stat_path".into(),
                unavailable_reason: "filesystem_provider_not_installed".into(),
                replay_schema: "filesystem.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_FILESYSTEM_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: filesystem_provider_descriptors(),
        },
        [FOUNDATION_FILESYSTEM_SERVICE_ID.to_string()],
    )
}

fn filesystem_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor(
            "local-scoped-workspace",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("wasi-preopen", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "remote-artifact",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("mock", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "unavailable",
            DomainPackProviderCapabilityState::Unavailable,
        ),
    ]
    .into_iter()
    .map(|descriptor| (descriptor.provider_class.clone(), descriptor))
    .collect()
}

fn provider_descriptor(
    provider_class: &str,
    availability: DomainPackProviderCapabilityState,
) -> DomainPackProviderDescriptor {
    let capability = FilesystemProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_FILESYSTEM_COMMANDS),
        supported_root_kinds: schema_set(&["app_workspace", "session_workspace", "temporary"]),
        supports_recursive_operations: provider_class != "unavailable",
        supports_watch: provider_class != "unavailable",
        supports_snapshot: provider_class != "unavailable",
        supports_atomic_write: matches!(provider_class, "local-scoped-workspace" | "mock"),
        max_file_bytes: 16_777_216,
        max_directory_entries: 10_000,
        availability,
        unavailable_reason: (provider_class == "unavailable")
            .then(|| "filesystem_provider_not_installed".into()),
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_FILESYSTEM_SERVICE_ID.into(),
        availability,
        capability_hash: filesystem_stable_hash(&capability),
        compatibility_hash: "foundation-filesystem-provider-v1".into(),
        diagnostics_schema: "filesystem.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            (
                "max_file_bytes".into(),
                capability.max_file_bytes.to_string(),
            ),
            (
                "max_directory_entries".into(),
                capability.max_directory_entries.to_string(),
            ),
            ("watch".into(), capability.supports_watch.to_string()),
            ("snapshot".into(), capability.supports_snapshot.to_string()),
            (
                "atomic_write".into(),
                capability.supports_atomic_write.to_string(),
            ),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Logical root granted to an application by manifest admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemRootRef {
    pub root_id: String,
    pub root_kind: String,
}

/// Provider-neutral path under a logical root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPathRef {
    pub root: FilesystemRootRef,
    pub relative_path: String,
}

/// Opaque handle lease returned by a future filesystem provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemHandleRef {
    pub handle_id: String,
    pub root: FilesystemRootRef,
    pub access_mode: FilesystemAccessMode,
    pub revision_id: Option<String>,
}

/// Access mode requested for a path or handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemAccessMode {
    Read,
    Write,
    Append,
    List,
    Metadata,
    Delete,
    Watch,
    Snapshot,
    Restore,
}

/// Conflict behavior for mutating commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemConflictMode {
    Fail,
    Overwrite,
    CreateNew,
    MergeDirectory,
    Tombstone,
}

/// Opaque content reference used to keep file bytes out of traces and audits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemContentRef {
    pub content_ref: String,
    pub encoding: Option<String>,
    pub expected_hash: Option<String>,
}

/// Bounded metadata projection for stat, list, snapshot, and audit surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemMetadata {
    pub path_hash: String,
    pub entry_kind: String,
    pub size_bytes: Option<u64>,
    pub revision_id: Option<String>,
}

/// Stream event metadata for file watches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemWatchEvent {
    pub event_id: String,
    pub path_hash: String,
    pub event_kind: String,
}

/// Snapshot reference used by restore and replay tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemSnapshotRef {
    pub snapshot_id: String,
    pub root: FilesystemRootRef,
    pub tree_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    /// Declared logical root kinds accepted by this provider Strategy.
    pub supported_root_kinds: BTreeSet<String>,
    /// Whether bounded recursive copy/list/delete semantics are available.
    pub supports_recursive_operations: bool,
    pub supports_watch: bool,
    pub supports_snapshot: bool,
    pub supports_atomic_write: bool,
    pub max_file_bytes: u64,
    pub max_directory_entries: u32,
    pub availability: DomainPackProviderCapabilityState,
    /// Bounded diagnostic for unavailable providers; never a host/provider payload.
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub open_handle_count: u32,
    pub active_watch_count: u32,
    pub root_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemOpenHandleCommand {
    pub path: FilesystemPathRef,
    pub access_mode: FilesystemAccessMode,
    pub conflict_mode: FilesystemConflictMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemCloseHandleCommand {
    pub handle: FilesystemHandleRef,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemReadFileCommand {
    pub path: Option<FilesystemPathRef>,
    pub handle: Option<FilesystemHandleRef>,
    pub range_start: u64,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemWriteFileCommand {
    pub path: FilesystemPathRef,
    pub content: FilesystemContentRef,
    pub conflict_mode: FilesystemConflictMode,
    pub atomic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemAppendFileCommand {
    pub path: FilesystemPathRef,
    pub content: FilesystemContentRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemListDirectoryCommand {
    pub path: FilesystemPathRef,
    pub recursive: bool,
    pub page_size: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemStatPathCommand {
    pub path: FilesystemPathRef,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemCreateDirectoryCommand {
    pub path: FilesystemPathRef,
    pub recursive: bool,
    pub conflict_mode: FilesystemConflictMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemCopyPathCommand {
    pub source: FilesystemPathRef,
    pub destination: FilesystemPathRef,
    pub recursive: bool,
    pub conflict_mode: FilesystemConflictMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemMovePathCommand {
    pub source: FilesystemPathRef,
    pub destination: FilesystemPathRef,
    pub atomic_preferred: bool,
    pub conflict_mode: FilesystemConflictMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemDeletePathCommand {
    pub path: FilesystemPathRef,
    pub recursive: bool,
    pub tombstone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemCreateTempCommand {
    pub root: FilesystemRootRef,
    pub namespace: String,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemWatchPathCommand {
    pub path: FilesystemPathRef,
    pub recursive: bool,
    pub event_filter: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemSnapshotTreeCommand {
    pub root: FilesystemRootRef,
    pub include_pattern: Option<String>,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemRestoreSnapshotCommand {
    pub snapshot: FilesystemSnapshotRef,
    pub target_root: FilesystemRootRef,
    pub conflict_mode: FilesystemConflictMode,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemResultStatus {
    Success,
    PartialStreamPage,
    Denied,
    NotFound,
    AlreadyExists,
    Conflict,
    InvalidPath,
    InvalidHandle,
    QuotaExceeded,
    TooLarge,
    Unsupported,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemError {
    pub code: FilesystemResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemResultEnvelope<T> {
    pub status: FilesystemResultStatus,
    pub data: Option<T>,
    pub error: Option<FilesystemError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the filesystem contract surface.
pub fn foundation_filesystem_descriptor_hashes() -> FilesystemDescriptorHashes {
    FilesystemDescriptorHashes {
        command_schema_hash: filesystem_stable_hash(&FOUNDATION_FILESYSTEM_COMMANDS),
        result_schema_hash: filesystem_stable_hash(&FilesystemResultStatus::Success),
        snapshot_schema_hash: filesystem_stable_hash(&FilesystemProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            open_handle_count: 0,
            active_watch_count: 0,
            root_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: filesystem_stable_hash(&FilesystemProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(FOUNDATION_FILESYSTEM_COMMANDS),
            supported_root_kinds: BTreeSet::new(),
            supports_recursive_operations: false,
            supports_watch: false,
            supports_snapshot: false,
            supports_atomic_write: false,
            max_file_bytes: 0,
            max_directory_entries: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
            unavailable_reason: Some("filesystem_provider_not_installed".into()),
        }),
        unavailable_schema_hash: filesystem_stable_hash(&FilesystemError {
            code: FilesystemResultStatus::Unavailable,
            message: "filesystem provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor compatibility tests.
pub fn filesystem_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
