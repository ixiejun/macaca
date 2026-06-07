//! Route C workspace topology guard.
//!
//! This test is executable architecture documentation. Route C no longer treats
//! all crates as flat peers: the filesystem must show whether a package belongs
//! to foundation contracts, the microkernel, replaceable services, runtime host
//! infrastructure, application framework, SDK facade, presentation shells, or
//! cross-layer tests. The dependency boundary gate still decides which crates
//! may depend on each other; this guard only verifies that package directories
//! live in the expected Route C layer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Finds the workspace root by walking upward until a Cargo workspace manifest
/// is found. This avoids hardcoding the number of path parents, which changed
/// when the integration-test crate moved from a flat `crates/` layout into the
/// Route C `crates/tests/` layer.
fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR");
}

/// Registry of expected package manifest suffixes.
///
/// The registry is intentionally explicit. Unknown workspace packages fail so
/// new crates cannot enter the repository without declaring their Route C layer
/// through OpenSpec and updating this executable topology specification.
fn expected_manifest_suffixes() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("macaca-proto", "crates/foundation/macaca-proto/Cargo.toml"),
        ("macaca-ipc", "crates/foundation/macaca-ipc/Cargo.toml"),
        (
            "macaca-persist",
            "crates/foundation/macaca-persist/Cargo.toml",
        ),
        ("macaca-kernel", "crates/kernel/macaca-kernel/Cargo.toml"),
        (
            "macaca-autonomy-evolution",
            "crates/services/macaca-autonomy-evolution/Cargo.toml",
        ),
        (
            "macaca-heartbeat",
            "crates/services/macaca-heartbeat/Cargo.toml",
        ),
        (
            "macaca-scheduled-agent-task",
            "crates/services/macaca-scheduled-agent-task/Cargo.toml",
        ),
        (
            "macaca-scheduler",
            "crates/services/macaca-scheduler/Cargo.toml",
        ),
        ("macaca-task", "crates/services/macaca-task/Cargo.toml"),
        ("macaca-llm", "crates/services/macaca-llm/Cargo.toml"),
        ("macaca-memory", "crates/services/macaca-memory/Cargo.toml"),
        (
            "macaca-context",
            "crates/services/macaca-context/Cargo.toml",
        ),
        ("macaca-driver", "crates/services/macaca-driver/Cargo.toml"),
        ("macaca-skill", "crates/services/macaca-skill/Cargo.toml"),
        (
            "macaca-gateway",
            "crates/services/macaca-gateway/Cargo.toml",
        ),
        ("macaca-tools", "crates/services/macaca-tools/Cargo.toml"),
        ("macaca-runtime", "crates/runtime/macaca-runtime/Cargo.toml"),
        (
            "macaca-runtime-host",
            "crates/runtime/macaca-runtime-host/Cargo.toml",
        ),
        (
            "macaca-framework",
            "crates/runtime/macaca-framework/Cargo.toml",
        ),
        ("macaca-agent", "crates/application/macaca-agent/Cargo.toml"),
        ("macaca-app", "crates/application/macaca-app/Cargo.toml"),
        ("macaca-sdk", "crates/facade/macaca-sdk/Cargo.toml"),
        ("macaca-web", "crates/shells/macaca-web/Cargo.toml"),
        ("macaca-cli", "crates/shells/macaca-cli/Cargo.toml"),
        (
            "macaca-integration-tests",
            "crates/tests/macaca-integration-tests/Cargo.toml",
        ),
    ])
}

fn run_cargo_metadata() -> Value {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should emit valid JSON")
}

fn workspace_package_paths(metadata: &Value) -> BTreeMap<String, String> {
    let workspace_members: BTreeSet<String> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members should be an array")
        .iter()
        .map(|id| {
            id.as_str()
                .expect("workspace member id should be a string")
                .to_owned()
        })
        .collect();

    let mut packages = BTreeMap::new();
    for package in metadata["packages"]
        .as_array()
        .expect("packages should be an array")
    {
        let package_id = package["id"]
            .as_str()
            .expect("package id should be a string");
        if !workspace_members.contains(package_id) {
            continue;
        }
        let name = package["name"]
            .as_str()
            .expect("package name should be a string")
            .to_owned();
        let manifest_path = package["manifest_path"]
            .as_str()
            .expect("manifest_path should be a string")
            .replace('\\', "/");
        packages.insert(name, manifest_path);
    }
    packages
}

#[test]
fn route_c_workspace_topology_matches_expected_layers() {
    let expected = expected_manifest_suffixes();
    let actual = workspace_package_paths(&run_cargo_metadata());

    let unknown: Vec<&String> = actual
        .keys()
        .filter(|name| !expected.contains_key(name.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "unknown workspace package(s) missing Route C topology classification: {unknown:?}. \
Update OpenSpec, macaca/crates/README.md, and route_c_workspace_topology.rs before relying on new crates."
    );

    let missing: Vec<&str> = expected
        .keys()
        .copied()
        .filter(|name| !actual.contains_key(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "expected Route C workspace package(s) missing from cargo metadata: {missing:?}"
    );

    let mut mismatches = Vec::new();
    for (name, expected_suffix) in expected {
        let manifest_path = actual
            .get(name)
            .expect("missing packages were checked above");
        if !manifest_path.ends_with(expected_suffix) {
            mismatches.push(format!(
                "{name}: expected suffix `{expected_suffix}`, actual `{manifest_path}`"
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "Route C workspace topology mismatch:\n{}",
        mismatches.join("\n")
    );
}
