use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for provider-neutral key-value state.
pub const FOUNDATION_KEY_VALUE_STATE_PACK_ID: &str = "pack.foundation.key.value.state.v1";
/// Stable service id used by future key-value state providers.
pub const FOUNDATION_KEY_VALUE_STATE_SERVICE_ID: &str = "service.foundation.key.value.state";

/// Canonical command names described by `pack.foundation.key.value.state.v1`.
///
/// Commands are namespace-scoped state primitives. They do not expose
/// provider-native APIs, cluster topology, database handles, or business keys.
pub const FOUNDATION_KEY_VALUE_STATE_COMMANDS: &[&str] = &[
    "kv.get",
    "kv.put",
    "kv.delete",
    "kv.exists",
    "kv.batch_get",
    "kv.batch_put",
    "kv.batch_delete",
    "kv.list_keys",
    "kv.compare_and_set",
    "kv.increment",
    "kv.set_ttl",
    "kv.get_ttl",
    "kv.watch_namespace",
    "kv.snapshot_namespace",
    "kv.restore_namespace",
    "kv.migrate_namespace",
    "kv.compact_namespace",
];

/// Build the descriptor-only catalog entry for key-value state.
pub fn foundation_key_value_state_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_KEY_VALUE_STATE_COMMANDS);
    let result_schemas = FOUNDATION_KEY_VALUE_STATE_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_KEY_VALUE_STATE_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_KEY_VALUE_STATE_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_KEY_VALUE_STATE_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "state.read",
                "state.write",
                "state.delete",
                "state.list",
                "state.watch",
                "state.ttl",
                "state.counter",
                "state.snapshot",
                "state.restore",
                "state.migrate",
                "state.compact",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-key-value-state",
            ]),
            migration_notes: vec![
                "The key-value state pack is discoverable as an industrial descriptor and becomes callable only after an approved state provider registers.".into(),
                "Namespaces, keys, revisions, TTL, watch, snapshot, and compaction metadata are provider-neutral and do not leak backend-native APIs.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(10_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "key_value_state_metadata".into(),
                retention_policy: "values_by_reference_redacted_snapshot_metadata_only".into(),
                redaction_policy: "raw_values_secrets_provider_payloads_and_unbounded_key_lists_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.keyValueState".into(),
                docs_url: "docs://macaca/developer-packs/foundation/key-value-state".into(),
                examples: vec![
                    "Declare `pack.foundation.key.value.state.v1` as optional until a state provider is installed.".into(),
                    "Use revisions and compare-and-set for optimistic concurrency without exposing backend transactions.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "kv.get_ttl".into(),
                unavailable_reason: "key_value_state_provider_not_installed".into(),
                replay_schema: "key.value.state.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_KEY_VALUE_STATE_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: key_value_state_provider_descriptors(),
        },
        [FOUNDATION_KEY_VALUE_STATE_SERVICE_ID.to_string()],
    )
}

fn key_value_state_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor(
            "embedded-durable",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("remote-kv", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "lease-consensus",
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
    let capability = KeyValueStateProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_KEY_VALUE_STATE_COMMANDS),
        supports_ttl: provider_class != "unavailable",
        supports_watch: provider_class != "unavailable",
        supports_snapshot: provider_class != "unavailable",
        supports_compaction: matches!(provider_class, "embedded-durable" | "lease-consensus"),
        max_value_bytes: 1_048_576,
        max_batch_entries: 500,
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_KEY_VALUE_STATE_SERVICE_ID.into(),
        availability,
        capability_hash: key_value_state_stable_hash(&capability),
        compatibility_hash: "foundation-key-value-state-provider-v1".into(),
        diagnostics_schema: "key.value.state.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("ttl".into(), capability.supports_ttl.to_string()),
            ("watch".into(), capability.supports_watch.to_string()),
            ("snapshot".into(), capability.supports_snapshot.to_string()),
            (
                "compaction".into(),
                capability.supports_compaction.to_string(),
            ),
            (
                "max_batch_entries".into(),
                capability.max_batch_entries.to_string(),
            ),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueNamespaceRef {
    pub namespace: String,
    pub tenant_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueKeyRef {
    pub namespace: KeyValueNamespaceRef,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueTypedValueRef {
    pub value_ref: String,
    pub value_kind: String,
    pub schema_id: Option<String>,
    pub secret_reference_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueRevision {
    pub revision_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueTtlPolicy {
    pub ttl_seconds: Option<u64>,
    pub expire_at_epoch_millis: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyValueConsistencyLevel {
    Local,
    Session,
    Strong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyValueConflictMode {
    Fail,
    Overwrite,
    CompareRevision,
    MergeObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueWatchEvent {
    pub event_id: String,
    pub key_hash: String,
    pub revision: KeyValueRevision,
    pub event_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueSnapshotRef {
    pub snapshot_id: String,
    pub namespace: KeyValueNamespaceRef,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueStateProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supports_ttl: bool,
    pub supports_watch: bool,
    pub supports_snapshot: bool,
    pub supports_compaction: bool,
    pub max_value_bytes: u64,
    pub max_batch_entries: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueStateProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub namespace_hashes: BTreeMap<String, String>,
    pub active_watch_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueGetCommand {
    pub key: KeyValueKeyRef,
    pub consistency: KeyValueConsistencyLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePutCommand {
    pub key: KeyValueKeyRef,
    pub value: KeyValueTypedValueRef,
    pub ttl: Option<KeyValueTtlPolicy>,
    pub conflict_mode: KeyValueConflictMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueDeleteCommand {
    pub key: KeyValueKeyRef,
    pub expected_revision: Option<KeyValueRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueExistsCommand {
    pub key: KeyValueKeyRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueBatchGetCommand {
    pub keys: Vec<KeyValueKeyRef>,
    pub consistency: KeyValueConsistencyLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueBatchPutCommand {
    pub entries: Vec<KeyValuePutCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueBatchDeleteCommand {
    pub keys: Vec<KeyValueKeyRef>,
    pub expected_revision: Option<KeyValueRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueListKeysCommand {
    pub namespace: KeyValueNamespaceRef,
    pub prefix: Option<String>,
    pub page_size: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueCompareAndSetCommand {
    pub key: KeyValueKeyRef,
    pub expected_revision: KeyValueRevision,
    pub value: KeyValueTypedValueRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueIncrementCommand {
    pub key: KeyValueKeyRef,
    pub delta: i64,
    pub initialize: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueSetTtlCommand {
    pub key: KeyValueKeyRef,
    pub ttl: KeyValueTtlPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueGetTtlCommand {
    pub key: KeyValueKeyRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueWatchNamespaceCommand {
    pub namespace: KeyValueNamespaceRef,
    pub prefix: Option<String>,
    pub start_revision: Option<KeyValueRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueSnapshotNamespaceCommand {
    pub namespace: KeyValueNamespaceRef,
    pub include_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueRestoreNamespaceCommand {
    pub snapshot: KeyValueSnapshotRef,
    pub conflict_mode: KeyValueConflictMode,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueMigrateNamespaceCommand {
    pub source: KeyValueNamespaceRef,
    pub target: KeyValueNamespaceRef,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueCompactNamespaceCommand {
    pub namespace: KeyValueNamespaceRef,
    pub before_revision: KeyValueRevision,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyValueStateResultStatus {
    Success,
    PartialPage,
    WatchCheckpoint,
    Denied,
    NotFound,
    AlreadyExists,
    Conflict,
    InvalidKey,
    InvalidNamespace,
    QuotaExceeded,
    TooLarge,
    Unsupported,
    Expired,
    CompactedRevision,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueStateError {
    pub code: KeyValueStateResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueStateResultEnvelope<T> {
    pub status: KeyValueStateResultStatus,
    pub data: Option<T>,
    pub error: Option<KeyValueStateError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueStateDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the key-value state contract surface.
pub fn foundation_key_value_state_descriptor_hashes() -> KeyValueStateDescriptorHashes {
    KeyValueStateDescriptorHashes {
        command_schema_hash: key_value_state_stable_hash(&FOUNDATION_KEY_VALUE_STATE_COMMANDS),
        result_schema_hash: key_value_state_stable_hash(&KeyValueStateResultStatus::Success),
        snapshot_schema_hash: key_value_state_stable_hash(&KeyValueStateProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            namespace_hashes: BTreeMap::new(),
            active_watch_count: 0,
        }),
        provider_capability_schema_hash: key_value_state_stable_hash(
            &KeyValueStateProviderCapability {
                provider_class: "unavailable".into(),
                supported_commands: schema_set(FOUNDATION_KEY_VALUE_STATE_COMMANDS),
                supports_ttl: false,
                supports_watch: false,
                supports_snapshot: false,
                supports_compaction: false,
                max_value_bytes: 0,
                max_batch_entries: 0,
                availability: DomainPackProviderCapabilityState::Unavailable,
            },
        ),
        unavailable_schema_hash: key_value_state_stable_hash(&KeyValueStateError {
            code: KeyValueStateResultStatus::Unavailable,
            message: "key-value state provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor compatibility tests.
pub fn key_value_state_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
