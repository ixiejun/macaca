use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityState, DomainPackProviderDescriptor, DomainPackSdkMetadata,
    DomainPackStability,
};

/// Stable pack id for provider-neutral secret references.
pub const FOUNDATION_SECRETS_REFERENCE_PACK_ID: &str = "pack.foundation.secrets.reference.v1";
/// Stable service id used by future secret-reference providers.
pub const FOUNDATION_SECRETS_REFERENCE_SERVICE_ID: &str = "service.foundation.secrets.reference";

/// Canonical command names described by `pack.foundation.secrets.reference.v1`.
///
/// Commands describe references, leases, rotation, and provider injection. They
/// never describe an app-facing raw secret value result.
pub const FOUNDATION_SECRETS_REFERENCE_COMMANDS: &[&str] = &[
    "secrets.create_reference",
    "secrets.import_reference",
    "secrets.inspect_reference",
    "secrets.list_references",
    "secrets.bind_purpose",
    "secrets.resolve_for_provider",
    "secrets.create_lease",
    "secrets.renew_lease",
    "secrets.revoke_lease",
    "secrets.rotate_reference",
    "secrets.version_status",
    "secrets.audit_access",
];

/// Build the descriptor-only catalog entry for secret references.
///
/// The descriptor makes secret-reference semantics discoverable without binding
/// AWS, Vault, Kubernetes, Keychain, or KMS adapters. Concrete resolution must
/// happen inside provider-to-provider injection paths after policy approval.
pub fn foundation_secrets_reference_pack_definition() -> DomainPackDefinition {
    let command_schemas = schema_set(FOUNDATION_SECRETS_REFERENCE_COMMANDS);
    let result_schemas = FOUNDATION_SECRETS_REFERENCE_COMMANDS
        .iter()
        .map(|command| format!("{command}.result.v1"))
        .collect::<BTreeSet<_>>();

    DomainPackDefinition::with_metadata(
        FOUNDATION_SECRETS_REFERENCE_PACK_ID,
        DomainPackMetadata {
            family_id: "foundation".into(),
            parent_pack_id: Some("pack.foundation.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            availability: DomainPackAvailability::PreviewUnavailable,
            service_command_schemas: BTreeMap::from([(
                FOUNDATION_SECRETS_REFERENCE_SERVICE_ID.into(),
                command_schemas,
            )]),
            service_result_schemas: BTreeMap::from([(
                FOUNDATION_SECRETS_REFERENCE_SERVICE_ID.into(),
                result_schemas,
            )]),
            permission_scopes: schema_set(&[
                "secrets.reference.read",
                "secrets.reference.create",
                "secrets.reference.import",
                "secrets.reference.list",
                "secrets.reference.bind",
                "secrets.reference.resolve",
                "secrets.reference.lease",
                "secrets.reference.rotate",
                "secrets.reference.revoke",
                "secrets.reference.audit",
            ]),
            source_attribution: schema_set(&[
                "openspec:add-developer-pack-industrial-capability-catalog",
                "openspec:add-pack-foundation-secrets-reference",
            ]),
            migration_notes: vec![
                "The secrets-reference pack is discoverable as an industrial descriptor and becomes callable only after an approved secret-reference service provider registers.".into(),
                "Raw secret values, provider-private locators, credentials, and private keys must never become app-facing results or observability payloads.".into(),
            ],
            policy_template: DomainPackPolicyTemplate {
                timeout_ms: Some(5_000),
                max_retries: Some(0),
                budget_units: Some(1),
                allow_network: None,
            },
            data_governance: DomainPackDataGovernance {
                classification: "secret_reference_metadata".into(),
                retention_policy: "reference_lease_audit_metadata_only".into(),
                redaction_policy: "raw_secret_values_and_private_locators_redacted".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "sdk.packs.foundation.secrets.reference".into(),
                docs_url: "docs://macaca/developer-packs/foundation/secrets-reference".into(),
                examples: vec![
                    "Declare `pack.foundation.secrets.reference.v1` as optional until a secret-reference provider is installed.".into(),
                    "Use `secrets.resolve_for_provider` only for policy-approved provider injection, never for app-facing raw values.".into(),
                ],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "secrets.audit_access".into(),
                unavailable_reason: "secrets_reference_provider_not_installed".into(),
                replay_schema: "secrets.reference.pack.replay.v1".into(),
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::from([(
                    FOUNDATION_SECRETS_REFERENCE_SERVICE_ID.into(),
                    "^1".into(),
                )]),
            },
            provider_descriptors: secrets_reference_provider_descriptors(),
        },
        [FOUNDATION_SECRETS_REFERENCE_SERVICE_ID.to_string()],
    )
}

fn secrets_reference_provider_descriptors() -> BTreeMap<String, DomainPackProviderDescriptor> {
    [
        provider_descriptor("vault", DomainPackProviderCapabilityState::Preview),
        provider_descriptor("cloud-secrets", DomainPackProviderCapabilityState::Preview),
        provider_descriptor("host-keychain", DomainPackProviderCapabilityState::Preview),
        provider_descriptor(
            "kubernetes-secret",
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
    let capability = SecretsReferenceProviderCapability {
        provider_class: provider_class.into(),
        supported_commands: schema_set(FOUNDATION_SECRETS_REFERENCE_COMMANDS),
        supported_version_states: BTreeSet::from([
            SecretVersionState::Current,
            SecretVersionState::Previous,
            SecretVersionState::Disabled,
            SecretVersionState::Destroyed,
        ]),
        supports_leases: provider_class != "unavailable",
        supports_rotation: matches!(provider_class, "vault" | "cloud-secrets" | "mock"),
        supports_provider_injection: provider_class != "unavailable",
        raw_value_app_results_forbidden: true,
        max_lease_ttl_seconds: 86_400,
        availability,
    };
    DomainPackProviderDescriptor {
        provider_class: provider_class.into(),
        service_id: FOUNDATION_SECRETS_REFERENCE_SERVICE_ID.into(),
        availability,
        capability_hash: secrets_reference_stable_hash(&capability),
        compatibility_hash: "foundation-secrets-reference-provider-v1".into(),
        diagnostics_schema: "secrets.reference.provider.diagnostics.v1".into(),
        metadata: BTreeMap::from([
            ("leases".into(), capability.supports_leases.to_string()),
            ("rotation".into(), capability.supports_rotation.to_string()),
            (
                "provider_injection".into(),
                capability.supports_provider_injection.to_string(),
            ),
            ("max_lease_ttl_seconds".into(), "86400".into()),
        ]),
    }
}

fn schema_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretVersionState {
    Current,
    Previous,
    Disabled,
    Destroyed,
    RotationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretReference {
    pub reference_id: String,
    pub provider_class: String,
    pub version_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretExternalLocator {
    pub provider_class: String,
    pub redacted_locator_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretPurposeBinding {
    pub purpose: String,
    pub service_id: String,
    pub expires_at_epoch_millis: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretAccessPolicy {
    pub allowed_service_ids: BTreeSet<String>,
    pub requires_approval: bool,
    pub max_lease_ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretLeaseReference {
    pub lease_id: String,
    pub reference_id: String,
    pub expires_at_epoch_millis: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretResolutionHandle {
    pub handle_id: String,
    pub service_id: String,
    pub purpose: String,
    pub lease: SecretLeaseReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretVersionStatus {
    pub reference: SecretReference,
    pub state: SecretVersionState,
    pub rotation_due_epoch_millis: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretAuditRecord {
    pub event_id: String,
    pub reference_id: String,
    pub service_id: String,
    pub purpose: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsReferenceProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_version_states: BTreeSet<SecretVersionState>,
    pub supports_leases: bool,
    pub supports_rotation: bool,
    pub supports_provider_injection: bool,
    pub raw_value_app_results_forbidden: bool,
    pub max_lease_ttl_seconds: u64,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsReferenceProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub reference_state_hashes: BTreeMap<String, String>,
    pub lease_state_hashes: BTreeMap<String, String>,
    pub audit_tail_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsCreateReferenceCommand {
    pub reference: SecretReference,
    pub purpose: SecretPurposeBinding,
    pub policy: SecretAccessPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsImportReferenceCommand {
    pub locator: SecretExternalLocator,
    pub purpose: SecretPurposeBinding,
    pub policy: SecretAccessPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsInspectReferenceCommand {
    pub reference: SecretReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsListReferencesCommand {
    pub provider_class: Option<String>,
    pub cursor: Option<String>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsBindPurposeCommand {
    pub reference: SecretReference,
    pub purpose: SecretPurposeBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsResolveForProviderCommand {
    pub reference: SecretReference,
    pub purpose: String,
    pub service_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsCreateLeaseCommand {
    pub reference: SecretReference,
    pub purpose: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsRenewLeaseCommand {
    pub lease: SecretLeaseReference,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsRevokeLeaseCommand {
    pub lease: SecretLeaseReference,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsRotateReferenceCommand {
    pub reference: SecretReference,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsVersionStatusCommand {
    pub reference: SecretReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsAuditAccessCommand {
    pub reference: SecretReference,
    pub since_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretsReferenceResultStatus {
    Success,
    Denied,
    NotFound,
    Disabled,
    Destroyed,
    Expired,
    RotationRequired,
    LeaseExpired,
    InvalidPurpose,
    Unsupported,
    Unavailable,
    ProviderFailure,
    RawSecretForbidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsReferenceError {
    pub code: SecretsReferenceResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsReferenceResultEnvelope<T> {
    pub status: SecretsReferenceResultStatus,
    pub data: Option<T>,
    pub error: Option<SecretsReferenceError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretsReferenceDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the secret-reference contract surface.
pub fn foundation_secrets_reference_descriptor_hashes() -> SecretsReferenceDescriptorHashes {
    SecretsReferenceDescriptorHashes {
        command_schema_hash: secrets_reference_stable_hash(&FOUNDATION_SECRETS_REFERENCE_COMMANDS),
        result_schema_hash: secrets_reference_stable_hash(&SecretsReferenceResultStatus::Success),
        snapshot_schema_hash: secrets_reference_stable_hash(&SecretsReferenceProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            reference_state_hashes: BTreeMap::new(),
            lease_state_hashes: BTreeMap::new(),
            audit_tail_hash: "audit-tail".into(),
        }),
        provider_capability_schema_hash: secrets_reference_stable_hash(
            &SecretsReferenceProviderCapability {
                provider_class: "unavailable".into(),
                supported_commands: schema_set(FOUNDATION_SECRETS_REFERENCE_COMMANDS),
                supported_version_states: BTreeSet::from([SecretVersionState::Disabled]),
                supports_leases: false,
                supports_rotation: false,
                supports_provider_injection: false,
                raw_value_app_results_forbidden: true,
                max_lease_ttl_seconds: 0,
                availability: DomainPackProviderCapabilityState::Unavailable,
            },
        ),
        unavailable_schema_hash: secrets_reference_stable_hash(&SecretsReferenceError {
            code: SecretsReferenceResultStatus::Unavailable,
            message: "secret-reference provider is not installed".into(),
            retryable: false,
        }),
    }
}

/// Compute a deterministic, non-secret hash for descriptor compatibility tests.
pub fn secrets_reference_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = payload.iter().fold(0_u64, |state, byte| {
        state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
    });
    format!("{digest:016x}")
}
