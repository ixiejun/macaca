use super::commerce_payment_intent::{
    PaymentActionRequirement, PaymentAuthorization, PaymentCancellation, PaymentCapture,
    PaymentIntentArtifactHandle, PaymentIntentAuditExportPlan, PaymentIntentEventReference,
    PaymentIntentFreshness, PaymentIntentPlan, PaymentIntentRecord, PaymentMethodReference,
};

impl PaymentIntentFreshness {
    /// Return whether status or webhook evidence should produce a stale-data result.
    pub fn has_stale_data(&self) -> bool {
        matches!(self.freshness_class.as_str(), "stale" | "expired")
    }
}

impl PaymentIntentPlan {
    /// Validate create/confirm planning before any payment gateway adapter can run.
    pub fn has_execution_preconditions(&self) -> bool {
        self.has_valid_amount()
            && bounded_payment_token(&self.plan_ref, 160)
            && valid_currency(&self.currency)
            && matches!(self.capture_mode.as_str(), "automatic" | "manual")
            && bounded_payment_token(&self.merchant_account_ref, 160)
            && bounded_payment_token(&self.idempotency_key_hash, 256)
            && self.payment_method.has_safe_token_reference()
    }
}

impl PaymentMethodReference {
    /// Validate token-only payment method references and reject raw credentials.
    pub fn has_safe_token_reference(&self) -> bool {
        self.is_tokenized_only()
            && bounded_payment_token(&self.token_ref, 256)
            && matches!(self.method_type.as_str(), "card" | "wallet" | "bank_debit")
            && self
                .region_support
                .iter()
                .all(|region| bounded_payment_token(region, 16))
    }
}

impl PaymentIntentRecord {
    /// Validate state-machine evidence without exposing provider-native payloads.
    pub fn has_valid_state(&self) -> bool {
        self.amount_micros > 0
            && valid_currency(&self.currency)
            && matches!(self.capture_mode.as_str(), "automatic" | "manual")
            && matches!(
                self.state.as_str(),
                "requires_confirmation"
                    | "requires_action"
                    | "authorized"
                    | "captured"
                    | "cancelled"
                    | "failed"
            )
            && !self.freshness.has_stale_data()
            && self
                .action_requirements
                .iter()
                .all(PaymentActionRequirement::is_handle_only)
    }
}

impl PaymentActionRequirement {
    /// Validate next-action data is handle-only and never a client secret or raw SCA payload.
    pub fn is_handle_only(&self) -> bool {
        bounded_payment_token(&self.action_ref, 160)
            && matches!(self.action_type.as_str(), "redirect" | "sca" | "none")
            && self
                .redirect_handle
                .as_deref()
                .is_none_or(|handle| bounded_payment_token(handle, 256))
            && self.expires_at_epoch_ms > 0
    }
}

impl PaymentAuthorization {
    /// Validate authorization evidence and expiry metadata before capture planning.
    pub fn is_unexpired_reference(&self) -> bool {
        bounded_payment_token(&self.authorization_ref, 160)
            && self.amount_micros > 0
            && valid_currency(&self.currency)
            && self.expires_at_epoch_ms > 0
            && bounded_payment_token(&self.provider_reference_hash, 256)
            && bounded_payment_token(&self.side_effect_evidence_ref, 256)
    }
}

impl PaymentCapture {
    /// Validate full and partial capture bounds against the authorized amount.
    pub fn is_amount_allowed(&self, authorized_amount_micros: i64) -> bool {
        bounded_payment_token(&self.capture_ref, 160)
            && self.amount_micros > 0
            && self.amount_micros <= authorized_amount_micros
            && valid_currency(&self.currency)
            && bounded_payment_token(&self.provider_reference_hash, 256)
            && bounded_payment_token(&self.side_effect_evidence_ref, 256)
    }
}

impl PaymentCancellation {
    /// Validate cancel/void evidence without introducing refund semantics.
    pub fn is_boundary_safe(&self) -> bool {
        bounded_payment_token(&self.cancellation_ref, 160)
            && bounded_payment_token(&self.reason_ref, 160)
            && bounded_payment_token(&self.provider_reference_hash, 256)
            && bounded_payment_token(&self.side_effect_evidence_ref, 256)
    }
}

impl PaymentIntentEventReference {
    /// Validate webhook/event evidence is bounded, fresh, and replay-addressable.
    pub fn is_fresh_reference(&self) -> bool {
        bounded_payment_token(&self.event_ref, 160)
            && bounded_payment_token(&self.provider_class, 96)
            && bounded_payment_token(&self.event_type, 96)
            && self.event_timestamp_epoch_ms > 0
            && bounded_payment_token(&self.delivery_id_hash, 256)
            && !self.webhook_freshness.has_stale_data()
            && bounded_payment_token(&self.replay_pointer, 256)
            && bounded_payment_token(&self.bounded_result_code, 96)
    }
}

impl PaymentIntentAuditExportPlan {
    /// Validate audit export planning before retained artifacts are created.
    pub fn is_bounded_plan(&self) -> bool {
        bounded_payment_token(&self.export_ref, 160)
            && bounded_payment_token(&self.scope_ref, 160)
            && matches!(self.format.as_str(), "json" | "csv" | "ndjson")
            && bounded_payment_token(&self.redaction_profile, 160)
    }
}

impl PaymentIntentArtifactHandle {
    /// Validate audit artifact handles without leaking raw gateway output.
    pub fn is_bounded_export(&self) -> bool {
        bounded_payment_token(&self.artifact_id, 160)
            && matches!(self.export_format.as_str(), "json" | "csv" | "ndjson")
            && bounded_payment_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_payment_token(&self.retention_class, 96)
            && bounded_payment_token(&self.redaction_profile, 160)
            && bounded_payment_token(&self.access_policy_ref, 160)
    }
}

fn valid_currency(currency: &str) -> bool {
    currency.len() == 3 && currency.chars().all(|ch| ch.is_ascii_uppercase())
}

fn bounded_payment_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
