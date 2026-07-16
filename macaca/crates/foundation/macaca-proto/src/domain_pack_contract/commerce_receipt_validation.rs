use super::commerce_receipt::{
    ReceiptArtifactHandle, ReceiptAudience, ReceiptAuditExportPlan, ReceiptCorrectionReference,
    ReceiptDeliveryRequest, ReceiptDeliveryState, ReceiptEventReference, ReceiptFreshness,
    ReceiptLine, ReceiptRecord, ReceiptSourceReference, ReceiptVariant, ReceiptVerificationResult,
};

impl ReceiptFreshness {
    /// Return whether receipt/source metadata should produce a stale-data result.
    pub fn has_stale_data(&self) -> bool {
        matches!(self.freshness_class.as_str(), "stale" | "expired")
    }
}

impl ReceiptSourceReference {
    /// Validate that source linkage is visible as a bounded reference only.
    pub fn is_visible_reference(&self) -> bool {
        bounded_receipt_token(&self.source_ref, 160)
            && matches!(
                self.source_kind.as_str(),
                "payment_intent"
                    | "capture"
                    | "charge"
                    | "transaction"
                    | "order"
                    | "invoice"
                    | "refund"
                    | "void"
                    | "cash_payment"
                    | "terminal_transaction"
                    | "external_document"
                    | "provider_event"
            )
            && bounded_receipt_token(&self.provider_reference_hash, 256)
            && bounded_receipt_token(&self.redaction_class, 96)
    }
}

impl ReceiptRecord {
    /// Validate receipt issue/reissue evidence before a provider adapter can run.
    pub fn has_issue_preconditions(&self, max_lines: usize, max_sources: usize) -> bool {
        self.is_bounded(max_lines, max_sources)
            && bounded_receipt_token(&self.receipt_ref, 160)
            && matches!(
                self.issue_state.as_str(),
                "planned" | "issued" | "reissued" | "corrected" | "voided"
            )
            && !self.freshness.has_stale_data()
            && self.totals.totals_match()
            && self.audience.is_supported()
            && self.variant.is_supported()
            && self
                .source_refs
                .iter()
                .all(ReceiptSourceReference::is_visible_reference)
            && self.lines.iter().all(ReceiptLine::has_valid_amounts)
    }
}

impl ReceiptLine {
    /// Validate line amounts and references without raw receipt body data.
    pub fn has_valid_amounts(&self) -> bool {
        bounded_receipt_token(&self.line_ref, 160)
            && bounded_receipt_token(&self.description_ref, 256)
            && self.quantity_micros > 0
            && self.unit_amount_micros >= 0
    }
}

impl ReceiptAudience {
    /// Validate generic receipt audience classes without application-specific recipients.
    pub fn is_supported(&self) -> bool {
        matches!(
            self.audience_kind.as_str(),
            "customer" | "merchant" | "cashier" | "gift" | "regulatory" | "custom"
        )
    }
}

impl ReceiptVariant {
    /// Validate receipt variants as generic presentation handles only.
    pub fn is_supported(&self) -> bool {
        matches!(
            self.variant_kind.as_str(),
            "hosted" | "printable" | "terminal" | "refund" | "correction" | "custom"
        )
    }
}

impl ReceiptDeliveryRequest {
    /// Validate delivery requests require approval/idempotency and reference-only destinations.
    pub fn has_delivery_preconditions(&self) -> bool {
        bounded_receipt_token(&self.request_ref, 160)
            && matches!(
                self.channel.as_str(),
                "email_ref" | "sms_ref" | "hosted" | "terminal_print"
            )
            && bounded_receipt_token(&self.destination_ref, 256)
            && self.approval_ref.is_some()
            && bounded_receipt_token(&self.idempotency_key_hash, 256)
    }
}

impl ReceiptDeliveryState {
    /// Validate bounded delivery diagnostics before exposing status to SDKs.
    pub fn is_bounded(&self, max_attempts: u32) -> bool {
        matches!(
            self.state.as_str(),
            "planned" | "pending" | "sent" | "delivered" | "failed"
        ) && self.attempt_count <= max_attempts
            && self
                .provider_message_ref
                .as_deref()
                .is_none_or(|reference| bounded_receipt_token(reference, 256))
            && self
                .terminal_action_ref
                .as_deref()
                .is_none_or(|reference| bounded_receipt_token(reference, 256))
    }
}

impl ReceiptVerificationResult {
    /// Validate verification evidence is linked, replayable, and conflict-aware.
    pub fn is_consistent(&self) -> bool {
        bounded_receipt_token(&self.verification_ref, 160)
            && self.source_linked
            && self.totals_match
            && matches!(self.checksum_status.as_str(), "matched" | "skipped")
            && bounded_receipt_token(&self.replay_pointer, 256)
    }
}

impl ReceiptCorrectionReference {
    /// Validate correction references do not execute refunds, voids, or chargebacks.
    pub fn is_boundary_safe(&self) -> bool {
        bounded_receipt_token(&self.correction_ref, 160)
            && matches!(
                self.correction_kind.as_str(),
                "refund"
                    | "void"
                    | "cancellation"
                    | "chargeback"
                    | "return"
                    | "replacement"
                    | "adjustment"
            )
            && bounded_receipt_token(&self.source_ref, 160)
            && self.no_side_effect_payload_marker
    }
}

impl ReceiptEventReference {
    /// Validate event references remain bounded and do not store webhook bodies.
    pub fn is_fresh_reference(&self) -> bool {
        bounded_receipt_token(&self.event_ref, 160)
            && bounded_receipt_token(&self.provider_class, 96)
            && bounded_receipt_token(&self.event_type, 96)
            && self.event_timestamp_epoch_ms > 0
            && bounded_receipt_token(&self.delivery_id_hash, 256)
            && !self.webhook_freshness.has_stale_data()
            && bounded_receipt_token(&self.replay_pointer, 256)
            && bounded_receipt_token(&self.bounded_result_code, 96)
    }
}

impl ReceiptAuditExportPlan {
    /// Validate audit export plans stay bounded and redacted.
    pub fn is_bounded_plan(&self) -> bool {
        bounded_receipt_token(&self.export_ref, 160)
            && bounded_receipt_token(&self.scope_ref, 160)
            && matches!(self.format.as_str(), "json" | "csv" | "ndjson")
            && bounded_receipt_token(&self.retention_class, 96)
            && bounded_receipt_token(&self.redaction_profile, 160)
            && bounded_receipt_token(&self.replay_pointer, 256)
    }
}

impl ReceiptArtifactHandle {
    /// Validate artifact handles without exposing hosted URLs, PDFs, or print data.
    pub fn is_bounded_artifact(&self) -> bool {
        bounded_receipt_token(&self.artifact_id, 160)
            && matches!(
                self.artifact_type.as_str(),
                "hosted" | "pdf" | "json" | "print"
            )
            && self
                .hosted_url_metadata_ref
                .as_deref()
                .is_none_or(|reference| bounded_receipt_token(reference, 256))
            && bounded_receipt_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_receipt_token(&self.retention_class, 96)
            && bounded_receipt_token(&self.redaction_profile, 160)
            && bounded_receipt_token(&self.access_policy_ref, 160)
            && bounded_receipt_token(&self.replay_pointer, 256)
    }
}

fn bounded_receipt_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
