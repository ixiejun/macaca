use super::finance_invoice::{
    InvoiceDraftPlan, InvoiceLine, InvoicePartyReference, InvoiceTaxReference, InvoiceTotals,
};

#[test]
fn invoice_preflight_validates_reference_only_parties_and_balanced_totals() {
    let seller = InvoicePartyReference {
        party_ref: "seller-ref".into(),
        party_kind: "seller".into(),
        display_name_ref: "seller-name".into(),
        tax_identifier_ref: None,
        billing_address_ref: Some("billing-ref".into()),
        shipping_address_ref: None,
        redaction_class: "reference_only".into(),
    };
    let buyer = InvoicePartyReference {
        party_ref: "buyer-ref".into(),
        party_kind: "buyer".into(),
        display_name_ref: "buyer-name".into(),
        tax_identifier_ref: None,
        billing_address_ref: Some("billing-ref".into()),
        shipping_address_ref: None,
        redaction_class: "reference_only".into(),
    };
    let line = InvoiceLine {
        line_ref: "line-ref".into(),
        item_ref: "item-ref".into(),
        quantity_micros: 1_000_000,
        unit_price_micros: 100,
        currency: "USD".into(),
        service_period_start: None,
        service_period_end: None,
        tax: Some(InvoiceTaxReference {
            tax_code_ref: "tax-code".into(),
            jurisdiction_ref: "jurisdiction".into(),
            amount_micros: 10,
        }),
        discount: None,
        adjustment: None,
        rounding_evidence_hash: "rounding-hash".into(),
    };
    let plan = InvoiceDraftPlan {
        plan_ref: "draft-plan".into(),
        seller,
        buyer,
        lines: vec![line],
        totals: InvoiceTotals {
            subtotal_micros: 100,
            tax_total_micros: 10,
            discount_total_micros: 0,
            amount_due_micros: 110,
            amount_paid_micros: 0,
            amount_remaining_micros: 110,
            currency: "USD".into(),
            precision: 2,
            validation: vec![],
        },
        idempotency_key: "idem-invoice".into(),
    };
    assert!(plan.has_safe_preconditions(10));
}

#[test]
fn invoice_preflight_rejects_raw_party_data_and_invalid_currency_or_totals() {
    let raw_party = InvoicePartyReference {
        party_ref: "seller-ref".into(),
        party_kind: "seller".into(),
        display_name_ref: "https://example.test/raw".into(),
        tax_identifier_ref: None,
        billing_address_ref: None,
        shipping_address_ref: None,
        redaction_class: "reference_only".into(),
    };
    assert!(!raw_party.is_safe_reference());
    let line = InvoiceLine {
        line_ref: "line-ref".into(),
        item_ref: "item-ref".into(),
        quantity_micros: 1,
        unit_price_micros: 1,
        currency: "usd".into(),
        service_period_start: None,
        service_period_end: None,
        tax: None,
        discount: None,
        adjustment: None,
        rounding_evidence_hash: "hash".into(),
    };
    assert!(!line.has_safe_preconditions());
}
