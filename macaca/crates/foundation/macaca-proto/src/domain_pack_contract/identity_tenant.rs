use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::identity_common::{
    define_identity_command_wrappers, identity_pack_definition, identity_stable_hash,
    IdentityPackCommandEnvelope, IdentityPackDescriptor, IdentityPackError, IdentityPackPage,
    IdentityProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const IDENTITY_TENANT_PACK_ID: &str = "pack.identity.tenant.v1";
pub const IDENTITY_TENANT_SERVICE_ID: &str = "service.identity.tenant";

pub const IDENTITY_TENANT_COMMANDS: &[&str] = &[
    "tenant.inspect_provider",
    "tenant.discover_schema",
    "tenant.plan_create",
    "tenant.create",
    "tenant.get",
    "tenant.search",
    "tenant.plan_update",
    "tenant.update",
    "tenant.plan_lifecycle_transition",
    "tenant.request_lifecycle_transition",
    "tenant.inspect_isolation_policy",
    "tenant.plan_policy_attachment",
    "tenant.request_policy_attachment",
    "tenant.inspect_quota",
    "tenant.plan_quota_reservation",
    "tenant.request_quota_reservation",
    "tenant.snapshot_usage",
    "tenant.inspect_residency",
    "tenant.inspect_config",
    "tenant.update_config_reference",
    "tenant.inspect_relationships",
    "tenant.export_audit",
    "tenant.get_artifact",
];

const TENANT_PERMISSION_SCOPES: &[&str] = &[
    "identity.tenant.read",
    "identity.tenant.search",
    "identity.tenant.write",
    "identity.tenant.lifecycle",
    "identity.tenant.policy.read",
    "identity.tenant.policy.write",
    "identity.tenant.quota.read",
    "identity.tenant.quota.reserve",
    "identity.tenant.usage.read",
    "identity.tenant.residency.read",
    "identity.tenant.config.read",
    "identity.tenant.config.write",
    "identity.tenant.relationship.read",
    "identity.tenant.audit.export",
    "identity.tenant.artifact.read",
];

const TENANT_RECORD_METADATA: &[(&str, &str)] = &[
    ("records", "true"),
    ("relationships", "reference_only"),
    ("lifecycle", "planned"),
];
const TENANT_POLICY_METADATA: &[(&str, &str)] = &[
    ("policy_references", "true"),
    ("policy_engine", "false"),
    ("approval", "required_for_high_impact"),
];
const TENANT_QUOTA_METADATA: &[(&str, &str)] = &[
    ("quota_envelopes", "true"),
    ("usage_snapshots", "bounded"),
    ("billing_entitlement", "false"),
];
const TENANT_CONFIG_METADATA: &[(&str, &str)] = &[
    ("config_references", "true"),
    ("secret_values", "false"),
    ("residency_hints", "true"),
];
const TENANT_MOCK_METADATA: &[(&str, &str)] = &[
    ("tenants", "synthetic"),
    ("quotas", "synthetic"),
    ("callable", "false"),
];
const TENANT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("tenants", "false"),
    ("quota", "false"),
    ("reason", "provider_not_installed"),
];

const TENANT_PROVIDER_CLASSES: &[IdentityProviderClass<'_>] = &[
    IdentityProviderClass {
        provider_class: "tenant-record",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TENANT_RECORD_METADATA,
    },
    IdentityProviderClass {
        provider_class: "tenant-policy",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TENANT_POLICY_METADATA,
    },
    IdentityProviderClass {
        provider_class: "tenant-quota",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TENANT_QUOTA_METADATA,
    },
    IdentityProviderClass {
        provider_class: "tenant-config",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TENANT_CONFIG_METADATA,
    },
    IdentityProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TENANT_MOCK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: TENANT_UNAVAILABLE_METADATA,
    },
];

/// Build the tenant descriptor without binding cloud, quota, billing, or policy engines.
pub fn identity_tenant_pack_definition() -> DomainPackDefinition {
    identity_pack_definition(IdentityPackDescriptor {
        pack_id: IDENTITY_TENANT_PACK_ID,
        child_change_id: "openspec:add-pack-identity-tenant",
        docs_slug: "tenant",
        sdk_slug: "tenant",
        service_id: IDENTITY_TENANT_SERVICE_ID,
        commands: IDENTITY_TENANT_COMMANDS,
        permission_scopes: TENANT_PERMISSION_SCOPES,
        provider_classes: TENANT_PROVIDER_CLASSES,
        health_probe: "tenant.inspect_provider",
        unavailable_reason: "identity_tenant_provider_not_installed",
        replay_schema: "identity.tenant.replay.v1",
        data_classification: "identity_tenant_reference_metadata",
        retention_policy: "tenant_lifecycle_policy_quota_usage_residency_config_relationship_and_audit_metadata_by_reference",
        redaction_policy: "raw_credentials_client_secrets_tokens_private_keys_provider_payloads_manifests_package_bytes_raw_audit_exports_full_usage_exports_and_unbounded_tenant_lists_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.identity.tenant.v1` as optional until a tenant provider is installed.",
            "Use tenant handles, policy references, quota envelopes, config references, and audit artifact handles instead of cloud or directory-native payloads.",
        ],
        migration_notes: &[
            "Tenant commands become callable only after an approved tenant service provider registers matching schemas.",
            "Account lifecycle, profile data, auth handoff, organization membership, billing entitlement, payments, cloud provisioning, and application multitenancy workflows remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantScope {
    pub tenant_scope: String,
    pub provider_scope: String,
    pub parent_relationship_ref: Option<String>,
    pub caller_subject_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub supported_lifecycle_states: BTreeSet<String>,
    pub quota_dimensions: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantRecord {
    pub tenant_ref: String,
    pub display_label_ref: String,
    pub identifiers: Vec<TenantIdentifier>,
    pub lifecycle_state: TenantLifecycleState,
    pub isolation_policy_refs: Vec<TenantIsolationPolicyReference>,
    pub quota_refs: Vec<TenantQuotaEnvelope>,
    pub residency_hints: Vec<TenantResidencyHint>,
    pub config_refs: Vec<TenantConfigReference>,
    pub relationship_refs: Vec<TenantRelationshipReference>,
    pub version_token_hash: String,
    pub freshness: TenantFreshness,
    pub audit_refs: Vec<TenantAuditReference>,
}

impl TenantRecord {
    /// Bound tenant projections before they are used in diagnostics or snapshots.
    pub fn is_bounded(&self, max_identifiers: usize, max_relationships: usize) -> bool {
        self.identifiers.len() <= max_identifiers
            && self.relationship_refs.len() <= max_relationships
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantIdentifier {
    pub identifier_ref: String,
    pub identifier_kind: String,
    pub uniqueness_scope: String,
    pub verification_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantLifecycleState {
    pub state: String,
    pub provider_state_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantIsolationPolicyReference {
    pub policy_ref: String,
    pub policy_type: String,
    pub decision_freshness_ref: Option<String>,
    pub attachment_state: String,
    pub data_boundary_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantQuotaEnvelope {
    pub quota_ref: String,
    pub dimension: String,
    pub hard_limit: u64,
    pub soft_limit: Option<u64>,
    pub burst_limit: Option<u64>,
    pub reservation_state: String,
    pub budget_ref: Option<String>,
    pub enforcement_mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantUsageSnapshot {
    pub snapshot_ref: String,
    pub counters: BTreeMap<String, u64>,
    pub measured_from_epoch_ms: u64,
    pub measured_to_epoch_ms: u64,
    pub freshness: TenantFreshness,
    pub redaction_profile: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantResidencyHint {
    pub residency_ref: String,
    pub allowed_regions: BTreeSet<String>,
    pub preferred_regions: BTreeSet<String>,
    pub restricted_regions: BTreeSet<String>,
    pub data_boundary_policy_ref: Option<String>,
    pub freshness: TenantFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantConfigReference {
    pub config_ref: String,
    pub config_kind: String,
    pub external_ref: Option<String>,
    pub secret_ref: Option<String>,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantRelationshipReference {
    pub relationship_ref: String,
    pub relationship_kind: String,
    pub related_ref: String,
    pub provider_class: String,
    pub version_token_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantAuditReference {
    pub event_ref: String,
    pub event_type: String,
    pub bounded_reason_code: String,
    pub provider_cursor_ref: Option<String>,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_identity_command_wrappers!(
    TenantInspectProviderCommand,
    TenantDiscoverSchemaCommand,
    TenantPlanCreateCommand,
    TenantCreateCommand,
    TenantGetCommand,
    TenantSearchCommand,
    TenantPlanUpdateCommand,
    TenantUpdateCommand,
    TenantPlanLifecycleTransitionCommand,
    TenantRequestLifecycleTransitionCommand,
    TenantInspectIsolationPolicyCommand,
    TenantPlanPolicyAttachmentCommand,
    TenantRequestPolicyAttachmentCommand,
    TenantInspectQuotaCommand,
    TenantPlanQuotaReservationCommand,
    TenantRequestQuotaReservationCommand,
    TenantSnapshotUsageCommand,
    TenantInspectResidencyCommand,
    TenantInspectConfigCommand,
    TenantUpdateConfigReferenceCommand,
    TenantInspectRelationshipsCommand,
    TenantExportAuditCommand,
    TenantGetArtifactCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantResultStatus {
    Success,
    Paged,
    Partial,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    QuotaExceeded,
    RateLimited,
    Timeout,
    Cancelled,
    SecretReferenceDenied,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantResultEnvelope<T> {
    pub status: TenantResultStatus,
    pub data: Option<T>,
    pub page: Option<IdentityPackPage<T>>,
    pub error: Option<IdentityPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub schema_version_compatibility_hash: String,
    pub command_availability_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub tenant_hash: String,
    pub quota_hash: String,
    pub quota_envelope_hash: String,
    pub usage_hash: String,
    pub config_reference_hash: String,
    pub artifact_hash: String,
    pub policy_template_hash: String,
    pub redaction_profile_hash: String,
}

pub fn identity_tenant_descriptor_hashes() -> TenantDescriptorHashes {
    let freshness = TenantFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let quota = TenantQuotaEnvelope {
        quota_ref: "quota".into(),
        dimension: "sessions".into(),
        hard_limit: 100,
        reservation_state: "available".into(),
        enforcement_mode: "hard".into(),
        ..Default::default()
    };
    TenantDescriptorHashes {
        command_schema_hash: tenant_stable_hash(&IDENTITY_TENANT_COMMANDS),
        result_schema_hash: tenant_stable_hash(&TenantResultStatus::Success),
        schema_version_compatibility_hash: tenant_stable_hash(&(
            &identity_tenant_pack_definition().metadata.version,
            &identity_tenant_pack_definition().metadata.compatibility,
        )),
        command_availability_hash: tenant_stable_hash(&(
            IDENTITY_TENANT_SERVICE_ID,
            IDENTITY_TENANT_COMMANDS,
            "preview_unavailable_until_provider_registered",
        )),
        descriptor_hash: tenant_stable_hash(&identity_tenant_pack_definition()),
        provider_capability_hash: tenant_stable_hash(&TenantProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["quota".into(), "policy".into()]),
            supported_lifecycle_states: BTreeSet::from(["active".into(), "suspended".into()]),
            quota_dimensions: BTreeSet::from(["sessions".into()]),
            limits: BTreeMap::from([("max_tenants".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        tenant_hash: tenant_stable_hash(&TenantRecord {
            tenant_ref: "tenant".into(),
            display_label_ref: "display".into(),
            identifiers: vec![TenantIdentifier {
                identifier_ref: "slug".into(),
                identifier_kind: "slug".into(),
                uniqueness_scope: "global".into(),
                verification_state: "verified".into(),
            }],
            lifecycle_state: TenantLifecycleState {
                state: "active".into(),
                ..Default::default()
            },
            quota_refs: vec![quota.clone()],
            version_token_hash: "version".into(),
            freshness: freshness.clone(),
            ..Default::default()
        }),
        quota_hash: tenant_stable_hash(&quota),
        quota_envelope_hash: tenant_stable_hash(&quota),
        usage_hash: tenant_stable_hash(&TenantUsageSnapshot {
            snapshot_ref: "usage".into(),
            counters: BTreeMap::from([("sessions".into(), 5)]),
            measured_from_epoch_ms: 1,
            measured_to_epoch_ms: 2,
            freshness,
            redaction_profile: "bounded_counters".into(),
            confidence: "observed".into(),
        }),
        config_reference_hash: tenant_stable_hash(&TenantConfigReference {
            config_ref: "config".into(),
            config_kind: "region_policy".into(),
            external_ref: Some("config:tenant".into()),
            secret_ref: None,
            redaction_class: "handle_only".into(),
        }),
        artifact_hash: tenant_stable_hash(&TenantArtifactHandle {
            artifact_id: "artifact".into(),
            content_class: "audit_export".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
        policy_template_hash: tenant_stable_hash(
            &identity_tenant_pack_definition().metadata.policy_template,
        ),
        redaction_profile_hash: tenant_stable_hash(
            &identity_tenant_pack_definition()
                .metadata
                .data_governance
                .redaction_policy,
        ),
    }
}

pub fn tenant_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    identity_stable_hash(value)
}
