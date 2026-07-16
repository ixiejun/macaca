use super::commerce_order::{
    FulfillmentIntent, OrderArtifactHandle, OrderAuditExportPlan, OrderCancellationPlan,
    OrderFreshness, OrderLifecycleTransitionPlan, OrderLine, OrderRecord,
};

impl OrderFreshness {
    /// Return whether an order status snapshot should be treated as stale.
    pub fn has_stale_data(&self) -> bool {
        matches!(self.freshness_class.as_str(), "stale" | "expired")
    }
}

impl OrderRecord {
    /// Validate order state before an order service provider Strategy executes.
    pub fn is_lifecycle_ready(&self, max_lines: usize, max_refs: usize, max_quantity: u32) -> bool {
        self.is_bounded(max_lines, max_refs)
            && bounded_order_token(&self.order_ref, 160)
            && bounded_order_token(&self.version_token_hash, 256)
            && matches!(
                self.lifecycle_state.state.as_str(),
                "planned" | "created" | "paid" | "fulfilled" | "cancelled" | "returned"
            )
            && !self.freshness.has_stale_data()
            && self.totals.totals_match()
            && self
                .lines
                .iter()
                .all(|line| line.has_valid_totals(max_quantity))
    }

    /// Detect version-token conflicts without exposing raw provider tokens.
    pub fn has_version_conflict(&self, expected_version_token_hash: &str) -> bool {
        bounded_order_token(expected_version_token_hash, 256)
            && self.version_token_hash != expected_version_token_hash
    }
}

impl OrderLine {
    /// Validate line identity, source shape, quantity, and line total sign.
    pub fn has_valid_totals(&self, max_quantity: u32) -> bool {
        let has_catalog_ref = self.product_ref.is_some() || self.variant_ref.is_some();
        let has_custom_ref = self.custom_line_ref.is_some();
        bounded_order_token(&self.line_ref, 160)
            && (has_catalog_ref ^ has_custom_ref)
            && self.quantity > 0
            && self.quantity <= max_quantity
            && self.line_total_micros() >= 0
    }

    /// Compute a bounded synthetic line total for contract tests and preflight checks.
    pub fn line_total_micros(&self) -> i64 {
        (self.price_snapshot_micros - self.discounts_micros
            + self.tax_micros
            + self.duties_micros
            + self.fees_micros)
            * i64::from(self.quantity)
    }
}

impl OrderLifecycleTransitionPlan {
    /// Validate lifecycle transition planning evidence before side effects.
    pub fn has_valid_transition(&self) -> bool {
        bounded_order_token(&self.plan_ref, 160)
            && bounded_order_token(&self.from_state, 96)
            && bounded_order_token(&self.to_state, 96)
            && self.from_state != self.to_state
            && self.validation_diagnostics.len() <= 16
            && (!matches!(self.to_state.as_str(), "cancelled" | "returned")
                || self.requires_approval)
    }
}

impl FulfillmentIntent {
    /// Validate fulfillment intent remains a boundary marker, not carrier execution.
    pub fn is_boundary_safe(&self) -> bool {
        bounded_order_token(&self.intent_ref, 160)
            && matches!(self.intent_kind.as_str(), "pickup" | "shipment" | "digital")
            && !self.line_allocations.is_empty()
            && self
                .line_allocations
                .iter()
                .all(|(line_ref, quantity)| bounded_order_token(line_ref, 160) && *quantity > 0)
            && self
                .tracking_reference_handle
                .as_deref()
                .is_none_or(|handle| bounded_order_token(handle, 256))
            && self.carrier_handoff_boundary
    }
}

impl OrderCancellationPlan {
    /// Validate cancellation can be requested without refund execution semantics.
    pub fn is_eligible(&self) -> bool {
        bounded_order_token(&self.plan_ref, 160)
            && bounded_order_token(&self.order_ref, 160)
            && bounded_order_token(&self.reason_ref, 160)
            && self.provider_supported
            && self.requires_approval
    }
}

impl OrderAuditExportPlan {
    /// Validate retained audit export plans remain bounded and redacted.
    pub fn is_bounded_plan(&self) -> bool {
        bounded_order_token(&self.export_ref, 160)
            && bounded_order_token(&self.scope_ref, 160)
            && matches!(self.format.as_str(), "json" | "csv" | "ndjson")
            && bounded_order_token(&self.redaction_profile, 160)
            && bounded_order_token(&self.retention_class, 96)
            && bounded_order_token(&self.replay_pointer, 256)
    }
}

impl OrderArtifactHandle {
    /// Validate audit artifact handles without exposing raw order exports.
    pub fn is_bounded_export(&self) -> bool {
        bounded_order_token(&self.artifact_id, 160)
            && matches!(self.export_format.as_str(), "json" | "csv" | "ndjson")
            && bounded_order_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_order_token(&self.retention_class, 96)
            && bounded_order_token(&self.access_policy_ref, 160)
    }
}

fn bounded_order_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
