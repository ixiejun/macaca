//! P2 exit validation for `refactor-unified-call-path-microkernel` (tasks §3.8).
//!
//! These are static/architecture contract tests — they do not start network
//! services and intentionally avoid application-specific identifiers.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Forbidden microkernel module exports evicted during P2 purification.
const FORBIDDEN_KERNEL_MODULES: &[&str] = &[
    "pub mod web3",
    "pub mod evm",
    "pub mod a2a",
    "pub mod payment_policy",
    "pub mod provider_compat",
    "pub mod executor",
];

fn workspace_root() -> PathBuf {
    for ancestor in PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root");
}

#[test]
fn kernel_lib_exports_only_system_invariant_modules() {
    let kernel_lib = workspace_root().join("crates/kernel/macaca-kernel/src/lib.rs");
    let source = fs::read_to_string(&kernel_lib)
        .unwrap_or_else(|err| panic!("read {}: {err}", kernel_lib.display()));

    for forbidden in FORBIDDEN_KERNEL_MODULES {
        assert!(
            !source.contains(forbidden),
            "kernel lib.rs must not contain `{forbidden}` after P2 eviction"
        );
    }
}

#[test]
fn persist_cargo_has_no_production_dependency_on_context() {
    let persist_toml = workspace_root().join("crates/foundation/macaca-persist/Cargo.toml");
    let manifest = fs::read_to_string(&persist_toml)
        .unwrap_or_else(|err| panic!("read {}: {err}", persist_toml.display()));

    let in_dependencies = manifest
        .split("[dev-dependencies]")
        .next()
        .unwrap_or(&manifest);
    assert!(
        !in_dependencies.contains("macaca-context"),
        "macaca-persist production dependencies must not reference macaca-context"
    );
}

#[test]
fn cargo_tree_confirms_persist_context_edge_removed() {
    let output = Command::new("cargo")
        .args(["tree", "-e", "normal", "-p", "macaca-persist", "--depth", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("cargo tree should execute");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(
        !tree.contains("macaca-context"),
        "macaca-persist depth-1 tree must not list macaca-context:\n{tree}"
    );
    assert!(
        tree.contains("macaca-proto"),
        "macaca-persist should still depend on macaca-proto for lineage DTOs:\n{tree}"
    );
}
