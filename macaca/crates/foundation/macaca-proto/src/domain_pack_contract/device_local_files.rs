use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::device_common::{
    define_device_command_wrappers, device_pack_definition, device_stable_hash,
    DevicePackCommandEnvelope, DevicePackDescriptor, DevicePackError, DevicePackPage,
    DeviceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVICE_LOCAL_FILES_PACK_ID: &str = "pack.device.local.files.v1";
pub const DEVICE_LOCAL_FILES_SERVICE_ID: &str = "service.device.local_files";

pub const DEVICE_LOCAL_FILES_COMMANDS: &[&str] = &[
    "local_files.request_open_handle",
    "local_files.request_save_handle",
    "local_files.request_directory_handle",
    "local_files.inspect_handle",
    "local_files.list_handles",
    "local_files.revoke_handle",
    "local_files.read",
    "local_files.write",
    "local_files.append",
    "local_files.truncate",
    "local_files.list_directory",
    "local_files.import_file",
    "local_files.export_file",
    "local_files.cancel_transfer",
    "local_files.inspect_host",
];

const LOCAL_FILES_PERMISSION_SCOPES: &[&str] = &[
    "device.local_files.open",
    "device.local_files.save",
    "device.local_files.directory",
    "device.local_files.read",
    "device.local_files.write",
    "device.local_files.grant.manage",
];

const LOCAL_HOST_METADATA: &[(&str, &str)] = &[
    ("host_native", "true"),
    ("raw_paths", "false"),
    ("picker", "host_owned"),
];
const LOCAL_BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser", "true"),
    ("scoped_handles", "true"),
    ("raw_contents_in_trace", "false"),
];
const LOCAL_REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_host", "true"),
    ("transfer", "policy_bound"),
    ("content_scan", "required_when_configured"),
];
const LOCAL_MOCK_METADATA: &[(&str, &str)] = &[("fixtures", "synthetic"), ("callable", "false")];
const LOCAL_UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("picker", "false"), ("reason", "provider_not_installed")];

const LOCAL_FILES_PROVIDER_CLASSES: &[DeviceProviderClass<'_>] = &[
    DeviceProviderClass {
        provider_class: "host-native",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_HOST_METADATA,
    },
    DeviceProviderClass {
        provider_class: "browser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_BROWSER_METADATA,
    },
    DeviceProviderClass {
        provider_class: "remote-host",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_REMOTE_METADATA,
    },
    DeviceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_MOCK_METADATA,
    },
    DeviceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: LOCAL_UNAVAILABLE_METADATA,
    },
];

/// Build the local-files descriptor without binding host filesystem APIs.
pub fn device_local_files_pack_definition() -> DomainPackDefinition {
    device_pack_definition(DevicePackDescriptor {
        pack_id: DEVICE_LOCAL_FILES_PACK_ID,
        child_change_id: "openspec:add-pack-device-local-files",
        docs_slug: "local-files",
        sdk_slug: "local.files",
        service_id: DEVICE_LOCAL_FILES_SERVICE_ID,
        commands: DEVICE_LOCAL_FILES_COMMANDS,
        permission_scopes: LOCAL_FILES_PERMISSION_SCOPES,
        provider_classes: LOCAL_FILES_PROVIDER_CLASSES,
        health_probe: "local_files.inspect_host",
        unavailable_reason: "device_local_files_provider_not_installed",
        replay_schema: "device.local_files.replay.v1",
        data_classification: "device_local_file_reference_metadata",
        retention_policy: "handles_grants_metadata_filters_chunks_transfers_write_plans_and_host_status_by_reference",
        redaction_policy: "raw_host_paths_raw_file_contents_file_names_when_forbidden_transfer_chunks_provider_payloads_and_credentials_redacted",
        timeout_ms: 120_000,
        budget_units: 6,
        examples: &[
            "Declare `pack.device.local.files.v1` as optional until a local-file provider is installed.",
            "Use picker grants, opaque handles, chunks, write plans, and transfer handles instead of raw host paths.",
        ],
        migration_notes: &[
            "Local-file commands become callable only after an approved local file service provider registers matching schemas.",
            "Foundation filesystem, package runtime storage, media parsing, document parsing, camera, and application file workflows remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileHandle {
    pub handle_ref: String,
    pub handle_kind: String,
    pub grant_ref: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileGrant {
    pub grant_ref: String,
    pub state: String,
    pub scope: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileMetadata {
    pub metadata_ref: String,
    pub handle_ref: String,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileFilter {
    pub filter_ref: String,
    pub mime_types: BTreeSet<String>,
    pub extensions: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileChunk {
    pub chunk_ref: String,
    pub offset: u64,
    pub length: u64,
    pub content_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileTransfer {
    pub transfer_ref: String,
    pub handle_ref: String,
    pub direction: String,
    pub state: String,
    pub bytes_total: u64,
}

impl LocalFileTransfer {
    /// Bound transfers before host file IO can be requested.
    pub fn is_bounded(&self, max_bytes: u64) -> bool {
        self.bytes_total <= max_bytes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileDirectoryEntry {
    pub entry_ref: String,
    pub handle_ref: String,
    pub entry_kind: String,
    pub display_name_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileWritePlan {
    pub plan_ref: String,
    pub handle_ref: String,
    pub mode: String,
    pub destructive: bool,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileHostStatus {
    pub host_ref: String,
    pub picker_available: bool,
    pub foreground_required: bool,
    pub directory_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFileError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_device_command_wrappers!(
    LocalFilesRequestOpenHandleCommand,
    LocalFilesRequestSaveHandleCommand,
    LocalFilesRequestDirectoryHandleCommand,
    LocalFilesInspectHandleCommand,
    LocalFilesListHandlesCommand,
    LocalFilesRevokeHandleCommand,
    LocalFilesReadCommand,
    LocalFilesWriteCommand,
    LocalFilesAppendCommand,
    LocalFilesTruncateCommand,
    LocalFilesListDirectoryCommand,
    LocalFilesImportFileCommand,
    LocalFilesExportFileCommand,
    LocalFilesCancelTransferCommand,
    LocalFilesInspectHostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFilesResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    PickerCancelled,
    PermissionPromptRequired,
    ForegroundRequired,
    GrantExpired,
    HandleRevoked,
    HandleNotFound,
    ReadOnly,
    WriteConflict,
    DestructiveApprovalRequired,
    FileTooLarge,
    DirectoryTraversalDenied,
    ContentScanBlocked,
    TransferCancelled,
    QuotaExceeded,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFilesResultEnvelope<T> {
    pub status: LocalFilesResultStatus,
    pub data: Option<T>,
    pub page: Option<DevicePackPage<T>>,
    pub error: Option<DevicePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalFilesDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub grant_hash: String,
    pub metadata_hash: String,
    pub transfer_hash: String,
    pub redaction_profile_hash: String,
}

pub fn device_local_files_descriptor_hashes() -> LocalFilesDescriptorHashes {
    LocalFilesDescriptorHashes {
        command_schema_hash: local_files_stable_hash(&DEVICE_LOCAL_FILES_COMMANDS),
        result_schema_hash: local_files_stable_hash(&LocalFilesResultStatus::Success),
        descriptor_hash: local_files_stable_hash(&device_local_files_pack_definition()),
        grant_hash: local_files_stable_hash(&LocalFileGrant {
            grant_ref: "grant".into(),
            state: "granted".into(),
            scope: "read".into(),
            expires_at_epoch_ms: 10,
        }),
        metadata_hash: local_files_stable_hash(&LocalFileMetadata {
            metadata_ref: "metadata".into(),
            handle_ref: "handle".into(),
            size_bytes: 1024,
            mime_type: Some("application/octet-stream".into()),
        }),
        transfer_hash: local_files_stable_hash(&LocalFileTransfer {
            transfer_ref: "transfer".into(),
            handle_ref: "handle".into(),
            direction: "read".into(),
            state: "active".into(),
            bytes_total: 1024,
        }),
        redaction_profile_hash: local_files_stable_hash("local-files-redaction-v1"),
    }
}

pub fn local_files_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    device_stable_hash(value)
}
