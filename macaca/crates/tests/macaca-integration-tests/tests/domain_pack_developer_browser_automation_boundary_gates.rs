use std::fs;
use std::path::Path;
#[test]
fn browser_provider_is_runtime_host_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(root.join(
        "crates/runtime/macaca-runtime-host/src/developer_browser_automation_service_provider.rs",
    ))
    .unwrap();
    assert!(source.contains("impl SystemService"));
    for forbidden in ["playwright", "puppeteer", "selenium", "chromedriver"] {
        assert!(!source.to_lowercase().contains(forbidden));
    }
}
#[test]
fn browser_contract_excludes_provider_native_commands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(root.join(
        "crates/foundation/macaca-proto/src/domain_pack_contract/developer_browser_automation.rs",
    ))
    .unwrap();
    for forbidden in ["playwright.", "cdp.", "webdriver.", "selenium."] {
        assert!(!source.to_lowercase().contains(forbidden));
    }
}
