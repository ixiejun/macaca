use super::finance_common::FinanceCommandEnvelope;
use super::finance_invoice::{
    InvoiceArtifactHandle, InvoiceConcurrencyToken, InvoiceDeliveryState, InvoiceDraftPlan,
    InvoiceLifecycleState, InvoiceLine, InvoicePartyReference, InvoiceRecord, InvoiceTaxReference,
    InvoiceTotals, TaxIdentifierReference,
};

fn bounded_invoice_reference(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}

fn supported_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

impl FinanceCommandEnvelope {
    /// Bound trace-safe command metadata before a runtime provider receives an invoice request.
    pub fn has_bounded_invoice_request(&self, max_parameters: usize, max_page_size: u32) -> bool {
        bounded_invoice_reference(&self.subject_ref, 160)
            && self.parameters.len() <= max_parameters
            && self.parameters.iter().all(|(key, value)| {
                bounded_invoice_reference(key, 96) && bounded_invoice_reference(value, 256)
            })
            && self
                .cursor
                .as_deref()
                .is_none_or(|cursor| bounded_invoice_reference(cursor, 256))
            && self
                .page_size
                .is_none_or(|page_size| page_size > 0 && page_size <= max_page_size)
            && self
                .idempotency_key
                .as_deref()
                .is_none_or(|key| bounded_invoice_reference(key, 160))
    }
}

impl TaxIdentifierReference {
    /// Tax identifiers are represented only by redacted references and masked evidence.
    pub fn is_safe_reference(&self) -> bool {
        bounded_invoice_reference(&self.tax_ref, 160)
            && bounded_invoice_reference(&self.jurisdiction_ref, 160)
            && bounded_invoice_reference(&self.masked_value_ref, 256)
    }
}

impl InvoicePartyReference {
    /// Require an invoice party handle and redacted address/tax references.
    pub fn is_safe_reference(&self) -> bool {
        bounded_invoice_reference(&self.party_ref, 160)
            && matches!(self.party_kind.as_str(), "seller" | "buyer" | "recipient")
            && bounded_invoice_reference(&self.display_name_ref, 256)
            && self
                .tax_identifier_ref
                .as_ref()
                .is_none_or(TaxIdentifierReference::is_safe_reference)
            && self
                .billing_address_ref
                .as_deref()
                .is_none_or(|value| bounded_invoice_reference(value, 256))
            && self
                .shipping_address_ref
                .as_deref()
                .is_none_or(|value| bounded_invoice_reference(value, 256))
            && bounded_invoice_reference(&self.redaction_class, 96)
    }
}

impl InvoiceTaxReference {
    /// Validate non-negative tax metadata without exposing jurisdictional payloads.
    pub fn is_safe_reference(&self) -> bool {
        bounded_invoice_reference(&self.tax_code_ref, 160)
            && bounded_invoice_reference(&self.jurisdiction_ref, 160)
            && self.amount_micros >= 0
    }
}

impl InvoiceLine {
    /// Validate arithmetic inputs and reference-only line metadata before draft planning.
    pub fn has_safe_preconditions(&self) -> bool {
        bounded_invoice_reference(&self.line_ref, 160)
            && bounded_invoice_reference(&self.item_ref, 160)
            && self.quantity_micros > 0
            && self.unit_price_micros >= 0
            && supported_currency(&self.currency)
            && self
                .tax
                .as_ref()
                .is_none_or(InvoiceTaxReference::is_safe_reference)
            && self.discount.as_ref().is_none_or(|discount| {
                bounded_invoice_reference(&discount.discount_ref, 160)
                    && discount.amount_micros >= 0
                    && bounded_invoice_reference(&discount.reason_ref, 160)
            })
            && self.adjustment.as_ref().is_none_or(|adjustment| {
                bounded_invoice_reference(&adjustment.adjustment_ref, 160)
                    && bounded_invoice_reference(&adjustment.reason_ref, 160)
            })
            && bounded_invoice_reference(&self.rounding_evidence_hash, 256)
    }
}

impl InvoiceTotals {
    /// Validate currency precision and non-negative, internally consistent totals.
    pub fn is_consistent_with(&self, lines: &[InvoiceLine]) -> bool {
        let subtotal = lines
            .iter()
            .map(|line| line.quantity_micros * line.unit_price_micros / 1_000_000)
            .sum::<i64>();
        let tax = lines
            .iter()
            .filter_map(|line| line.tax.as_ref())
            .map(|value| value.amount_micros)
            .sum::<i64>();
        let discount = lines
            .iter()
            .filter_map(|line| line.discount.as_ref())
            .map(|value| value.amount_micros)
            .sum::<i64>();
        let adjustment = lines
            .iter()
            .filter_map(|line| line.adjustment.as_ref())
            .map(|value| value.amount_micros)
            .sum::<i64>();
        supported_currency(&self.currency)
            && self.precision <= 6
            && self.subtotal_micros == subtotal
            && self.tax_total_micros == tax
            && self.discount_total_micros == discount
            && self.amount_due_micros == subtotal + tax - discount + adjustment
            && self.amount_paid_micros >= 0
            && self.amount_remaining_micros == self.amount_due_micros - self.amount_paid_micros
            && self.amount_remaining_micros >= 0
            && self.validation.iter().all(|diagnostic| {
                bounded_invoice_reference(&diagnostic.code, 96)
                    && bounded_invoice_reference(&diagnostic.trace_safe_detail, 256)
            })
    }
}

impl InvoiceDraftPlan {
    /// Check parties, line totals, currency, and idempotency before runtime approval.
    pub fn has_safe_preconditions(&self, max_lines: usize) -> bool {
        bounded_invoice_reference(&self.plan_ref, 160)
            && self.seller.is_safe_reference()
            && self.seller.party_kind == "seller"
            && self.buyer.is_safe_reference()
            && matches!(self.buyer.party_kind.as_str(), "buyer" | "recipient")
            && !self.lines.is_empty()
            && self.lines.len() <= max_lines
            && self.lines.iter().all(InvoiceLine::has_safe_preconditions)
            && self.totals.is_consistent_with(&self.lines)
            && bounded_invoice_reference(&self.idempotency_key, 160)
    }
}

impl InvoiceLifecycleState {
    /// Validate lifecycle ordering without implementing provider transition behavior.
    pub fn is_consistent(&self) -> bool {
        matches!(self.state.as_str(), "draft" | "issued" | "paid" | "void")
            && (self.state != "issued" || self.issued_at_epoch_ms.is_some_and(|value| value > 0))
            && (self.state != "void" || self.voided_at_epoch_ms.is_some_and(|value| value > 0))
    }
}

impl InvoiceConcurrencyToken {
    /// Require bounded provider-neutral version evidence for mutation requests.
    pub fn is_safe_reference(&self) -> bool {
        bounded_invoice_reference(&self.token_hash, 256)
            && bounded_invoice_reference(&self.provider_version_ref, 160)
    }
}

impl InvoiceDeliveryState {
    /// Preserve recipient policy evidence without holding delivery addresses or payloads.
    pub fn is_safe_policy_projection(&self) -> bool {
        matches!(
            self.state.as_str(),
            "not_requested" | "planned" | "sent" | "failed"
        ) && self
            .channel
            .as_deref()
            .is_none_or(|value| bounded_invoice_reference(value, 96))
            && bounded_invoice_reference(&self.recipient_policy_hash, 256)
    }
}

impl InvoiceArtifactHandle {
    /// Bound asynchronous export results as expiring artifact metadata only.
    pub fn is_safe_async_result(&self, now_epoch_ms: u64) -> bool {
        bounded_invoice_reference(&self.artifact_id, 160)
            && matches!(self.export_format.as_str(), "json" | "csv" | "pdf")
            && bounded_invoice_reference(&self.checksum, 256)
            && self.expires_at_epoch_ms > now_epoch_ms
            && bounded_invoice_reference(&self.access_policy, 160)
    }
}

impl InvoiceRecord {
    /// Validate a redacted invoice status projection before trace or SDK exposure.
    pub fn has_safe_projection(&self) -> bool {
        bounded_invoice_reference(&self.invoice_ref, 160)
            && self.lifecycle.is_consistent()
            && self.delivery.is_safe_policy_projection()
            && self.concurrency.is_safe_reference()
            && bounded_invoice_reference(&self.lifecycle_evidence.evidence_ref, 160)
            && bounded_invoice_reference(&self.lifecycle_evidence.provider_trace_ref, 256)
            && bounded_invoice_reference(&self.lifecycle_evidence.idempotency_key, 160)
    }
}
