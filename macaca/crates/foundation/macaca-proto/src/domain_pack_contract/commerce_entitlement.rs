use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const COMMERCE_ENTITLEMENT_PACK_ID: &str = "pack.commerce.entitlement.v1";
pub const COMMERCE_ENTITLEMENT_SERVICE_ID: &str = "service.commerce.entitlement";

pub const COMMERCE_ENTITLEMENT_COMMANDS: &[&str] = &[
    "entitlement.inspect_provider",
    "entitlement.describe_schema",
    "entitlement.plan_grant",
    "entitlement.grant",
    "entitlement.check",
    "entitlement.batch_check",
    "entitlement.sync_source",
    "entitlement.plan_suspend",
    "entitlement.suspend",
    "entitlement.plan_resume",
    "entitlement.resume",
    "entitlement.plan_revoke",
    "entitlement.revoke",
    "entitlement.plan_transfer",
    "entitlement.transfer",
    "entitlement.assign_seat",
    "entitlement.release_seat",
    "entitlement.record_usage",
    "entitlement.get_usage_balance",
    "entitlement.record_event_reference",
    "entitlement.plan_proof_export",
    "entitlement.proof_export_request",
    "entitlement.get_artifact_handle",
];

const ENTITLEMENT_PERMISSION_SCOPES: &[&str] = &[
    "commerce.entitlement.read",
    "commerce.entitlement.grant",
    "commerce.entitlement.revoke",
    "commerce.entitlement.suspend",
    "commerce.entitlement.transfer",
    "commerce.entitlement.seat",
    "commerce.entitlement.meter",
    "commerce.entitlement.proof_export",
];

const ENTITLEMENT_GRANT_METADATA: &[(&str, &str)] = &[
    ("grants", "true"),
    ("checks", "true"),
    ("source_sync", "true"),
    ("state_transitions", "approval_required"),
];
const ENTITLEMENT_USAGE_METADATA: &[(&str, &str)] = &[
    ("usage_metering", "true"),
    ("seat_assignment", "true"),
    ("batch_check", "bounded"),
    ("application_feature_gate", "false"),
];
const ENTITLEMENT_PROOF_METADATA: &[(&str, &str)] = &[
    ("proof_export", "handle_only"),
    ("event_references", "true"),
    ("signed_payloads", "redacted"),
];
const ENTITLEMENT_MOCK_METADATA: &[(&str, &str)] = &[
    ("grants", "synthetic"),
    ("usage", "synthetic"),
    ("callable", "false"),
];
const ENTITLEMENT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("grants", "false"),
    ("checks", "false"),
    ("reason", "provider_not_installed"),
];

const ENTITLEMENT_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "entitlement-grant",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ENTITLEMENT_GRANT_METADATA,
    },
    CommerceProviderClass {
        provider_class: "entitlement-usage",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ENTITLEMENT_USAGE_METADATA,
    },
    CommerceProviderClass {
        provider_class: "entitlement-proof",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ENTITLEMENT_PROOF_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ENTITLEMENT_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ENTITLEMENT_UNAVAILABLE_METADATA,
    },
];

pub fn commerce_entitlement_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_ENTITLEMENT_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-entitlement",
        docs_slug: "entitlement",
        sdk_slug: "entitlement",
        service_id: COMMERCE_ENTITLEMENT_SERVICE_ID,
        commands: COMMERCE_ENTITLEMENT_COMMANDS,
        permission_scopes: ENTITLEMENT_PERMISSION_SCOPES,
        provider_classes: ENTITLEMENT_PROVIDER_CLASSES,
        health_probe: "entitlement.inspect_provider",
        unavailable_reason: "commerce_entitlement_provider_not_installed",
        replay_schema: "commerce.entitlement.replay.v1",
        data_classification: "regulated_entitlement_reference_metadata",
        retention_policy: "entitlement_grant_source_usage_seat_event_and_proof_artifact_metadata_by_reference",
        redaction_policy: "purchase_tokens_signed_payloads_payment_credentials_webhooks_license_secrets_private_keys_signatures_provider_payloads_and_unbounded_exports_redacted",
        timeout_ms: 120_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.commerce.entitlement.v1` as optional until an entitlement provider is installed.",
            "Use subject/resource references, source evidence, usage records, event references, and proof handles instead of provider-native entitlement payloads.",
        ],
        migration_notes: &[
            "Entitlement commands become callable only after an approved entitlement service provider registers matching schemas.",
            "Billing, payment, refunds, invoices, receipts, pricing rules, checkout flows, and application-specific feature gates remain separate boundaries.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementScope {
    pub tenant_scope: String,
    pub provider_scope_ref: String,
    pub subject_ref: Option<String>,
    pub resource_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementProviderCapability {
    pub provider_class: String,
    pub source_types: BTreeSet<String>,
    pub states: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub event_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementSubject {
    pub subject_ref: String,
    pub subject_kind: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementResource {
    pub resource_ref: String,
    pub resource_kind: String,
    pub external_resource_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementDimension {
    pub dimension_ref: String,
    pub dimension_kind: String,
    pub unit: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementSourceEvidence {
    pub source_ref: String,
    pub source_kind: String,
    pub authority_ref: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementGrant {
    pub grant_ref: String,
    pub subject: EntitlementSubject,
    pub resource: EntitlementResource,
    pub dimensions: Vec<EntitlementDimension>,
    pub state: CommerceEntitlementState,
    pub valid_from_epoch_ms: Option<u64>,
    pub valid_until_epoch_ms: Option<u64>,
    pub quantity: i64,
    pub usage_balance: Option<EntitlementUsageBalance>,
    pub source_evidence: Vec<EntitlementSourceEvidence>,
    pub grant_reason_ref: Option<String>,
    pub suspension_reason_ref: Option<String>,
    pub revocation_reason_ref: Option<String>,
    pub transfer_history_refs: Vec<String>,
    pub freshness: EntitlementFreshness,
    pub redaction_class: String,
}

impl EntitlementGrant {
    pub fn is_active_at(&self, epoch_ms: u64) -> bool {
        self.state.state == "active"
            && self
                .valid_from_epoch_ms
                .is_none_or(|start| start <= epoch_ms)
            && self.valid_until_epoch_ms.is_none_or(|end| epoch_ms <= end)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommerceEntitlementState {
    pub state: String,
    pub provider_state_ref: Option<String>,
    pub mapping_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementSeatAssignment {
    pub assignment_ref: String,
    pub seat_pool_ref: String,
    pub assignee_ref: String,
    pub quantity: i64,
    pub role_ref: Option<String>,
    pub assignment_state: String,
    pub release_state: Option<String>,
    pub audit_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementUsageRecord {
    pub usage_ref: String,
    pub dimension: EntitlementDimension,
    pub quantity: i64,
    pub usage_window_ref: Option<String>,
    pub idempotency_key_hash: String,
    pub source_evidence_ref: String,
    pub freshness: Option<EntitlementFreshness>,
    pub conflict_metadata_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementUsageBalance {
    pub dimension_ref: String,
    pub balance: i64,
    pub limit: Option<i64>,
    pub reset_policy_ref: Option<String>,
    pub source_evidence_ref: Option<String>,
    pub conflict_metadata_ref: Option<String>,
    pub freshness: Option<EntitlementFreshness>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementEventReference {
    pub event_ref: String,
    pub provider_class: String,
    pub event_type: String,
    pub event_timestamp_epoch_ms: u64,
    pub delivery_id_hash: String,
    pub webhook_freshness: EntitlementFreshness,
    pub replay_pointer: String,
    pub bounded_result_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementProofExportPlan {
    pub export_ref: String,
    pub proof_type: String,
    pub scope_ref: String,
    pub retention_class: String,
    pub redaction_profile: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementArtifactHandle {
    pub artifact_id: String,
    pub proof_type: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub redaction_profile: String,
    pub access_policy_ref: String,
    pub replay_pointer: String,
}

define_commerce_command_wrappers!(
    CommerceEntitlementInspectProviderCommand,
    CommerceEntitlementDescribeSchemaCommand,
    CommerceEntitlementPlanGrantCommand,
    CommerceEntitlementGrantCommand,
    CommerceEntitlementCheckCommand,
    CommerceEntitlementBatchCheckCommand,
    CommerceEntitlementSyncSourceCommand,
    CommerceEntitlementPlanSuspendCommand,
    CommerceEntitlementSuspendCommand,
    CommerceEntitlementPlanResumeCommand,
    CommerceEntitlementResumeCommand,
    CommerceEntitlementPlanRevokeCommand,
    CommerceEntitlementRevokeCommand,
    CommerceEntitlementPlanTransferCommand,
    CommerceEntitlementTransferCommand,
    CommerceEntitlementAssignSeatCommand,
    CommerceEntitlementReleaseSeatCommand,
    CommerceEntitlementRecordUsageCommand,
    CommerceEntitlementGetUsageBalanceCommand,
    CommerceEntitlementRecordEventReferenceCommand,
    CommerceEntitlementPlanProofExportCommand,
    CommerceEntitlementProofExportRequestCommand,
    CommerceEntitlementGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementResultStatus {
    Success,
    Paged,
    Partial,
    Accepted,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    ApprovalRequired,
    SourceAuthorityDenied,
    ProofRedacted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementResultEnvelope<T> {
    pub status: EntitlementResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub grant_hash: String,
    pub seat_hash: String,
    pub usage_hash: String,
    pub event_hash: String,
    pub artifact_hash: String,
}

pub fn commerce_entitlement_descriptor_hashes() -> EntitlementDescriptorHashes {
    let subject = EntitlementSubject {
        subject_ref: "subject".into(),
        subject_kind: "account".into(),
        redaction_class: "reference_only".into(),
    };
    let resource = EntitlementResource {
        resource_ref: "resource".into(),
        resource_kind: "feature".into(),
        external_resource_ref: None,
    };
    let dimension = EntitlementDimension {
        dimension_ref: "seat".into(),
        dimension_kind: "seats".into(),
        unit: "count".into(),
    };
    EntitlementDescriptorHashes {
        command_schema_hash: entitlement_stable_hash(&COMMERCE_ENTITLEMENT_COMMANDS),
        result_schema_hash: entitlement_stable_hash(&EntitlementResultStatus::Success),
        descriptor_hash: entitlement_stable_hash(&commerce_entitlement_pack_definition()),
        provider_capability_hash: entitlement_stable_hash(&EntitlementProviderCapability {
            provider_class: "mock".into(),
            source_types: BTreeSet::from(["order".into(), "receipt".into()]),
            states: BTreeSet::from(["active".into(), "revoked".into()]),
            feature_flags: BTreeSet::from(["usage".into(), "seat".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        grant_hash: entitlement_stable_hash(&EntitlementGrant {
            grant_ref: "grant".into(),
            subject,
            resource,
            dimensions: vec![dimension.clone()],
            state: CommerceEntitlementState {
                state: "active".into(),
                ..Default::default()
            },
            valid_from_epoch_ms: Some(1),
            valid_until_epoch_ms: Some(10),
            quantity: 1,
            grant_reason_ref: Some("purchase".into()),
            freshness: EntitlementFreshness {
                source_timestamp_epoch_ms: 1,
                event_timestamp_epoch_ms: Some(2),
                freshness_class: "current".into(),
            },
            redaction_class: "entitlement_reference_only".into(),
            ..Default::default()
        }),
        seat_hash: entitlement_stable_hash(&EntitlementSeatAssignment {
            assignment_ref: "assignment".into(),
            seat_pool_ref: "pool".into(),
            assignee_ref: "subject".into(),
            quantity: 1,
            assignment_state: "assigned".into(),
            audit_evidence_ref: Some("evidence".into()),
            ..Default::default()
        }),
        usage_hash: entitlement_stable_hash(&EntitlementUsageRecord {
            usage_ref: "usage".into(),
            dimension,
            quantity: 1,
            usage_window_ref: Some("window".into()),
            idempotency_key_hash: "idem".into(),
            source_evidence_ref: "source".into(),
            conflict_metadata_ref: None,
            freshness: Some(EntitlementFreshness {
                source_timestamp_epoch_ms: 1,
                event_timestamp_epoch_ms: None,
                freshness_class: "fresh".into(),
            }),
        }),
        event_hash: entitlement_stable_hash(&EntitlementEventReference {
            event_ref: "event".into(),
            provider_class: "mock".into(),
            event_type: "grant.updated".into(),
            event_timestamp_epoch_ms: 1,
            delivery_id_hash: "delivery".into(),
            webhook_freshness: EntitlementFreshness {
                source_timestamp_epoch_ms: 1,
                event_timestamp_epoch_ms: Some(1),
                freshness_class: "fresh".into(),
            },
            replay_pointer: "replay".into(),
            bounded_result_code: "accepted".into(),
        }),
        artifact_hash: entitlement_stable_hash(&EntitlementArtifactHandle {
            artifact_id: "artifact".into(),
            proof_type: "json".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            redaction_profile: "entitlement".into(),
            access_policy_ref: "policy".into(),
            replay_pointer: "replay".into(),
        }),
    }
}

pub fn entitlement_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
