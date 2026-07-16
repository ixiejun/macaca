use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const COMMERCE_CART_PACK_ID: &str = "pack.commerce.cart.v1";
pub const COMMERCE_CART_SERVICE_ID: &str = "service.commerce.cart";

pub const COMMERCE_CART_COMMANDS: &[&str] = &[
    "cart.inspect_provider",
    "cart.describe_schema",
    "cart.create_cart",
    "cart.read_cart",
    "cart.search_carts",
    "cart.plan_context_update",
    "cart.update_context",
    "cart.plan_line_mutation",
    "cart.line_request",
    "cart.plan_discount",
    "cart.discount_request",
    "cart.estimate_cart",
    "cart.validate_cart",
    "cart.plan_handoff",
    "cart.handoff_request",
    "cart.inspect_abandonment",
    "cart.plan_export",
    "cart.export_cart",
    "cart.get_artifact_handle",
];

const CART_PERMISSION_SCOPES: &[&str] = &[
    "commerce.cart.read",
    "commerce.cart.write",
    "commerce.cart.estimate",
    "commerce.cart.discount",
    "commerce.cart.handoff",
    "commerce.cart.export",
];

const CART_STATE_METADATA: &[(&str, &str)] = &[
    ("lines", "true"),
    ("context", "true"),
    ("versioning", "required_when_supported"),
    ("stale_data", "true"),
];
const CART_ESTIMATE_METADATA: &[(&str, &str)] = &[
    ("tax", "estimate"),
    ("shipping", "estimate"),
    ("discounts", "true"),
    ("freshness", "required"),
];
const CART_HANDOFF_METADATA: &[(&str, &str)] = &[
    ("checkout_url", "handle_only"),
    ("order_draft", "reference_only"),
    ("payment_execution", "false"),
    ("approval", "required"),
];
const CART_MOCK_METADATA: &[(&str, &str)] = &[
    ("carts", "synthetic"),
    ("estimates", "synthetic"),
    ("callable", "false"),
];
const CART_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("carts", "false"),
    ("handoff", "false"),
    ("reason", "provider_not_installed"),
];

const CART_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "cart-state",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CART_STATE_METADATA,
    },
    CommerceProviderClass {
        provider_class: "cart-estimate",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CART_ESTIMATE_METADATA,
    },
    CommerceProviderClass {
        provider_class: "cart-handoff",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CART_HANDOFF_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CART_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CART_UNAVAILABLE_METADATA,
    },
];

pub fn commerce_cart_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_CART_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-cart",
        docs_slug: "cart",
        sdk_slug: "cart",
        service_id: COMMERCE_CART_SERVICE_ID,
        commands: COMMERCE_CART_COMMANDS,
        permission_scopes: CART_PERMISSION_SCOPES,
        provider_classes: CART_PROVIDER_CLASSES,
        health_probe: "cart.inspect_provider",
        unavailable_reason: "commerce_cart_provider_not_installed",
        replay_schema: "commerce.cart.replay.v1",
        data_classification: "commerce_cart_reference_metadata",
        retention_policy: "cart_context_line_estimate_handoff_and_artifact_metadata_by_reference",
        redaction_policy: "buyer_pii_payment_data_secret_checkout_urls_raw_provider_payloads_mutation_dsl_and_unbounded_exports_redacted",
        timeout_ms: 90_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.commerce.cart.v1` as optional until a cart provider is installed.",
            "Use cart handles, version tokens, estimates, handoff handles, and artifacts instead of raw checkout payloads.",
        ],
        migration_notes: &[
            "Cart commands become callable only after an approved cart service provider registers matching schemas.",
            "Order placement, payment, receipt, entitlement, fulfillment, and inventory adjustment remain separate capability packs.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartScope {
    pub tenant_scope: String,
    pub store_ref: String,
    pub channel_ref: String,
    pub cart_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartProviderCapability {
    pub provider_class: String,
    pub feature_flags: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartFreshness {
    pub priced_at_epoch_ms: Option<u64>,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub stale_flags: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cart {
    pub cart_ref: String,
    pub lifecycle_state: String,
    pub context: CartContext,
    pub lines: Vec<CartLine>,
    pub adjustments: Vec<CartAdjustment>,
    pub estimate: CartEstimate,
    pub validation_issues: Vec<CartValidationIssue>,
    pub version_token_hash: String,
    pub freshness: CartFreshness,
    pub redaction_class: String,
}

impl Cart {
    /// Bounded cart fixtures prevent unbounded provider baskets from entering tests or traces.
    pub fn is_bounded(&self, max_lines: usize, max_issues: usize) -> bool {
        self.lines.len() <= max_lines && self.validation_issues.len() <= max_issues
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartContext {
    pub buyer_ref: Option<String>,
    pub anonymous_session_ref: Option<String>,
    pub locale: String,
    pub currency: String,
    pub country_code: Option<String>,
    pub customer_group_ref: Option<String>,
    pub channel_ref: Option<String>,
    pub address_refs: Vec<String>,
    pub consent_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartLine {
    pub line_ref: String,
    pub product_ref: Option<String>,
    pub variant_ref: Option<String>,
    pub custom_line_ref: Option<String>,
    pub quantity: u32,
    pub unit_price_micros: Option<i64>,
    pub selected_options: BTreeMap<String, String>,
    pub selling_plan_ref: Option<String>,
    pub requires_shipping: bool,
    pub validation_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartAdjustment {
    pub adjustment_ref: String,
    pub adjustment_kind: String,
    pub target_ref: Option<String>,
    pub amount_micros: i64,
    pub currency: String,
    pub provider_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartDiscountApplication {
    pub discount_ref: String,
    pub discount_kind: String,
    pub code_ref: Option<String>,
    pub target_ref: Option<String>,
    pub eligible: bool,
    pub stacking_group_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartTotals {
    pub subtotal_micros: i64,
    pub line_discount_micros: i64,
    pub cart_discount_micros: i64,
    pub tax_micros: i64,
    pub duties_micros: i64,
    pub shipping_micros: i64,
    pub fees_micros: i64,
    pub total_micros: i64,
    pub currency: String,
    pub currency_precision: u8,
}

impl CartTotals {
    pub fn totals_match(&self) -> bool {
        self.total_micros
            == self.subtotal_micros - self.line_discount_micros - self.cart_discount_micros
                + self.tax_micros
                + self.duties_micros
                + self.shipping_micros
                + self.fees_micros
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartEstimate {
    pub totals: CartTotals,
    pub price_valid_until_epoch_ms: Option<u64>,
    pub stale_flags: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartValidationIssue {
    pub issue_code: String,
    pub severity: String,
    pub retriable: bool,
    pub remediation_ref: Option<String>,
    pub bounded_provider_reason: Option<String>,
    pub target_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartHandoffIntent {
    pub handoff_ref: String,
    pub checkout_url_handle: Option<String>,
    pub order_draft_handle: Option<String>,
    pub quote_handle: Option<String>,
    pub expires_at_epoch_ms: u64,
    pub access_policy_ref: String,
    pub no_payment_no_order_marker: bool,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub redaction_profile: String,
}

define_commerce_command_wrappers!(
    CartInspectProviderCommand,
    CartDescribeSchemaCommand,
    CartCreateCartCommand,
    CartReadCartCommand,
    CartSearchCartsCommand,
    CartPlanContextUpdateCommand,
    CartUpdateContextCommand,
    CartPlanLineMutationCommand,
    CartLineRequestCommand,
    CartPlanDiscountCommand,
    CartDiscountRequestCommand,
    CartEstimateCartCommand,
    CartValidateCartCommand,
    CartPlanHandoffCommand,
    CartHandoffRequestCommand,
    CartInspectAbandonmentCommand,
    CartPlanExportCommand,
    CartExportCartCommand,
    CartGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CartResultStatus {
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
    HandoffAccepted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartResultEnvelope<T> {
    pub status: CartResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub cart_hash: String,
    pub totals_hash: String,
    pub handoff_hash: String,
    pub artifact_hash: String,
}

pub fn commerce_cart_descriptor_hashes() -> CartDescriptorHashes {
    let totals = CartTotals {
        subtotal_micros: 1_000_000,
        line_discount_micros: 100_000,
        total_micros: 900_000,
        currency: "USD".into(),
        currency_precision: 2,
        ..Default::default()
    };
    let cart = Cart {
        cart_ref: "cart".into(),
        lifecycle_state: "active".into(),
        lines: vec![CartLine {
            line_ref: "line".into(),
            variant_ref: Some("variant".into()),
            quantity: 1,
            validation_state: "valid".into(),
            ..Default::default()
        }],
        estimate: CartEstimate {
            totals: totals.clone(),
            ..Default::default()
        },
        version_token_hash: "version".into(),
        redaction_class: "buyer_reference_only".into(),
        ..Default::default()
    };
    CartDescriptorHashes {
        command_schema_hash: cart_stable_hash(&COMMERCE_CART_COMMANDS),
        result_schema_hash: cart_stable_hash(&CartResultStatus::Success),
        descriptor_hash: cart_stable_hash(&commerce_cart_pack_definition()),
        provider_capability_hash: cart_stable_hash(&CartProviderCapability {
            provider_class: "mock".into(),
            feature_flags: BTreeSet::from(["lines".into(), "handoff".into()]),
            limits: BTreeMap::from([("max_lines".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        cart_hash: cart_stable_hash(&cart),
        totals_hash: cart_stable_hash(&totals),
        handoff_hash: cart_stable_hash(&CartHandoffIntent {
            handoff_ref: "handoff".into(),
            checkout_url_handle: Some("url-handle".into()),
            expires_at_epoch_ms: 10,
            access_policy_ref: "policy".into(),
            no_payment_no_order_marker: true,
            replay_pointer: "replay".into(),
            ..Default::default()
        }),
        artifact_hash: cart_stable_hash(&CartArtifactHandle {
            artifact_id: "artifact".into(),
            export_format: "json".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            redaction_profile: "cart".into(),
        }),
    }
}

pub fn cart_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
