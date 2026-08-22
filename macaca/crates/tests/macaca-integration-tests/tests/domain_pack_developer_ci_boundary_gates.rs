use std::{fs, path::Path};
#[test]
fn ci_provider_is_runtime_host_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let s = fs::read_to_string(
        root.join("crates/runtime/macaca-runtime-host/src/developer_ci_service_provider.rs"),
    )
    .unwrap();
    assert!(s.contains("impl SystemService"));
    for x in ["github", "gitlab", "circleci", "jenkins"] {
        assert!(!s.to_lowercase().contains(x));
    }
}
