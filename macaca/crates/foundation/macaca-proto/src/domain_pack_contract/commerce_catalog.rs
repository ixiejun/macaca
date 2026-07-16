use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::commerce_common::{
    commerce_pack_definition, commerce_stable_hash, define_commerce_command_wrappers,
    CommercePackCommandEnvelope, CommercePackDescriptor, CommercePackError, CommercePackPage,
    CommerceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub use super::commerce_catalog_hashes::*;

pub const COMMERCE_CATALOG_PACK_ID: &str = "pack.commerce.catalog.v1";
pub const COMMERCE_CATALOG_SERVICE_ID: &str = "service.commerce.catalog";

pub const COMMERCE_CATALOG_COMMANDS: &[&str] = &[
    "catalog.inspect_provider",
    "catalog.describe_schema",
    "catalog.list_products",
    "catalog.get_product",
    "catalog.list_variants",
    "catalog.get_variant",
    "catalog.search_catalog",
    "catalog.list_taxonomy",
    "catalog.get_price",
    "catalog.check_availability",
    "catalog.plan_product_mutation",
    "catalog.product_request",
    "catalog.plan_variant_mutation",
    "catalog.variant_request",
    "catalog.plan_media_mutation",
    "catalog.media_request",
    "catalog.plan_export",
    "catalog.export_catalog",
    "catalog.get_artifact_handle",
];

const CATALOG_PERMISSION_SCOPES: &[&str] = &[
    "commerce.catalog.read",
    "commerce.catalog.search",
    "commerce.catalog.price",
    "commerce.catalog.availability",
    "commerce.catalog.write",
    "commerce.catalog.publish",
    "commerce.catalog.export",
];

const CATALOG_READ_METADATA: &[(&str, &str)] = &[
    ("products", "true"),
    ("variants", "true"),
    ("prices", "reference"),
    ("availability", "snapshot"),
];
const CATALOG_SEARCH_METADATA: &[(&str, &str)] = &[
    ("filters", "portable"),
    ("facets", "true"),
    ("localization", "true"),
    ("async", "optional"),
];
const CATALOG_MUTATION_METADATA: &[(&str, &str)] = &[
    ("mutation_plans", "true"),
    ("publish", "approval_required"),
    ("idempotency", "required"),
    ("version_tokens", "required_when_supported"),
];
const CATALOG_MOCK_METADATA: &[(&str, &str)] = &[
    ("products", "synthetic"),
    ("variants", "synthetic"),
    ("search", "synthetic"),
    ("callable", "false"),
];
const CATALOG_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("products", "false"),
    ("search", "false"),
    ("write", "false"),
    ("reason", "provider_not_installed"),
];

const CATALOG_PROVIDER_CLASSES: &[CommerceProviderClass<'_>] = &[
    CommerceProviderClass {
        provider_class: "catalog-read-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CATALOG_READ_METADATA,
    },
    CommerceProviderClass {
        provider_class: "catalog-search",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CATALOG_SEARCH_METADATA,
    },
    CommerceProviderClass {
        provider_class: "catalog-mutation",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CATALOG_MUTATION_METADATA,
    },
    CommerceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CATALOG_MOCK_METADATA,
    },
    CommerceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CATALOG_UNAVAILABLE_METADATA,
    },
];

/// Build the catalog descriptor without binding storefront, checkout, payment, inventory, or UI code.
pub fn commerce_catalog_pack_definition() -> DomainPackDefinition {
    commerce_pack_definition(CommercePackDescriptor {
        pack_id: COMMERCE_CATALOG_PACK_ID,
        child_change_id: "openspec:add-pack-commerce-catalog",
        docs_slug: "catalog",
        sdk_slug: "catalog",
        service_id: COMMERCE_CATALOG_SERVICE_ID,
        commands: COMMERCE_CATALOG_COMMANDS,
        permission_scopes: CATALOG_PERMISSION_SCOPES,
        provider_classes: CATALOG_PROVIDER_CLASSES,
        health_probe: "catalog.inspect_provider",
        unavailable_reason: "commerce_catalog_provider_not_installed",
        replay_schema: "commerce.catalog.replay.v1",
        data_classification: "commerce_catalog_reference_metadata",
        retention_policy: "product_variant_price_availability_taxonomy_search_and_artifact_metadata_by_reference",
        redaction_policy: "credentials_unpublished_secrets_raw_provider_payloads_media_bytes_provider_search_dsl_and_unbounded_exports_redacted",
        timeout_ms: 90_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.commerce.catalog.v1` as optional until a commerce catalog provider is installed.",
            "Use catalog handles, portable filters, mutation plans, and artifact handles instead of provider-native payloads.",
        ],
        migration_notes: &[
            "Catalog commands become callable only after an approved catalog service provider registers matching schemas.",
            "Cart mutation, order creation, payment, receipt, entitlement, fulfillment, and inventory adjustment remain separate packs.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogScope {
    pub tenant_scope: String,
    pub store_ref: String,
    pub channel_ref: String,
    pub locale: String,
    pub currency: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogProviderCapability {
    pub provider_class: String,
    pub product_models: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogAttribution {
    pub source_ref: String,
    pub provider_class: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
    pub export_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogProduct {
    pub product_ref: String,
    pub title_ref: String,
    pub localized_content_refs: BTreeMap<String, String>,
    pub lifecycle_state: String,
    pub publication_state: String,
    pub product_type_ref: String,
    pub vendor_ref: Option<String>,
    pub brand_ref: Option<String>,
    pub taxonomy_refs: Vec<String>,
    pub media_refs: Vec<String>,
    pub provider_version_token: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogVariant {
    pub variant_ref: String,
    pub product_ref: String,
    pub sku_ref: String,
    pub barcode_ref: Option<String>,
    pub option_values: BTreeMap<String, String>,
    pub purchasable: bool,
    pub inventory_tracking: bool,
    pub shipping_profile_ref: Option<String>,
    pub customs_metadata_ref: Option<String>,
    pub media_refs: Vec<String>,
    pub default_price_ref: Option<String>,
    pub provider_version_token: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogAttribute {
    pub attribute_ref: String,
    pub value_kind: String,
    pub localized_label_refs: BTreeMap<String, String>,
    pub validation_rule_ref: Option<String>,
    pub provider_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogOption {
    pub option_ref: String,
    pub localized_label_refs: BTreeMap<String, String>,
    pub allowed_values: Vec<String>,
    pub validation_rule_ref: Option<String>,
    pub provider_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogModifier {
    pub modifier_ref: String,
    pub modifier_kind: String,
    pub localized_label_refs: BTreeMap<String, String>,
    pub validation_rule_ref: Option<String>,
    pub required: bool,
    pub provider_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceContext {
    pub price_book_ref: String,
    pub customer_group_ref: Option<String>,
    pub channel_ref: Option<String>,
    pub country_code: Option<String>,
    pub tax_included: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogPrice {
    pub price_ref: String,
    pub amount_micros: i64,
    pub currency: String,
    pub recurring_terms_ref: Option<String>,
    pub effective_from_epoch_ms: Option<u64>,
    pub effective_to_epoch_ms: Option<u64>,
    pub provider_price_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceBook {
    pub price_book_ref: String,
    pub name_ref: String,
    pub currency: String,
    pub context: PriceContext,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailabilitySnapshot {
    pub variant_ref: String,
    pub location_ref: Option<String>,
    pub channel_ref: Option<String>,
    pub status: String,
    pub quantity_ref: Option<String>,
    pub freshness: CatalogFreshness,
    pub attribution: CatalogAttribution,
    pub inventory_handoff_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogTaxonomyNode {
    pub node_ref: String,
    pub parent_ref: Option<String>,
    pub node_kind: String,
    pub localized_label_refs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogPublicationScope {
    pub scope_ref: String,
    pub store_ref: Option<String>,
    pub channel_ref: Option<String>,
    pub publication_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogChannel {
    pub channel_ref: String,
    pub channel_kind: String,
    pub country_codes: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogProjection {
    pub include_prices: bool,
    pub include_availability: bool,
    pub locale: String,
    pub price_context: Option<PriceContext>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSearchRequest {
    pub query_ref: String,
    pub filters: BTreeMap<String, String>,
    pub facets: BTreeSet<String>,
    pub sort_ref: Option<String>,
    pub projection: CatalogProjection,
    pub max_results: u32,
}

impl CatalogSearchRequest {
    /// Keep portable catalog search bounded before a provider-specific search DSL exists.
    pub fn is_bounded(&self, max_results: u32, max_filters: usize) -> bool {
        self.max_results > 0 && self.max_results <= max_results && self.filters.len() <= max_filters
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSearchResult {
    pub product_ref: Option<String>,
    pub variant_ref: Option<String>,
    pub score_micros: i64,
    pub unsupported_filters: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogMutationPlan {
    pub plan_ref: String,
    pub target_ref: String,
    pub mutation_kind: String,
    pub required_approval: bool,
    pub provider_precondition_ref: Option<String>,
    pub idempotency_key: String,
}

impl CatalogMutationPlan {
    pub fn requires_approval(&self) -> bool {
        self.required_approval || matches!(self.mutation_kind.as_str(), "publish" | "archive")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogMutationResult {
    pub mutation_ref: String,
    pub target_ref: String,
    pub status: String,
    pub version_token_hash: Option<String>,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub redaction_profile: String,
}

define_commerce_command_wrappers!(
    CatalogInspectProviderCommand,
    CatalogDescribeSchemaCommand,
    CatalogListProductsCommand,
    CatalogGetProductCommand,
    CatalogListVariantsCommand,
    CatalogGetVariantCommand,
    CatalogSearchCatalogCommand,
    CatalogListTaxonomyCommand,
    CatalogGetPriceCommand,
    CatalogCheckAvailabilityCommand,
    CatalogPlanProductMutationCommand,
    CatalogProductRequestCommand,
    CatalogPlanVariantMutationCommand,
    CatalogVariantRequestCommand,
    CatalogPlanMediaMutationCommand,
    CatalogMediaRequestCommand,
    CatalogPlanExportCommand,
    CatalogExportCatalogCommand,
    CatalogGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    SchemaMismatch,
    ApprovalRequired,
    VersionConflict,
    ExportAccepted,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogResultEnvelope<T> {
    pub status: CatalogResultStatus,
    pub data: Option<T>,
    pub page: Option<CommercePackPage<T>>,
    pub error: Option<CommercePackError>,
}

pub fn catalog_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    commerce_stable_hash(value)
}
