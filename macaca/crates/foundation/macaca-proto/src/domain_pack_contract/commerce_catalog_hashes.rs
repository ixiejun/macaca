use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::commerce_catalog::{
    catalog_stable_hash, commerce_catalog_pack_definition, AvailabilitySnapshot,
    CatalogArtifactHandle, CatalogAttribution, CatalogFreshness, CatalogMutationPlan, CatalogPrice,
    CatalogProduct, CatalogProjection, CatalogProviderCapability, CatalogRedactionPolicy,
    CatalogResultStatus, CatalogSearchRequest, CatalogVariant, COMMERCE_CATALOG_COMMANDS,
};
use super::model::DomainPackProviderCapabilityState;

/// Trace-safe compatibility evidence for the Commerce catalog contract.
///
/// Each field is derived from bounded DTO fixtures. The values are used by tests
/// and admission diagnostics to detect schema drift without storing raw catalog
/// exports, provider search DSLs, unpublished product data, or media bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub product_hash: String,
    pub variant_hash: String,
    pub price_hash: String,
    pub availability_hash: String,
    pub search_hash: String,
    pub mutation_hash: String,
    pub artifact_hash: String,
    pub redaction_hash: String,
}

pub fn commerce_catalog_descriptor_hashes() -> CatalogDescriptorHashes {
    let freshness = CatalogFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
        stale_reason: None,
    };
    let attribution = CatalogAttribution {
        source_ref: "source:catalog".into(),
        provider_class: "mock".into(),
        required_display_ref: Some("display".into()),
    };
    CatalogDescriptorHashes {
        command_schema_hash: catalog_stable_hash(&COMMERCE_CATALOG_COMMANDS),
        result_schema_hash: catalog_stable_hash(&CatalogResultStatus::Success),
        descriptor_hash: catalog_stable_hash(&commerce_catalog_pack_definition()),
        provider_capability_hash: catalog_stable_hash(&CatalogProviderCapability {
            provider_class: "mock".into(),
            product_models: BTreeSet::from(["product".into(), "variant".into()]),
            feature_flags: BTreeSet::from(["search".into(), "price".into()]),
            limits: BTreeMap::from([("max_results".into(), 50)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        product_hash: catalog_stable_hash(&CatalogProduct {
            product_ref: "product".into(),
            title_ref: "title".into(),
            localized_content_refs: BTreeMap::from([("en-US".into(), "content".into())]),
            lifecycle_state: "active".into(),
            publication_state: "published".into(),
            product_type_ref: "type".into(),
            vendor_ref: Some("vendor".into()),
            brand_ref: Some("brand".into()),
            taxonomy_refs: vec!["taxonomy".into()],
            media_refs: vec!["media".into()],
            provider_version_token: "version".into(),
        }),
        variant_hash: catalog_stable_hash(&CatalogVariant {
            variant_ref: "variant".into(),
            product_ref: "product".into(),
            sku_ref: "sku".into(),
            purchasable: true,
            inventory_tracking: true,
            shipping_profile_ref: Some("shipping".into()),
            customs_metadata_ref: Some("customs".into()),
            media_refs: vec!["media".into()],
            provider_version_token: "version".into(),
            ..Default::default()
        }),
        price_hash: catalog_stable_hash(&CatalogPrice {
            price_ref: "price".into(),
            amount_micros: 1_000_000,
            currency: "USD".into(),
            provider_price_ref: "provider-price".into(),
            ..Default::default()
        }),
        availability_hash: catalog_stable_hash(&AvailabilitySnapshot {
            variant_ref: "variant".into(),
            status: "available".into(),
            freshness,
            attribution,
            ..Default::default()
        }),
        search_hash: catalog_stable_hash(&CatalogSearchRequest {
            query_ref: "query".into(),
            filters: BTreeMap::from([("status".into(), "active".into())]),
            facets: BTreeSet::from(["brand".into()]),
            projection: CatalogProjection {
                locale: "en-US".into(),
                include_prices: true,
                include_availability: true,
                ..Default::default()
            },
            max_results: 10,
            ..Default::default()
        }),
        mutation_hash: catalog_stable_hash(&CatalogMutationPlan {
            plan_ref: "plan".into(),
            target_ref: "product".into(),
            mutation_kind: "publish".into(),
            required_approval: true,
            provider_precondition_ref: Some("version".into()),
            idempotency_key: "idem".into(),
        }),
        artifact_hash: catalog_stable_hash(&CatalogArtifactHandle {
            artifact_id: "artifact".into(),
            export_format: "jsonl".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            redaction_profile: "catalog".into(),
        }),
        redaction_hash: catalog_stable_hash(&CatalogRedactionPolicy {
            policy_ref: "redaction".into(),
            redacted_fields: BTreeSet::from(["raw_provider_payload".into()]),
            export_profile: "bounded".into(),
        }),
    }
}
