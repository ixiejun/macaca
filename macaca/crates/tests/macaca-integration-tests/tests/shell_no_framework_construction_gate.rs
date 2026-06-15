//! Shell framework construction ownership gate.
//!
//! Web and CLI shells may provide host-local materialization adapters, but the
//! runtime-host construction service must own the public construction port. This
//! gate prevents the Web shell from reintroducing direct construction-port
//! ownership while the remaining materializer migration continues.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn read(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn web_shell_does_not_own_framework_construction_port() {
    let root = workspace_root();
    let adapter = read(
        &root,
        "crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs",
    );
    let web_wiring = read(
        &root,
        "crates/shells/macaca-web/src/web_agent_execution_adapters.rs",
    );

    assert!(
        !adapter.contains("WebFrameworkAgentConstructionPort"),
        "web shell must not define a framework construction port"
    );
    assert!(
        !adapter.contains("impl FrameworkAgentConstructionPort"),
        "web shell must not implement runtime-host construction ownership"
    );
    assert!(
        adapter.contains("impl FrameworkAgentMaterializationPort"),
        "web shell may only expose the lower-level materialization adapter"
    );
    assert!(
        adapter.contains(
            "materialize_runtime_react_agent_from_context_snapshot_with_execution_policy"
        ),
        "web shell materializer must expose materialization semantics, not construction ownership"
    );
    assert!(
        !adapter.contains("build_runtime_agent_from_context_snapshot"),
        "web shell materializer must not call FrameworkRunner snapshot construction helpers"
    );
    assert!(
        web_wiring.contains("RuntimeHostFrameworkAgentConstructionService"),
        "web wiring must route framework construction through runtime-host"
    );
}
