use super::commerce_cart::{
    Cart, CartArtifactHandle, CartEstimate, CartFreshness, CartHandoffIntent, CartLine,
};

impl CartFreshness {
    /// Return whether provider data is already marked stale by bounded flags.
    pub fn has_stale_data(&self) -> bool {
        !self.stale_flags.is_empty()
    }
}

impl Cart {
    /// Validate state that must be checked before a cart mutation provider runs.
    ///
    /// This is a provider-neutral Specification guard: it accepts only lifecycle
    /// states that can be safely mutated, requires a version-token hash for
    /// replayable conflict detection, rejects stale estimate flags, and validates
    /// each line without inspecting vendor-specific availability payloads.
    pub fn is_mutation_ready(
        &self,
        max_lines: usize,
        max_issues: usize,
        max_quantity: u32,
    ) -> bool {
        self.is_bounded(max_lines, max_issues)
            && matches!(self.lifecycle_state.as_str(), "draft" | "active")
            && bounded_cart_token(&self.cart_ref, 160)
            && bounded_cart_token(&self.version_token_hash, 256)
            && !self.freshness.has_stale_data()
            && !self.estimate.has_stale_data()
            && self
                .lines
                .iter()
                .all(|line| line.is_available_for_mutation(max_quantity))
    }

    /// Detect optimistic-concurrency conflicts from bounded token hashes only.
    pub fn has_version_conflict(&self, expected_version_token_hash: &str) -> bool {
        bounded_cart_token(expected_version_token_hash, 256)
            && self.version_token_hash != expected_version_token_hash
    }
}

impl CartLine {
    /// Check a line can be sent to a provider mutation without raw catalog data.
    pub fn is_available_for_mutation(&self, max_quantity: u32) -> bool {
        let has_catalog_ref = self.product_ref.is_some() || self.variant_ref.is_some();
        let has_custom_ref = self.custom_line_ref.is_some();
        bounded_cart_token(&self.line_ref, 160)
            && (has_catalog_ref ^ has_custom_ref)
            && self.quantity > 0
            && self.quantity <= max_quantity
            && matches!(self.validation_state.as_str(), "valid" | "pending")
    }
}

impl CartEstimate {
    /// Return whether estimate metadata should trigger a stale-data result.
    pub fn has_stale_data(&self) -> bool {
        !self.stale_flags.is_empty()
    }
}

impl CartHandoffIntent {
    /// Validate that handoff data is a handle-only intent, not an order/payment action.
    pub fn is_boundary_safe(&self) -> bool {
        bounded_cart_token(&self.handoff_ref, 160)
            && self
                .checkout_url_handle
                .as_deref()
                .is_none_or(|handle| bounded_cart_token(handle, 256))
            && self
                .order_draft_handle
                .as_deref()
                .is_none_or(|handle| bounded_cart_token(handle, 256))
            && self
                .quote_handle
                .as_deref()
                .is_none_or(|handle| bounded_cart_token(handle, 256))
            && self.no_payment_no_order_marker
            && bounded_cart_token(&self.access_policy_ref, 160)
            && bounded_cart_token(&self.replay_pointer, 256)
    }
}

impl CartArtifactHandle {
    /// Validate export handles before retained artifacts enter trace or storage.
    pub fn is_bounded_export(&self) -> bool {
        bounded_cart_token(&self.artifact_id, 160)
            && matches!(self.export_format.as_str(), "json" | "csv" | "ndjson")
            && bounded_cart_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_cart_token(&self.retention_class, 96)
            && bounded_cart_token(&self.redaction_profile, 160)
    }
}

fn bounded_cart_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
