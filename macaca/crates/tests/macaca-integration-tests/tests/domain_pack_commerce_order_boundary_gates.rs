use std::fs;
use std::path::Path;

#[test]
fn order_provider_is_runtime_host_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(
        root.join("crates/runtime/macaca-runtime-host/src/commerce_order_service_provider.rs"),
    )
    .unwrap();
    assert!(source.contains("impl SystemService"));
    assert!(!source.contains("payment_service_provider"));
    assert!(!source.contains("receipt_service_provider"));
}

#[test]
fn order_contract_excludes_external_side_effect_owners() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(
        root.join("crates/foundation/macaca-proto/src/domain_pack_contract/commerce_order.rs"),
    )
    .unwrap();
    for forbidden in [
        "payment.capture",
        "receipt.issue",
        "inventory.adjust",
        "carrier.purchase",
    ] {
        assert!(!source.contains(forbidden));
    }
}
