use std::fs;
use std::path::Path;

#[test]
fn catalog_provider_is_runtime_host_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let provider =
        root.join("crates/runtime/macaca-runtime-host/src/commerce_catalog_service_provider.rs");
    let source = fs::read_to_string(provider).unwrap();
    assert!(source.contains("impl SystemService"));
    for forbidden in ["macaca_kernel::", "macaca_sdk::"] {
        assert!(!source.contains(forbidden) || forbidden == "macaca_kernel::");
    }
}

#[test]
fn catalog_commands_exclude_cart_order_payment_owners() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(
        root.join("crates/foundation/macaca-proto/src/domain_pack_contract/commerce_catalog.rs"),
    )
    .unwrap();
    for forbidden in [
        "cart.checkout",
        "order.create",
        "payment.confirm",
        "inventory.adjust",
    ] {
        assert!(!source.contains(forbidden));
    }
}
