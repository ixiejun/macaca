use std::fs;
use std::path::Path;

#[test]
fn payment_intent_provider_is_runtime_host_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(root.join(
        "crates/runtime/macaca-runtime-host/src/commerce_payment_intent_service_provider.rs",
    ))
    .unwrap();
    assert!(source.contains("impl SystemService"));
    for forbidden in [
        "receipt_service_provider",
        "payment.refund",
        "settlement.reconcile",
        "fraud_decision",
    ] {
        assert!(!source.contains(forbidden));
    }
}

#[test]
fn payment_intent_contract_excludes_refund_receipt_settlement_owners() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(root.join(
        "crates/foundation/macaca-proto/src/domain_pack_contract/commerce_payment_intent.rs",
    ))
    .unwrap();
    for forbidden in [
        "payment.refund",
        "receipt.issue",
        "settlement.reconcile",
        "payout.create",
    ] {
        assert!(!source.contains(forbidden));
    }
}
