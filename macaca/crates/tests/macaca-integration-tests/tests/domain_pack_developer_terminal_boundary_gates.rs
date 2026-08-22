use std::{fs, path::Path};
#[test]
fn terminal_provider_is_runtime_host_only_and_provider_neutral() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let s = fs::read_to_string(
        root.join("crates/runtime/macaca-runtime-host/src/developer_terminal_service_provider.rs"),
    )
    .unwrap();
    assert!(s.contains("impl SystemService"));
    for n in ["pty", "ssh", "docker", "child_process", "tokio::process"] {
        assert!(!s.to_lowercase().contains(n));
    }
}
