use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for the provider-neutral foundation random capability.
pub const FOUNDATION_RANDOM_PACK_ID: &str = "pack.foundation.random.v1";
/// Stable service id used by future random providers once a provider is installed.
pub const FOUNDATION_RANDOM_SERVICE_ID: &str = "service.foundation.random";

/// Canonical command names described by `pack.foundation.random.v1`.
///
/// These names are descriptor data, not executable routing logic. The pack remains
/// preview-unavailable until a serviceized provider registers concrete handlers through the
/// runtime composition root.
pub const FOUNDATION_RANDOM_COMMANDS: &[&str] = &[
    "random.bytes",
    "random.fill",
    "random.integer",
    "random.uuid_v4",
    "random.nonce",
    "random.token",
    "random.test_stream_create",
    "random.test_stream_bytes",
    "random.entropy_health",
    "random.provider_capabilities",
];

/// Build the descriptor-only catalog entry for `pack.foundation.random.v1`.
///
/// The descriptor contains command/result schemas, policy metadata, provider replacement
/// descriptors, SDK metadata, and diagnostics, but it intentionally stays unavailable. This lets
/// SDKs and admission reports discover the industrial contract without allowing applications to
/// call random commands until an approved random system service provider exists.
pub fn foundation_random_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_RANDOM_COMMANDS);
    let result_schemas = FOUNDATION_RANDOM_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_RANDOM_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_RANDOM_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_RANDOM_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "random.generate",
                "random.identifier",
                "random.token",
                "random.nonce",
                "random.health",
                "random.test_seed",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-random",
            ]),
            migration_notes: vec![
                "The random pack is discoverable as an industrial descriptor and becomes callable only after an approved random system service provider registers.".into(),
                "Production commands must never fall back to insecure pseudo-random providers.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(5_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: Some(false),
            },
            data_governance: DomainPackDataGovernance {
                classification: "secret_derived_output".into(),
                retention_policy: "metadata_only_no_generated_values".into(),
                redaction_policy: "generated_values_seed_and_provider_payload_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.random".into(),
                docs_url: "docs://macaca/developer-packs/foundation/random".into(),
                examples: vec![
                    "Declare `pack.foundation.random.v1` as optional until a random provider is installed.".into(),
                    "Use `random.entropy_health` diagnostics to explain unavailable entropy without logging generated values.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "random.entropy_health".into(),
                unavailable_reason: "random_provider_not_installed".into(),
                replay_schema: "random.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_RANDOM_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: random_provider_descriptors(),
        },
        [FOUNDATION_RANDOM_SERVICE_ID.to_string()],
    )
}

fn random_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor("host-csprng", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "deterministic-test",
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
    let capability = RandomProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_RANDOM_COMMANDS),
        supported_strengths: BTreeSet::from([
            RandomStrengthClass::Cryptographic,
            RandomStrengthClass::StrongWhenAvailable,
            RandomStrengthClass::DeterministicTest,
        ]),
        max_bytes_per_request: 65_536,
        max_token_length: 512,
        supports_bias_free_integer: true,
        supports_uuid_v4: true,
        supports_deterministic_test_streams: provider_class == "deterministic-test"
            || provider_class == "mock",
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_RANDOM_SERVICE_ID.into(),
        availability,
        capability_hash: random_stable_hash(&capability),
        compatibility_hash: "foundation-random-provider-v1".into(),
        diagnostics_schema: "random.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("max_bytes_per_request".into(), "65536".into()),
            ("max_token_length".into(), "512".into()),
            (
                "strength_classes".into(),
                "cryptographic,strong_when_available,deterministic_test".into(),
            ),
            (
                "deterministic_test_streams".into(),
                capability.supports_deterministic_test_streams.to_string(),
            ),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Strength requested by a random command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStrengthClass {
    Cryptographic,
    StrongWhenAvailable,
    DeterministicTest,
}

/// Generic purpose label used for policy, audit, and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomPurpose {
    SessionId,
    Nonce,
    IdempotencyKey,
    TemporaryName,
    TestData,
    ProviderProtocol,
    Generic,
}

/// Token alphabet requested by `random.token`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomAlphabetClass {
    UrlSafe,
    Hex,
    Base64Url,
    Numeric,
    LowercaseAlphaNumeric,
    CustomPolicyBounded,
}

/// Output encoding for byte-like random values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomOutputEncoding {
    RawBytes,
    Hex,
    Base64Url,
    Utf8Token,
}

/// Policy context that controls whether deterministic streams are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomReplayPolicy {
    ProductionDenied,
    TestOnly,
    ReplayOnly,
}

/// Command DTO for `random.bytes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomBytesCommand {
    pub length: u32,
    pub strength: RandomStrengthClass,
    pub purpose: RandomPurpose,
    pub encoding: RandomOutputEncoding,
    pub max_blocking_ms: Option<u64>,
}

/// Command DTO for `random.fill`; the artifact reference stays opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomFillCommand {
    pub artifact_ref: String,
    pub offset: u64,
    pub length: u32,
    pub strength: RandomStrengthClass,
    pub purpose: RandomPurpose,
}

/// Command DTO for bias-free bounded integer generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomIntegerCommand {
    pub min_inclusive: i128,
    pub max_exclusive: i128,
    pub purpose: RandomPurpose,
    pub require_bias_free: bool,
}

/// Command DTO for `random.uuid_v4`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomUuidV4Command {
    pub count: u32,
    pub lowercase: bool,
}

/// Command DTO for nonce generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomNonceCommand {
    pub byte_length: u32,
    pub purpose: RandomPurpose,
    pub encoding: RandomOutputEncoding,
    pub uniqueness_window: Option<String>,
}

/// Command DTO for token generation with bounded alphabet semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomTokenCommand {
    pub char_length: u32,
    pub alphabet: RandomAlphabetClass,
    pub purpose: RandomPurpose,
    pub collision_warning_policy: String,
}

/// Opaque deterministic seed reference. Raw seed material must never enter logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomSeedReference {
    pub seed_ref: String,
    pub replay_binding: String,
}

/// Command DTO for creating an approved deterministic test stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomTestStreamCreateCommand {
    pub seed: RandomSeedReference,
    pub algorithm_id: String,
    pub replay_policy: RandomReplayPolicy,
}

/// Opaque deterministic stream reference with position for replay diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomStreamReference {
    pub stream_id: String,
    pub algorithm_id: String,
    pub position: u64,
    pub replay_binding: String,
}

/// Command DTO for reading deterministic bytes from an approved test stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomTestStreamBytesCommand {
    pub stream: RandomStreamReference,
    pub length: u32,
    pub expected_position: u64,
}

/// Command DTO for entropy health inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomEntropyHealthCommand {
    pub include_blocking_risk: bool,
    pub include_limits: bool,
}

/// Command DTO for provider capability inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomProviderCapabilitiesCommand {
    pub include_preview: bool,
    pub include_unavailable: bool,
}

/// Provider capability report used by discovery, health, and audit snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_strengths: BTreeSet<RandomStrengthClass>,
    pub max_bytes_per_request: u32,
    pub max_token_length: u32,
    pub supports_bias_free_integer: bool,
    pub supports_uuid_v4: bool,
    pub supports_deterministic_test_streams: bool,
    pub availability: DomainPackProviderCapabilityState,
}

/// Entropy health DTO. It deliberately excludes generated values and provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomEntropyHealth {
    pub provider_class: String,
    pub entropy_available: bool,
    pub blocking_risk: bool,
    pub max_bytes_per_request: u32,
    pub unavailable_reason: Option<String>,
}

/// Snapshot DTO retained for replay diagnostics without raw generated data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub health: RandomEntropyHealth,
    pub stream_position_hashes: BTreeMap<String, String>,
}

/// Normalized result status used by every random command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomResultStatus {
    Success,
    Denied,
    InvalidLength,
    InvalidRange,
    InvalidAlphabet,
    Unsupported,
    DeterministicNotAllowed,
    QuotaExceeded,
    EntropyUnavailable,
    Blocked,
    Unavailable,
    ProviderFailure,
}

/// Bounded error DTO that never carries generated values or raw provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomError {
    pub code: RandomResultStatus,
    pub message: String,
    pub retryable: bool,
}

/// Generic result envelope shared by random command outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomResultEnvelope<T> {
    pub status: RandomResultStatus,
    pub data: Option<T>,
    pub error: Option<RandomError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

/// Hash bundle used by SDK discovery and schema compatibility tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub health_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the current random contract schema surface.
pub fn foundation_random_descriptor_hashes() -> RandomDescriptorHashes {
    RandomDescriptorHashes {
        command_schema_hash: random_stable_hash(&FOUNDATION_RANDOM_COMMANDS),
        result_schema_hash: random_stable_hash(&RandomResultStatus::Success),
        health_schema_hash: random_stable_hash(&RandomEntropyHealth {
            provider_class: "schema".into(),
            entropy_available: false,
            blocking_risk: false,
            max_bytes_per_request: 0,
            unavailable_reason: Some("schema".into()),
        }),
        snapshot_schema_hash: random_stable_hash(&RandomProviderSnapshot {
            descriptor_hash: "schema".into(),
            provider_class: "schema".into(),
            health: RandomEntropyHealth {
                provider_class: "schema".into(),
                entropy_available: false,
                blocking_risk: false,
                max_bytes_per_request: 0,
                unavailable_reason: Some("schema".into()),
            },
            stream_position_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: random_stable_hash(&RandomProviderCapability {
            provider_class: "schema".into(),
            supported_commands: schema_set(FOUNDATION_RANDOM_COMMANDS),
            supported_strengths: BTreeSet::from([RandomStrengthClass::Cryptographic]),
            max_bytes_per_request: 0,
            max_token_length: 0,
            supports_bias_free_integer: false,
            supports_uuid_v4: false,
            supports_deterministic_test_streams: false,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: random_stable_hash(&RandomError {
            code: RandomResultStatus::Unavailable,
            message: "schema".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor and schema evidence.
///
/// This helper is not a cryptographic primitive. It mirrors the pack descriptor hash strategy:
/// produce replay-stable identity evidence without adding a crypto dependency or including raw
/// random values, seeds, provider payloads, credentials, or application data.
pub fn random_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
