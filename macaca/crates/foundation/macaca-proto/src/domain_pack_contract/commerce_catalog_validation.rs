use super::commerce_catalog::{
    CatalogArtifactHandle, CatalogFreshness, CatalogMutationPlan, CatalogProduct,
    CatalogSearchRequest, CatalogVariant,
};

impl CatalogFreshness {
    /// Return whether catalog data should use the typed stale-data outcome.
    pub fn has_stale_data(&self) -> bool {
        self.stale_reason.is_some() || matches!(self.freshness_class.as_str(), "stale" | "expired")
    }
}

impl CatalogProduct {
    /// Validate product references and publication state without loading provider payloads.
    pub fn has_required_schema_fields(&self) -> bool {
        bounded_catalog_token(&self.product_ref, 160)
            && bounded_catalog_token(&self.title_ref, 256)
            && bounded_catalog_token(&self.product_type_ref, 160)
            && bounded_catalog_token(&self.provider_version_token, 256)
            && matches!(
                self.lifecycle_state.as_str(),
                "draft" | "active" | "archived" | "retired"
            )
            && matches!(
                self.publication_state.as_str(),
                "unpublished" | "published" | "scheduled"
            )
            && self
                .localized_content_refs
                .iter()
                .all(|(locale, content_ref)| {
                    bounded_catalog_token(locale, 32) && bounded_catalog_token(content_ref, 256)
                })
    }
}

impl CatalogVariant {
    /// Validate variant refs, purchasability metadata, and version-token evidence.
    pub fn has_required_schema_fields(&self) -> bool {
        bounded_catalog_token(&self.variant_ref, 160)
            && bounded_catalog_token(&self.product_ref, 160)
            && bounded_catalog_token(&self.sku_ref, 160)
            && bounded_catalog_token(&self.provider_version_token, 256)
            && self.option_values.iter().all(|(option_ref, value_ref)| {
                bounded_catalog_token(option_ref, 160) && bounded_catalog_token(value_ref, 256)
            })
    }
}

impl CatalogSearchRequest {
    /// Validate portable search inputs before provider-specific search Strategy dispatch.
    pub fn has_portable_preconditions(
        &self,
        max_results: u32,
        max_filters: usize,
        max_facets: usize,
    ) -> bool {
        self.is_bounded(max_results, max_filters)
            && self.facets.len() <= max_facets
            && self
                .query_ref
                .is_empty()
                .then_some(true)
                .unwrap_or_else(|| bounded_catalog_token(&self.query_ref, 256))
            && self.filters.iter().all(|(key, value)| {
                bounded_catalog_token(key, 96) && bounded_catalog_token(value, 256)
            })
            && self
                .facets
                .iter()
                .all(|facet| bounded_catalog_token(facet, 96))
            && self
                .sort_ref
                .as_deref()
                .is_none_or(|sort| bounded_catalog_token(sort, 96))
    }
}

impl CatalogMutationPlan {
    /// Validate mutation planning evidence without executing a product or media mutation.
    pub fn has_execution_preconditions(&self) -> bool {
        bounded_catalog_token(&self.plan_ref, 160)
            && bounded_catalog_token(&self.target_ref, 160)
            && bounded_catalog_token(&self.idempotency_key, 128)
            && matches!(
                self.mutation_kind.as_str(),
                "create" | "update" | "publish" | "unpublish" | "archive" | "media"
            )
            && (!matches!(self.mutation_kind.as_str(), "publish" | "archive")
                || self.requires_approval())
    }
}

impl CatalogArtifactHandle {
    /// Validate export handle metadata before retained catalog artifacts are exposed.
    pub fn is_bounded_export(&self) -> bool {
        bounded_catalog_token(&self.artifact_id, 160)
            && matches!(self.export_format.as_str(), "json" | "csv" | "ndjson")
            && bounded_catalog_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_catalog_token(&self.retention_class, 96)
            && bounded_catalog_token(&self.redaction_profile, 160)
    }
}

fn bounded_catalog_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
