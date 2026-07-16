use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const COMMERCE_RECEIPT_PACK_ID: &str = "pack.commerce.receipt.v1";
pub const COMMERCE_RECEIPT_SERVICE_ID: &str = "service.commerce.receipt";

pub const COMMERCE_RECEIPT_COMMANDS: &[&str] = &[
    "receipt.inspect_provider",
    "receipt.describe_schema",
    "receipt.plan_issue",
    "receipt.issue_receipt",
    "receipt.plan_reissue",
    "receipt.reissue_receipt",
    "receipt.read_receipt",
    "receipt.search_receipts",
    "receipt.sync_source",
    "receipt.verify_receipt",
    "receipt.plan_delivery",
    "receipt.delivery_request",
    "receipt.get_delivery_status",
    "receipt.link_correction_reference",
    "receipt.list_correction_references",
    "receipt.record_event_reference",
    "receipt.plan_audit_export",
    "receipt.audit_export_request",
    "receipt.get_artifact_handle",
];

const RECEIPT_PERMISSION_SCOPES: &[&str] = &[
    "commerce.receipt.read",
    "commerce.receipt.issue",
    "commerce.receipt.reissue",
    "commerce.receipt.verify",
    "commerce.receipt.deliver",
    "commerce.receipt.correction_reference",
    "commerce.receipt.audit_export",
];

const RECEIPT_RECORD_METADATA: &[(&str, &str)] = &[
    ("source_references", "true"),
    ("issue_reissue", "approval_required"),
    ("verification", "true"),
    ("artifacts", "handle_only"),
];
const RECEIPT_DELIVERY_METADATA: &[(&str, &str)] = &[
    ("delivery", "approval_required"),
    ("external_destination", "reference_only"),
    ("communication_workflow", "false"),
];
const RECEIPT_EVENT_METADATA: &[(&str, &str)] = &[
    ("event_references", "true"),
    ("webhook_body", "false"),
    ("correction_references", "reference_only"),
];
const RECEIPT_MOCK_METADATA: &[(&str, &str)] = &[
    ("receipts", "synthetic"),
    ("delivery", "synthetic"),
    ("callable", "false"),
];
const RECEIPT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("receipts", "false"),
    ("delivery", "false"),
    ("reason", "provider_not_installed"),
];

const RECEIPT_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "receipt-record",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RECEIPT_RECORD_METADATA,
    },
    CommerceProviderClass {
        provider_class: "receipt-delivery",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RECEIPT_DELIVERY_METADATA,
    },
    CommerceProviderClass {
        provider_class: "receipt-event",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RECEIPT_EVENT_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RECEIPT_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: RECEIPT_UNAVAILABLE_METADATA,
    },
];

pub fn commerce_receipt_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_RECEIPT_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-receipt",
        docs_slug: "receipt",
        sdk_slug: "receipt",
        service_id: COMMERCE_RECEIPT_SERVICE_ID,
        commands: COMMERCE_RECEIPT_COMMANDS,
        permission_scopes: RECEIPT_PERMISSION_SCOPES,
        provider_classes: RECEIPT_PROVIDER_CLASSES,
        health_probe: "receipt.inspect_provider",
        unavailable_reason: "commerce_receipt_provider_not_installed",
        replay_schema: "commerce.receipt.replay.v1",
        data_classification: "regulated_receipt_reference_metadata",
        retention_policy: "receipt_source_delivery_verification_correction_event_and_artifact_metadata_by_reference",
        redaction_policy: "buyer_pii_payment_credentials_raw_provider_payloads_webhooks_receipt_bodies_print_data_signatures_and_unbounded_exports_redacted",
        timeout_ms: 120_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.commerce.receipt.v1` as optional until a receipt provider is installed.",
            "Use source references, delivery requests, verification evidence, correction references, and artifact handles instead of raw receipt payloads.",
        ],
        migration_notes: &[
            "Receipt commands become callable only after an approved receipt service provider registers matching schemas.",
            "Payment execution, refunds, invoices, settlement, entitlement provisioning, and communication workflows remain separate capabilities.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptScope {
    pub tenant_scope: String,
    pub merchant_ref: String,
    pub receipt_ref: Option<String>,
    pub source_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptProviderCapability {
    pub provider_class: String,
    pub source_types: BTreeSet<String>,
    pub delivery_channels: BTreeSet<String>,
    pub artifact_formats: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptSourceReference {
    pub source_ref: String,
    pub source_kind: String,
    pub provider_reference_hash: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptRecord {
    pub receipt_ref: String,
    pub source_refs: Vec<ReceiptSourceReference>,
    pub receipt_number_ref: Option<String>,
    pub audience: ReceiptAudience,
    pub variant: ReceiptVariant,
    pub issue_state: String,
    pub issued_at_epoch_ms: Option<u64>,
    pub lines: Vec<ReceiptLine>,
    pub adjustments: Vec<ReceiptAdjustment>,
    pub totals: ReceiptTotals,
    pub delivery_state: ReceiptDeliveryState,
    pub verification_state: Option<String>,
    pub freshness: ReceiptFreshness,
    pub redaction_class: String,
}

impl ReceiptRecord {
    pub fn is_bounded(&self, max_lines: usize, max_sources: usize) -> bool {
        self.lines.len() <= max_lines && self.source_refs.len() <= max_sources
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptLine {
    pub line_ref: String,
    pub item_ref: Option<String>,
    pub description_ref: String,
    pub quantity_micros: i64,
    pub unit_amount_micros: i64,
    pub source_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptAdjustment {
    pub adjustment_ref: String,
    pub adjustment_kind: String,
    pub amount_micros: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptTotals {
    pub subtotal_micros: i64,
    pub discount_micros: i64,
    pub tax_micros: i64,
    pub duty_micros: i64,
    pub fee_micros: i64,
    pub shipping_micros: i64,
    pub gratuity_micros: i64,
    pub total_micros: i64,
    pub currency: String,
    pub currency_precision: u8,
}

impl ReceiptTotals {
    pub fn totals_match(&self) -> bool {
        self.total_micros
            == self.subtotal_micros - self.discount_micros
                + self.tax_micros
                + self.duty_micros
                + self.fee_micros
                + self.shipping_micros
                + self.gratuity_micros
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptAudience {
    pub audience_kind: String,
    pub audience_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptVariant {
    pub variant_kind: String,
    pub provider_variant_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDeliveryRequest {
    pub request_ref: String,
    pub channel: String,
    pub destination_ref: String,
    pub approval_ref: Option<String>,
    pub consent_ref: Option<String>,
    pub idempotency_key_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDeliveryState {
    pub state: String,
    pub attempt_count: u32,
    pub provider_message_ref: Option<String>,
    pub terminal_action_ref: Option<String>,
    pub bounded_failure_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptVerificationResult {
    pub verification_ref: String,
    pub source_linked: bool,
    pub totals_match: bool,
    pub checksum_status: String,
    pub provider_verification_ref: Option<String>,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptCorrectionReference {
    pub correction_ref: String,
    pub correction_kind: String,
    pub source_ref: String,
    pub no_side_effect_payload_marker: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptEventReference {
    pub event_ref: String,
    pub provider_class: String,
    pub event_type: String,
    pub event_timestamp_epoch_ms: u64,
    pub delivery_id_hash: String,
    pub webhook_freshness: ReceiptFreshness,
    pub replay_pointer: String,
    pub bounded_result_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptAuditExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub format: String,
    pub retention_class: String,
    pub redaction_profile: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptArtifactHandle {
    pub artifact_id: String,
    pub artifact_type: String,
    pub hosted_url_metadata_ref: Option<String>,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub redaction_profile: String,
    pub access_policy_ref: String,
    pub replay_pointer: String,
}

define_commerce_command_wrappers!(
    ReceiptInspectProviderCommand,
    ReceiptDescribeSchemaCommand,
    ReceiptPlanIssueCommand,
    ReceiptIssueReceiptCommand,
    ReceiptPlanReissueCommand,
    ReceiptReissueReceiptCommand,
    ReceiptReadReceiptCommand,
    ReceiptSearchReceiptsCommand,
    ReceiptSyncSourceCommand,
    ReceiptVerifyReceiptCommand,
    ReceiptPlanDeliveryCommand,
    ReceiptDeliveryRequestCommand,
    ReceiptGetDeliveryStatusCommand,
    ReceiptLinkCorrectionReferenceCommand,
    ReceiptListCorrectionReferencesCommand,
    ReceiptRecordEventReferenceCommand,
    ReceiptPlanAuditExportCommand,
    ReceiptAuditExportRequestCommand,
    ReceiptGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptResultStatus {
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
    VerificationFailed,
    ArtifactRedacted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptResultEnvelope<T> {
    pub status: ReceiptResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub receipt_hash: String,
    pub totals_hash: String,
    pub delivery_hash: String,
    pub verification_hash: String,
    pub event_hash: String,
    pub artifact_hash: String,
}

pub fn commerce_receipt_descriptor_hashes() -> ReceiptDescriptorHashes {
    let totals = ReceiptTotals {
        subtotal_micros: 1_000_000,
        total_micros: 1_000_000,
        currency: "USD".into(),
        currency_precision: 2,
        ..Default::default()
    };
    let receipt = ReceiptRecord {
        receipt_ref: "receipt".into(),
        source_refs: vec![ReceiptSourceReference {
            source_ref: "payment".into(),
            source_kind: "payment_intent".into(),
            provider_reference_hash: "provider".into(),
            redaction_class: "reference_only".into(),
        }],
        audience: ReceiptAudience {
            audience_kind: "customer".into(),
            ..Default::default()
        },
        variant: ReceiptVariant {
            variant_kind: "hosted".into(),
            ..Default::default()
        },
        totals: totals.clone(),
        freshness: ReceiptFreshness {
            source_timestamp_epoch_ms: 1,
            cache_timestamp_epoch_ms: Some(2),
            freshness_class: "current".into(),
        },
        redaction_class: "receipt_reference_only".into(),
        ..Default::default()
    };
    ReceiptDescriptorHashes {
        command_schema_hash: receipt_stable_hash(&COMMERCE_RECEIPT_COMMANDS),
        result_schema_hash: receipt_stable_hash(&ReceiptResultStatus::Success),
        descriptor_hash: receipt_stable_hash(&commerce_receipt_pack_definition()),
        provider_capability_hash: receipt_stable_hash(&ReceiptProviderCapability {
            provider_class: "mock".into(),
            source_types: BTreeSet::from(["payment_intent".into(), "order".into()]),
            delivery_channels: BTreeSet::from(["email_ref".into(), "hosted".into()]),
            artifact_formats: BTreeSet::from(["pdf_handle".into(), "json_handle".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        receipt_hash: receipt_stable_hash(&receipt),
        totals_hash: receipt_stable_hash(&totals),
        delivery_hash: receipt_stable_hash(&ReceiptDeliveryRequest {
            request_ref: "delivery".into(),
            channel: "hosted".into(),
            destination_ref: "destination".into(),
            idempotency_key_hash: "idem".into(),
            ..Default::default()
        }),
        verification_hash: receipt_stable_hash(&ReceiptVerificationResult {
            verification_ref: "verify".into(),
            source_linked: true,
            totals_match: true,
            checksum_status: "matched".into(),
            replay_pointer: "replay".into(),
            ..Default::default()
        }),
        event_hash: receipt_stable_hash(&ReceiptEventReference {
            event_ref: "event".into(),
            provider_class: "mock".into(),
            event_type: "receipt.updated".into(),
            event_timestamp_epoch_ms: 1,
            delivery_id_hash: "delivery".into(),
            webhook_freshness: ReceiptFreshness {
                source_timestamp_epoch_ms: 1,
                cache_timestamp_epoch_ms: Some(1),
                freshness_class: "fresh".into(),
            },
            replay_pointer: "replay".into(),
            bounded_result_code: "accepted".into(),
        }),
        artifact_hash: receipt_stable_hash(&ReceiptArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_type: "hosted_url".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            redaction_profile: "receipt".into(),
            access_policy_ref: "policy".into(),
            replay_pointer: "replay".into(),
            ..Default::default()
        }),
    }
}

pub fn receipt_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
