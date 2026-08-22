use std::{fs, path::Path};
#[test]
fn issue_tracker_provider_is_runtime_host_only_and_provider_neutral() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = fs::read_to_string(root.join(
        "crates/runtime/macaca-runtime-host/src/developer_issue_tracker_service_provider.rs",
    ))
    .unwrap();
    assert!(source.contains("impl SystemService"));
    for name in ["github", "gitlab", "jira", "linear", "graphql"] {
        assert!(!source.to_lowercase().contains(name));
    }
}
