use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const COMMERCE_ORDER_PACK_ID: &str = "pack.commerce.order.v1";
pub const COMMERCE_ORDER_SERVICE_ID: &str = "service.commerce.order";

pub const COMMERCE_ORDER_COMMANDS: &[&str] = &[
    "order.inspect_provider",
    "order.describe_schema",
    "order.plan_order",
    "order.create_order",
    "order.read_order",
    "order.search_orders",
    "order.sync_status",
    "order.plan_state_transition",
    "order.state_transition_request",
    "order.plan_fulfillment_intent",
    "order.fulfillment_intent_request",
    "order.plan_cancellation",
    "order.cancel_order",
    "order.list_return_references",
    "order.plan_audit_export",
    "order.audit_export_request",
    "order.get_artifact_handle",
];

pub const COMMERCE_ORDER_TRACE_EVENTS: &[&str] = &[
    "order_pack_declared",
    "order_pack_admission_validated",
    "order_pack_policy_decision",
    "order_pack_provider_inspected",
    "order_pack_service_call_requested",
    "order_pack_service_call_succeeded",
    "order_pack_service_call_failed",
    "order_pack_lifecycle_planned",
    "order_pack_fulfillment_intent_planned",
    "order_pack_unavailable",
    "order_pack_snapshot_recorded",
];

const ORDER_PERMISSION_SCOPES: &[&str] = &[
    "commerce.order.read",
    "commerce.order.write",
    "commerce.order.status",
    "commerce.order.fulfillment_intent",
    "commerce.order.cancel",
    "commerce.order.audit_export",
];

const ORDER_RECORD_METADATA: &[(&str, &str)] = &[
    ("records", "true"),
    ("source_conversion", "optional"),
    ("versioning", "required_when_supported"),
    ("status_sync", "true"),
];
const ORDER_LIFECYCLE_METADATA: &[(&str, &str)] = &[
    ("state_transitions", "approval_required"),
    ("cancellation", "approval_required"),
    ("return_references", "read_only"),
];
const ORDER_FULFILLMENT_METADATA: &[(&str, &str)] = &[
    ("fulfillment_intent", "reference_only"),
    ("carrier_execution", "false"),
    ("inventory_adjustment", "false"),
];
const ORDER_MOCK_METADATA: &[(&str, &str)] = &[
    ("orders", "synthetic"),
    ("lifecycle", "synthetic"),
    ("callable", "false"),
];
const ORDER_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("orders", "false"),
    ("lifecycle", "false"),
    ("reason", "provider_not_installed"),
];

const ORDER_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "order-record",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORDER_RECORD_METADATA,
    },
    CommerceProviderClass {
        provider_class: "order-lifecycle",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORDER_LIFECYCLE_METADATA,
    },
    CommerceProviderClass {
        provider_class: "fulfillment-intent",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORDER_FULFILLMENT_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ORDER_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: ORDER_UNAVAILABLE_METADATA,
    },
];

pub fn commerce_order_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_ORDER_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-order",
        docs_slug: "order",
        sdk_slug: "order",
        service_id: COMMERCE_ORDER_SERVICE_ID,
        commands: COMMERCE_ORDER_COMMANDS,
        permission_scopes: ORDER_PERMISSION_SCOPES,
        provider_classes: ORDER_PROVIDER_CLASSES,
        health_probe: "order.inspect_provider",
        unavailable_reason: "commerce_order_provider_not_installed",
        replay_schema: "commerce.order.replay.v1",
        data_classification: "commerce_order_reference_metadata",
        retention_policy: "order_lifecycle_fulfillment_return_and_audit_artifact_metadata_by_reference",
        redaction_policy: "buyer_pii_payment_credentials_raw_provider_payloads_labels_receipts_invoices_refunds_and_unbounded_exports_redacted",
        timeout_ms: 90_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.commerce.order.v1` as optional until an order provider is installed.",
            "Use order records, lifecycle plans, fulfillment intent references, and audit artifacts instead of provider-native order payloads.",
        ],
        migration_notes: &[
            "Order commands become callable only after an approved order service provider registers matching schemas.",
            "Payment execution, refunds, receipts, invoices, entitlement provisioning, inventory adjustment, and carrier execution remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderScope {
    pub tenant_scope: String,
    pub store_ref: String,
    pub channel_ref: String,
    pub order_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub supported_states: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderRecord {
    pub order_ref: String,
    pub external_number_ref: Option<String>,
    pub source_ref: Option<String>,
    pub lifecycle_state: OrderLifecycleState,
    pub lines: Vec<OrderLine>,
    pub adjustments: Vec<OrderAdjustment>,
    pub totals: OrderTotals,
    pub party_refs: Vec<String>,
    pub address_refs: Vec<String>,
    pub redacted_customer_ref: Option<String>,
    pub redacted_session_ref: Option<String>,
    pub payment_status_refs: Vec<String>,
    pub invoice_receipt_refs: Vec<String>,
    pub fulfillment_refs: Vec<String>,
    pub return_refs: Vec<String>,
    pub version_token_hash: String,
    pub freshness: OrderFreshness,
    pub redaction_class: String,
}

impl OrderRecord {
    pub fn is_bounded(&self, max_lines: usize, max_refs: usize) -> bool {
        self.lines.len() <= max_lines
            && self.fulfillment_refs.len() <= max_refs
            && self.return_refs.len() <= max_refs
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderLine {
    pub line_ref: String,
    pub product_ref: Option<String>,
    pub variant_ref: Option<String>,
    pub custom_line_ref: Option<String>,
    pub quantity: u32,
    pub price_snapshot_micros: i64,
    pub tax_micros: i64,
    pub duties_micros: i64,
    pub discounts_micros: i64,
    pub fees_micros: i64,
    pub source_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAdjustment {
    pub adjustment_ref: String,
    pub adjustment_kind: String,
    pub amount_micros: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderTotals {
    pub subtotal_micros: i64,
    pub discount_micros: i64,
    pub tax_micros: i64,
    pub duties_micros: i64,
    pub shipping_micros: i64,
    pub fees_micros: i64,
    pub total_micros: i64,
    pub currency: String,
    pub currency_precision: u8,
}

impl OrderTotals {
    pub fn totals_match(&self) -> bool {
        self.total_micros
            == self.subtotal_micros - self.discount_micros
                + self.tax_micros
                + self.duties_micros
                + self.shipping_micros
                + self.fees_micros
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderLifecycleState {
    pub state: String,
    pub provider_state_ref: Option<String>,
    pub custom_state_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderLifecycleTransitionPlan {
    pub plan_ref: String,
    pub from_state: String,
    pub to_state: String,
    pub requires_approval: bool,
    pub validation_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FulfillmentIntent {
    pub intent_ref: String,
    pub location_ref: Option<String>,
    pub intent_kind: String,
    pub line_allocations: BTreeMap<String, u32>,
    pub tracking_reference_handle: Option<String>,
    pub carrier_handoff_boundary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FulfillmentStatusReference {
    pub status_ref: String,
    pub state: String,
    pub tracking_reference_handle: Option<String>,
    pub freshness: OrderFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderCancellationPlan {
    pub plan_ref: String,
    pub order_ref: String,
    pub reason_ref: String,
    pub refundable_status_ref: Option<String>,
    pub provider_supported: bool,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderCancellationResult {
    pub cancellation_ref: String,
    pub order_ref: String,
    pub state: String,
    pub side_effect_evidence_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderReturnReference {
    pub return_ref: String,
    pub order_ref: String,
    pub line_refs: Vec<String>,
    pub refund_execution_boundary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAuditExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub format: String,
    pub redaction_profile: String,
    pub retention_class: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_commerce_command_wrappers!(
    OrderInspectProviderCommand,
    OrderDescribeSchemaCommand,
    OrderPlanOrderCommand,
    OrderCreateOrderCommand,
    OrderReadOrderCommand,
    OrderSearchOrdersCommand,
    OrderSyncStatusCommand,
    OrderPlanStateTransitionCommand,
    OrderStateTransitionRequestCommand,
    OrderPlanFulfillmentIntentCommand,
    OrderFulfillmentIntentRequestCommand,
    OrderPlanCancellationCommand,
    OrderCancelOrderCommand,
    OrderListReturnReferencesCommand,
    OrderPlanAuditExportCommand,
    OrderAuditExportRequestCommand,
    OrderGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    ApprovalRequired,
    VersionConflict,
    LifecycleInvalid,
    ExportAccepted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderResultEnvelope<T> {
    pub status: OrderResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub order_hash: String,
    pub totals_hash: String,
    pub fulfillment_hash: String,
    pub cancellation_hash: String,
    pub artifact_hash: String,
}

pub fn commerce_order_descriptor_hashes() -> OrderDescriptorHashes {
    let totals = OrderTotals {
        subtotal_micros: 1_000_000,
        discount_micros: 100_000,
        total_micros: 900_000,
        currency: "USD".into(),
        currency_precision: 2,
        ..Default::default()
    };
    let freshness = OrderFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
    };
    let order = OrderRecord {
        order_ref: "order".into(),
        lifecycle_state: OrderLifecycleState {
            state: "created".into(),
            ..Default::default()
        },
        lines: vec![OrderLine {
            line_ref: "line".into(),
            quantity: 1,
            price_snapshot_micros: 1_000_000,
            ..Default::default()
        }],
        totals: totals.clone(),
        version_token_hash: "version".into(),
        freshness: freshness.clone(),
        redaction_class: "buyer_reference_only".into(),
        ..Default::default()
    };
    OrderDescriptorHashes {
        command_schema_hash: order_stable_hash(&COMMERCE_ORDER_COMMANDS),
        result_schema_hash: order_stable_hash(&OrderResultStatus::Success),
        descriptor_hash: order_stable_hash(&commerce_order_pack_definition()),
        provider_capability_hash: order_stable_hash(&OrderProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["lifecycle".into(), "audit_export".into()]),
            supported_states: BTreeSet::from(["created".into(), "cancelled".into()]),
            limits: BTreeMap::from([("max_lines".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        order_hash: order_stable_hash(&order),
        totals_hash: order_stable_hash(&totals),
        fulfillment_hash: order_stable_hash(&FulfillmentIntent {
            intent_ref: "intent".into(),
            intent_kind: "ship".into(),
            carrier_handoff_boundary: true,
            ..Default::default()
        }),
        cancellation_hash: order_stable_hash(&OrderCancellationPlan {
            plan_ref: "cancel-plan".into(),
            order_ref: "order".into(),
            reason_ref: "customer-request".into(),
            provider_supported: true,
            requires_approval: true,
            ..Default::default()
        }),
        artifact_hash: order_stable_hash(&OrderArtifactHandle {
            artifact_id: "artifact".into(),
            export_format: "json".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
    }
}

pub fn order_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
