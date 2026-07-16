use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, finance_pack_definition, finance_stable_hash,
    FinanceCommandEnvelope, FinanceError, FinancePackDescriptor, FinancePage, FinanceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const FINANCE_INVOICE_PACK_ID: &str = "pack.finance.invoice.v1";
pub const FINANCE_INVOICE_SERVICE_ID: &str = "service.finance.invoice";
pub const FINANCE_INVOICE_COMMANDS: &[&str] = &[
    "invoice.inspect_provider",
    "invoice.describe_schema",
    "invoice.list_parties",
    "invoice.list_items",
    "invoice.plan_invoice",
    "invoice.create_draft",
    "invoice.list_invoices",
    "invoice.read_invoice",
    "invoice.plan_issue",
    "invoice.issue_invoice",
    "invoice.plan_delivery",
    "invoice.send_invoice",
    "invoice.sync_payment_status",
    "invoice.plan_reminder",
    "invoice.send_reminder",
    "invoice.plan_void",
    "invoice.void_invoice",
    "invoice.plan_export",
    "invoice.export_invoice",
    "invoice.get_artifact_handle",
];

const INVOICE_PERMISSION_SCOPES: &[&str] = &[
    "finance.invoice.read",
    "finance.invoice.write",
    "finance.invoice.issue",
    "finance.invoice.deliver",
    "finance.invoice.remind",
    "finance.invoice.export",
];

const INVOICE_LIFECYCLE_METADATA: &[(&str, &str)] = &[
    ("drafts", "true"),
    ("issue", "approval_required"),
    ("void", "approval_required"),
    ("payment_status", "sync_only"),
];
const INVOICE_DELIVERY_METADATA: &[(&str, &str)] = &[
    ("delivery", "approval_required"),
    ("reminders", "optional"),
    ("recipient_policy", "required"),
];
const INVOICE_EXPORT_METADATA: &[(&str, &str)] = &[
    ("pdf", "optional"),
    ("json", "true"),
    ("retention", "approval_required"),
];
const INVOICE_MOCK_METADATA: &[(&str, &str)] = &[
    ("parties", "synthetic"),
    ("invoices", "synthetic"),
    ("callable", "false"),
];
const INVOICE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("drafts", "false"),
    ("delivery", "false"),
    ("exports", "false"),
    ("reason", "provider_not_installed"),
];

const INVOICE_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "invoice-lifecycle",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INVOICE_LIFECYCLE_METADATA,
    },
    FinanceProviderClass {
        provider_class: "invoice-delivery",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INVOICE_DELIVERY_METADATA,
    },
    FinanceProviderClass {
        provider_class: "invoice-export",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INVOICE_EXPORT_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INVOICE_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: INVOICE_UNAVAILABLE_METADATA,
    },
];

/// Build the invoice descriptor without binding billing workflows or payment processors.
pub fn finance_invoice_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_INVOICE_PACK_ID,
        child_change_id: "openspec:add-pack-finance-invoice",
        docs_slug: "invoice",
        sdk_slug: "invoice",
        service_id: FINANCE_INVOICE_SERVICE_ID,
        commands: FINANCE_INVOICE_COMMANDS,
        permission_scopes: INVOICE_PERMISSION_SCOPES,
        provider_classes: INVOICE_PROVIDER_CLASSES,
        health_probe: "invoice.inspect_provider",
        unavailable_reason: "finance_invoice_provider_not_installed",
        replay_schema: "finance.invoice.replay.v1",
        data_classification: "regulated_invoice_reference_metadata",
        retention_policy: "party_item_invoice_lifecycle_delivery_export_and_artifact_metadata_by_reference",
        redaction_policy: "raw_pii_payment_credentials_tax_identifiers_hosted_urls_invoice_pdfs_provider_payloads_and_unbounded_lines_redacted",
        timeout_ms: 120_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.finance.invoice.v1` as optional until an invoice provider is installed.",
            "Plan invoice lifecycle and delivery side effects before requesting provider mutation.",
        ],
        migration_notes: &[
            "Invoice commands become callable only after an approved provider registers matching schemas.",
            "Payment processing, settlement, tax filing, revenue recognition, subscription billing, and collections strategy remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceScope {
    pub tenant_scope: String,
    pub entity_ref: String,
    pub recipient_policy_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceProviderCapability {
    pub provider_class: String,
    pub lifecycle_transitions: BTreeSet<String>,
    pub delivery_channels: BTreeSet<String>,
    pub export_formats: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceAttribution {
    pub source_ref: String,
    pub provider_class: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
    pub export_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoicePartyReference {
    pub party_ref: String,
    pub party_kind: String,
    pub display_name_ref: String,
    pub tax_identifier_ref: Option<TaxIdentifierReference>,
    pub billing_address_ref: Option<String>,
    pub shipping_address_ref: Option<String>,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxIdentifierReference {
    pub tax_ref: String,
    pub jurisdiction_ref: String,
    pub masked_value_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceItemReference {
    pub item_ref: String,
    pub sku_ref: String,
    pub description_ref: String,
    pub revenue_category_ref: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub line_ref: String,
    pub item_ref: String,
    pub quantity_micros: i64,
    pub unit_price_micros: i64,
    pub currency: String,
    pub service_period_start: Option<String>,
    pub service_period_end: Option<String>,
    pub tax: Option<InvoiceTaxReference>,
    pub discount: Option<InvoiceDiscount>,
    pub adjustment: Option<InvoiceAdjustment>,
    pub rounding_evidence_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTaxReference {
    pub tax_code_ref: String,
    pub jurisdiction_ref: String,
    pub amount_micros: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDiscount {
    pub discount_ref: String,
    pub amount_micros: i64,
    pub reason_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceAdjustment {
    pub adjustment_ref: String,
    pub amount_micros: i64,
    pub reason_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTotals {
    pub subtotal_micros: i64,
    pub tax_total_micros: i64,
    pub discount_total_micros: i64,
    pub amount_due_micros: i64,
    pub amount_paid_micros: i64,
    pub amount_remaining_micros: i64,
    pub currency: String,
    pub precision: u8,
    pub validation: Vec<InvoiceValidationDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceValidationDiagnostic {
    pub code: String,
    pub trace_safe_detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDraftPlan {
    pub plan_ref: String,
    pub seller: InvoicePartyReference,
    pub buyer: InvoicePartyReference,
    pub lines: Vec<InvoiceLine>,
    pub totals: InvoiceTotals,
    pub idempotency_key: String,
}

impl InvoiceDraftPlan {
    /// Validate totals over bounded synthetic DTOs without executing provider mutations.
    pub fn totals_match(&self) -> bool {
        let subtotal = self
            .lines
            .iter()
            .map(|line| line.quantity_micros * line.unit_price_micros / 1_000_000)
            .sum::<i64>();
        subtotal == self.totals.subtotal_micros
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceRecord {
    pub invoice_ref: String,
    pub lifecycle: InvoiceLifecycleState,
    pub delivery: InvoiceDeliveryState,
    pub payment_status: InvoicePaymentStatus,
    pub concurrency: InvoiceConcurrencyToken,
    pub lifecycle_evidence: LifecycleEvidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLifecycleState {
    pub state: String,
    pub issued_at_epoch_ms: Option<u64>,
    pub voided_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDeliveryState {
    pub state: String,
    pub channel: Option<String>,
    pub recipient_policy_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoicePaymentStatus {
    pub state: String,
    pub amount_paid_micros: i64,
    pub synced_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceConcurrencyToken {
    pub token_hash: String,
    pub provider_version_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleEvidence {
    pub evidence_ref: String,
    pub provider_trace_ref: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceReminderPlan {
    pub plan_ref: String,
    pub invoice_ref: String,
    pub channel: String,
    pub cadence_ref: String,
    pub eligible: bool,
    pub recipient_policy_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceReminderResult {
    pub result_state: String,
    pub delivery_state: InvoiceDeliveryState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceArtifactPlan {
    pub plan_ref: String,
    pub invoice_ref: String,
    pub export_format: String,
    pub retention_policy: String,
    pub redaction: InvoiceRedactionPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub access_policy: String,
}

define_finance_command_wrappers!(
    InvoiceInspectProviderCommand,
    InvoiceDescribeSchemaCommand,
    InvoiceListPartiesCommand,
    InvoiceListItemsCommand,
    InvoicePlanInvoiceCommand,
    InvoiceCreateDraftCommand,
    InvoiceListInvoicesCommand,
    InvoiceReadInvoiceCommand,
    InvoicePlanIssueCommand,
    InvoiceIssueInvoiceCommand,
    InvoicePlanDeliveryCommand,
    InvoiceSendInvoiceCommand,
    InvoiceSyncPaymentStatusCommand,
    InvoicePlanReminderCommand,
    InvoiceSendReminderCommand,
    InvoicePlanVoidCommand,
    InvoiceVoidInvoiceCommand,
    InvoicePlanExportCommand,
    InvoiceExportInvoiceCommand,
    InvoiceGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceResultEnvelope<T> {
    pub status: InvoiceResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub party_hash: String,
    pub line_hash: String,
    pub totals_hash: String,
    pub record_hash: String,
    pub reminder_hash: String,
    pub artifact_hash: String,
}

pub fn finance_invoice_descriptor_hashes() -> InvoiceDescriptorHashes {
    InvoiceDescriptorHashes {
        command_schema_hash: invoice_stable_hash(&FINANCE_INVOICE_COMMANDS),
        result_schema_hash: invoice_stable_hash(&InvoiceResultStatus::Success),
        descriptor_hash: invoice_stable_hash(&finance_invoice_pack_definition()),
        provider_capability_hash: invoice_stable_hash(&InvoiceProviderCapability {
            provider_class: "mock".into(),
            lifecycle_transitions: BTreeSet::from(["draft".into(), "issued".into()]),
            delivery_channels: BTreeSet::from(["email".into()]),
            export_formats: BTreeSet::from(["json".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        party_hash: invoice_stable_hash(&InvoicePartyReference::default()),
        line_hash: invoice_stable_hash(&InvoiceLine::default()),
        totals_hash: invoice_stable_hash(&InvoiceTotals::default()),
        record_hash: invoice_stable_hash(&InvoiceRecord::default()),
        reminder_hash: invoice_stable_hash(&InvoiceReminderPlan::default()),
        artifact_hash: invoice_stable_hash(&InvoiceArtifactHandle::default()),
    }
}

pub fn invoice_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
