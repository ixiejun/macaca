use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for provider-neutral session state.
pub const FOUNDATION_SESSION_STATE_PACK_ID: &str = "pack.foundation.session.state.v1";
/// Stable service id used by future session-state providers.
pub const FOUNDATION_SESSION_STATE_SERVICE_ID: &str = "service.foundation.session.state";

/// Canonical command names described by `pack.foundation.session.state.v1`.
///
/// These commands are session-state primitives only. Workflow recovery, task-board repair, and
/// review semantics belong to autonomy services, not this foundation pack.
pub const FOUNDATION_SESSION_STATE_COMMANDS: &[&str] = &[
    "session_state.get",
    "session_state.put",
    "session_state.delete",
    "session_state.merge_patch",
    "session_state.list_keys",
    "session_state.create_checkpoint",
    "session_state.list_checkpoints",
    "session_state.restore_checkpoint",
    "session_state.compare_checkpoint",
    "session_state.compact_history",
    "session_state.clear_session",
    "session_state.export_redacted",
    "session_state.inspect_recovery",
];

/// Build the descriptor-only catalog entry for `pack.foundation.session.state.v1`.
///
/// The descriptor defines the contract for session-scoped values, revisions, checkpoints, and
/// redacted recovery metadata without binding an embedded store, Redis-like store, or replay
/// provider. Concrete mutation and restore behavior must remain behind serviceized providers.
pub fn foundation_session_state_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_SESSION_STATE_COMMANDS);
    let result_schemas = FOUNDATION_SESSION_STATE_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_SESSION_STATE_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_SESSION_STATE_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_SESSION_STATE_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "session_state.read",
                "session_state.write",
                "session_state.delete",
                "session_state.list",
                "session_state.checkpoint",
                "session_state.restore",
                "session_state.compact",
                "session_state.clear",
                "session_state.export",
                "session_state.inspect_recovery",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-session-state",
            ]),
            migration_notes: vec![
                "The session-state pack is discoverable as an industrial descriptor and becomes callable only after an approved session-state service provider registers.".into(),
                "Workflow repair, task-board transitions, and shell-owned recovery semantics are explicitly outside this pack.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(5_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "session_state_metadata".into(),
                retention_policy: "state_values_by_reference_redacted_checkpoint_metadata_only".into(),
                redaction_policy: "raw_state_values_secrets_and_provider_payloads_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.session.state".into(),
                docs_url: "docs://macaca/developer-packs/foundation/session-state".into(),
                examples: vec![
                    "Declare `pack.foundation.session.state.v1` as optional until a session-state provider is installed.".into(),
                    "Use checkpoints and redacted exports for diagnostics without logging raw state values.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "session_state.inspect_recovery".into(),
                unavailable_reason: "session_state_provider_not_installed".into(),
                replay_schema: "session.state.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_SESSION_STATE_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: session_state_provider_descriptors(),
        },
        [FOUNDATION_SESSION_STATE_SERVICE_ID.to_string()],
    )
}

fn session_state_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor("embedded", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "remote-session-store",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("replay", DomainPackProviderCapabilityState::Preview),
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
    let capability = SessionStateProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_SESSION_STATE_COMMANDS),
        supports_checkpoints: provider_class != "unavailable",
        supports_restore: matches!(provider_class, "embedded" | "replay" | "mock"),
        supports_compaction: provider_class != "unavailable",
        supports_redacted_export: true,
        max_state_bytes: 1_048_576,
        max_checkpoint_bytes: 4_194_304,
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_SESSION_STATE_SERVICE_ID.into(),
        availability,
        capability_hash: session_state_stable_hash(&capability),
        compatibility_hash: "foundation-session-state-provider-v1".into(),
        diagnostics_schema: "session.state.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("max_state_bytes".into(), "1048576".into()),
            ("max_checkpoint_bytes".into(), "4194304".into()),
            (
                "checkpoints".into(),
                capability.supports_checkpoints.to_string(),
            ),
            ("restore".into(), capability.supports_restore.to_string()),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateSessionRef {
    pub session_id: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateKeyRef {
    pub session: SessionStateSessionRef,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateValueRef {
    pub value_ref: String,
    pub schema_id: Option<String>,
    pub secret_reference_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRevision {
    pub revision_id: String,
    pub previous_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateCheckpointRef {
    pub checkpoint_id: String,
    pub session: SessionStateSessionRef,
    pub revision_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRestorePlan {
    pub checkpoint: SessionStateCheckpointRef,
    pub dry_run: bool,
    pub cross_session_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRecoveryMetadata {
    pub latest_checkpoint: Option<SessionStateCheckpointRef>,
    pub latest_revision: Option<SessionStateRevision>,
    pub recovery_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRetentionPolicy {
    pub ttl_seconds: Option<u64>,
    pub max_checkpoints: u32,
    pub compact_after_revisions: u32,
}

/// Bounded manifest-time declaration for one application session-state scope.
///
/// This Value Object defines requested capability facts only. It intentionally
/// excludes raw state, database configuration, remote store URLs, and provider
/// handles so the Application Framework can validate admission generically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateManifestDeclaration {
    pub session: SessionStateSessionRef,
    pub checkpoint_support_required: bool,
    pub restore_support_required: bool,
    pub compaction_support_required: bool,
    pub retention: SessionStateRetentionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRedactionSummary {
    pub redacted_value_count: u32,
    pub redacted_secret_reference_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supports_checkpoints: bool,
    pub supports_restore: bool,
    pub supports_compaction: bool,
    pub supports_redacted_export: bool,
    pub max_state_bytes: u64,
    pub max_checkpoint_bytes: u64,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub revision_hashes: BTreeMap<String, String>,
    pub checkpoint_hashes: BTreeMap<String, String>,
    pub redaction_summary: SessionStateRedactionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateGetCommand {
    pub key: SessionStateKeyRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatePutCommand {
    pub key: SessionStateKeyRef,
    pub value: SessionStateValueRef,
    pub expected_revision: Option<SessionStateRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateDeleteCommand {
    pub key: SessionStateKeyRef,
    pub expected_revision: Option<SessionStateRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateMergePatchCommand {
    pub key: SessionStateKeyRef,
    pub patch_ref: String,
    pub expected_revision: Option<SessionStateRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateListKeysCommand {
    pub session: SessionStateSessionRef,
    pub prefix: Option<String>,
    pub page_size: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateCreateCheckpointCommand {
    pub session: SessionStateSessionRef,
    pub retention: SessionStateRetentionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateListCheckpointsCommand {
    pub session: SessionStateSessionRef,
    pub cursor: Option<String>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateRestoreCheckpointCommand {
    pub plan: SessionStateRestorePlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateCompareCheckpointCommand {
    pub left: SessionStateCheckpointRef,
    pub right: SessionStateCheckpointRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateCompactHistoryCommand {
    pub session: SessionStateSessionRef,
    pub before_revision: SessionStateRevision,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateClearSessionCommand {
    pub session: SessionStateSessionRef,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateExportRedactedCommand {
    pub session: SessionStateSessionRef,
    pub redaction_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateInspectRecoveryCommand {
    pub session: SessionStateSessionRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStateResultStatus {
    Success,
    PartialPage,
    Denied,
    NotFound,
    Conflict,
    InvalidSession,
    InvalidKey,
    InvalidCheckpoint,
    SchemaMismatch,
    QuotaExceeded,
    TooLarge,
    Unsupported,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateError {
    pub code: SessionStateResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateResultEnvelope<T> {
    pub status: SessionStateResultStatus,
    pub data: Option<T>,
    pub error: Option<SessionStateError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the session-state contract surface.
pub fn foundation_session_state_descriptor_hashes() -> SessionStateDescriptorHashes {
    let redaction = SessionStateRedactionSummary {
        redacted_value_count: 1,
        redacted_secret_reference_count: 1,
    };
    SessionStateDescriptorHashes {
        command_schema_hash: session_state_stable_hash(&FOUNDATION_SESSION_STATE_COMMANDS),
        result_schema_hash: session_state_stable_hash(&SessionStateResultStatus::Success),
        snapshot_schema_hash: session_state_stable_hash(&SessionStateProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            revision_hashes: BTreeMap::new(),
            checkpoint_hashes: BTreeMap::new(),
            redaction_summary: redaction,
        }),
        provider_capability_schema_hash: session_state_stable_hash(
            &SessionStateProviderCapability {
                provider_class: "unavailable".into(),
                supported_commands: schema_set(FOUNDATION_SESSION_STATE_COMMANDS),
                supports_checkpoints: false,
                supports_restore: false,
                supports_compaction: false,
                supports_redacted_export: true,
                max_state_bytes: 0,
                max_checkpoint_bytes: 0,
                availability: DomainPackProviderCapabilityState::Unavailable,
            },
        ),
        unavailable_schema_hash: session_state_stable_hash(&SessionStateError {
            code: SessionStateResultStatus::Unavailable,
            message: "session-state provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor compatibility tests.
pub fn session_state_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
