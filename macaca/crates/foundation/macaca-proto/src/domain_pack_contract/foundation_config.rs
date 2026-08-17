use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for provider-neutral configuration access.
pub const FOUNDATION_CONFIG_PACK_ID: &str = "pack.foundation.config.v1";
/// Stable service id used by future configuration providers.
pub const FOUNDATION_CONFIG_SERVICE_ID: &str = "service.foundation.config";

/// Canonical command names described by `pack.foundation.config.v1`.
///
/// The command list is metadata, not executable routing. The pack remains
/// preview-unavailable until a serviceized configuration provider is installed.
pub const FOUNDATION_CONFIG_COMMANDS: &[&str] = &[
    "config.describe_schema",
    "config.get",
    "config.get_many",
    "config.list_keys",
    "config.resolve_effective",
    "config.validate",
    "config.explain_provenance",
    "config.watch",
    "config.reload",
    "config.snapshot",
    "config.export_redacted",
];

/// Build the descriptor-only catalog entry for `pack.foundation.config.v1`.
///
/// Config is foundation infrastructure, but concrete sources remain replaceable
/// service providers. This descriptor exposes the contract while keeping package
/// descriptors, workspace config, environment variables, tenant config, and
/// remote config behind provider adapters.
pub fn foundation_config_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_CONFIG_COMMANDS);
    let result_schemas = FOUNDATION_CONFIG_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_CONFIG_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_CONFIG_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_CONFIG_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "config.read",
                "config.list",
                "config.validate",
                "config.watch",
                "config.reload",
                "config.snapshot",
                "config.export",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-config",
            ]),
            migration_notes: vec![
                "The config pack is discoverable as an industrial descriptor and becomes callable only after an approved config system service provider registers.".into(),
                "Raw secret values and provider-native config handles must be represented by secret references or redacted artifact references.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(5_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "configuration_metadata".into(),
                retention_policy: "schema_provenance_and_redaction_metadata_only".into(),
                redaction_policy: "raw_values_env_dumps_and_secret_values_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.config".into(),
                docs_url: "docs://macaca/developer-packs/foundation/config".into(),
                examples: vec![
                    "Declare `pack.foundation.config.v1` as optional until a config provider is installed.".into(),
                    "Use `config.export_redacted` for diagnostics instead of logging raw values.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "config.describe_schema".into(),
                unavailable_reason: "config_provider_not_installed".into(),
                replay_schema: "config.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_CONFIG_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: config_provider_descriptors(),
        },
        [FOUNDATION_CONFIG_SERVICE_ID.to_string()],
    )
}

fn config_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor(
            "package-descriptor",
            DomainPackProviderCapabilityState::Preview,
        ),
        provider_descriptor("workspace", DomainPackProviderCapabilityState::Preview),
        provider_descriptor("environment", DomainPackProviderCapabilityState::Preview),
        provider_descriptor("remote", DomainPackProviderCapabilityState::Preview),
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
    let capability = ConfigProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_CONFIG_COMMANDS),
        supported_value_kinds: BTreeSet::from([
            ConfigValueKind::String,
            ConfigValueKind::Number,
            ConfigValueKind::Boolean,
            ConfigValueKind::Json,
            ConfigValueKind::SecretReference,
        ]),
        supports_watch: provider_class != "unavailable",
        supports_reload: matches!(
            provider_class,
            "workspace" | "environment" | "remote" | "mock"
        ),
        supports_redacted_export: true,
        max_keys_per_page: 1_000,
        max_value_bytes: 65_536,
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_CONFIG_SERVICE_ID.into(),
        availability,
        capability_hash: config_stable_hash(&capability),
        compatibility_hash: "foundation-config-provider-v1".into(),
        diagnostics_schema: "config.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("max_keys_per_page".into(), "1000".into()),
            ("max_value_bytes".into(), "65536".into()),
            ("watch".into(), capability.supports_watch.to_string()),
            ("reload".into(), capability.supports_reload.to_string()),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Supported value classes; raw values are carried by provider-owned artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigValueKind {
    String,
    Number,
    Boolean,
    Json,
    SecretReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigKeyReference {
    pub key: String,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSchemaReference {
    pub schema_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigLayerReference {
    pub layer_id: String,
    pub precedence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSelector {
    pub profile: String,
    pub tenant_ref: Option<String>,
    pub environment_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSourceReference {
    pub source_id: String,
    pub provider_class: String,
    pub redacted_location_hash: String,
}

/// Typed value reference; the raw value remains in a provider-owned artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigTypedValueRef {
    pub kind: ConfigValueKind,
    pub value_ref: String,
    pub schema: Option<ConfigSchemaReference>,
    pub secret_reference_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigProvenance {
    pub key: ConfigKeyReference,
    pub winning_layer: ConfigLayerReference,
    pub source: ConfigSourceReference,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigWatchEvent {
    pub event_id: String,
    pub key: ConfigKeyReference,
    pub source_hash: String,
    pub redaction_summary: ConfigRedactionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValidationReport {
    pub schema: ConfigSchemaReference,
    pub valid: bool,
    pub issue_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigRedactionSummary {
    pub redacted_value_count: u32,
    pub redacted_source_count: u32,
    pub contains_secret_references: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_value_kinds: BTreeSet<ConfigValueKind>,
    pub supports_watch: bool,
    pub supports_reload: bool,
    pub supports_redacted_export: bool,
    pub max_keys_per_page: u32,
    pub max_value_bytes: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub source_hashes: BTreeMap<String, String>,
    pub schema_hashes: BTreeMap<String, String>,
    /// Declarative precedence names retained without source paths or values.
    #[serde(default)]
    pub layer_order: Vec<String>,
    /// Bounded result of the last admitted validation pass.
    #[serde(default)]
    pub validation_status: String,
    /// Opaque trace/replay reference used to locate audit evidence.
    #[serde(default)]
    pub replay_ref: String,
    pub redaction_summary: ConfigRedactionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDescribeSchemaCommand {
    pub schema: ConfigSchemaReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigGetCommand {
    pub key: ConfigKeyReference,
    pub selector: ConfigSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigGetManyCommand {
    pub keys: Vec<ConfigKeyReference>,
    pub selector: ConfigSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigListKeysCommand {
    pub namespace: String,
    pub prefix: Option<String>,
    pub page_size: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigResolveEffectiveCommand {
    pub key: ConfigKeyReference,
    pub selector: ConfigSelector,
    pub include_provenance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValidateCommand {
    pub candidate_ref: String,
    pub schema: ConfigSchemaReference,
    pub selector: ConfigSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigExplainProvenanceCommand {
    pub key: ConfigKeyReference,
    pub selector: ConfigSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigWatchCommand {
    pub namespace: String,
    pub selector: ConfigSelector,
    pub start_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigReloadCommand {
    pub source: ConfigSourceReference,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshotCommand {
    pub selector: ConfigSelector,
    pub include_values: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigExportRedactedCommand {
    pub selector: ConfigSelector,
    pub redaction_level: String,
}

/// Normalized result status shared by every config command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigResultStatus {
    Success,
    PartialPage,
    WatchCheckpoint,
    Denied,
    NotFound,
    InvalidKey,
    InvalidSchema,
    ValidationFailed,
    SecretValueForbidden,
    UnavailableSource,
    UnsupportedSelector,
    QuotaExceeded,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigError {
    pub code: ConfigResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigResultEnvelope<T> {
    pub status: ConfigResultStatus,
    pub data: Option<T>,
    pub error: Option<ConfigError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the current config contract schema surface.
pub fn foundation_config_descriptor_hashes() -> ConfigDescriptorHashes {
    let redaction = ConfigRedactionSummary {
        redacted_value_count: 1,
        redacted_source_count: 1,
        contains_secret_references: true,
    };
    ConfigDescriptorHashes {
        command_schema_hash: config_stable_hash(&FOUNDATION_CONFIG_COMMANDS),
        result_schema_hash: config_stable_hash(&ConfigResultStatus::Success),
        snapshot_schema_hash: config_stable_hash(&ConfigProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            source_hashes: BTreeMap::new(),
            schema_hashes: BTreeMap::new(),
            layer_order: Vec::new(),
            validation_status: "not_evaluated".into(),
            replay_ref: "replay:foundation-config:unavailable".into(),
            redaction_summary: redaction,
        }),
        provider_capability_schema_hash: config_stable_hash(&ConfigProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(FOUNDATION_CONFIG_COMMANDS),
            supported_value_kinds: BTreeSet::from([ConfigValueKind::SecretReference]),
            supports_watch: false,
            supports_reload: false,
            supports_redacted_export: true,
            max_keys_per_page: 0,
            max_value_bytes: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: config_stable_hash(&ConfigError {
            code: ConfigResultStatus::Unavailable,
            message: "config provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor compatibility tests.
pub fn config_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
