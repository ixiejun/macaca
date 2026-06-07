//! Kernel microkernel purity gate (P5 §6.1.5).
//!
//! The Macaca kernel must remain a **constitutional** layer: system invariants,
//! scheduling primitives, and protocol-facing facades only. Replaceable provider
//! implementations (LLM, memory, task, driver, gateway, etc.) belong in
//! service-runtime / runtime-host composition roots, not as direct `Cargo.toml`
//! edges on `macaca-kernel`.
//!
//! This gate audits **workspace** direct dependencies only — external crates
//! (`tokio`, `serde`, …) are expected infrastructure. The invariant matches
//! task §3.6.6: `macaca-kernel` may depend on `macaca-proto` and `macaca-ipc`
//! alone among workspace members.
//!
//! Design pattern: **Specification by Example** — `cargo metadata` is the
//! executable contract; failures name the forbidden workspace crate and the
//! service-client replacement boundary.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Workspace crate names permitted as direct production dependencies of `macaca-kernel`.
const PERMITTED_KERNEL_WORKSPACE_DEPS: &[&str] = &["macaca-proto", "macaca-ipc"];

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from integration-tests manifest");
}

fn run_cargo_metadata() -> Value {
    eprintln!(
        "kernel_purity_gate event=metadata_start workspace={}",
        workspace_root().display()
    );
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute cargo metadata for kernel purity gate");
    assert!(
        output.status.success(),
        "cargo metadata failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "kernel_purity_gate event=metadata_complete bytes={}",
        output.stdout.len()
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should emit valid JSON")
}

/// Returns sorted workspace `macaca-*` crate names that `macaca-kernel` depends on
/// through normal or build production edges (dev-dependencies are ignored).
fn kernel_workspace_dependency_names(metadata: &Value) -> BTreeSet<String> {
    let workspace_members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members should be an array")
        .iter()
        .map(|id| id.as_str().expect("workspace member id should be a string"))
        .collect();

    let packages = metadata["packages"]
        .as_array()
        .expect("packages should be an array");

    let workspace_names: BTreeSet<&str> = packages
        .iter()
        .filter(|package| {
            workspace_members.contains(package["id"].as_str().unwrap_or_default())
        })
        .map(|package| package["name"].as_str().expect("package name should be a string"))
        .collect();

    let kernel_package = packages
        .iter()
        .find(|package| package["name"].as_str() == Some("macaca-kernel"))
        .expect("macaca-kernel must exist in workspace metadata");

    let mut deps = BTreeSet::new();
    for dependency in kernel_package["dependencies"]
        .as_array()
        .expect("kernel dependencies should be an array")
    {
        let kind = dependency["kind"].as_str();
        if matches!(kind, Some("dev")) {
            continue;
        }
        let name = dependency["name"]
            .as_str()
            .expect("dependency name should be a string");
        if workspace_names.contains(name) {
            deps.insert(name.to_string());
        }
    }
    deps
}

/// Main gate entry — panics with replacement guidance when kernel depends on
/// disallowed workspace crates.
pub fn assert_kernel_workspace_dependency_purity() {
    let metadata = run_cargo_metadata();
    let observed = kernel_workspace_dependency_names(&metadata);
    let permitted: BTreeSet<&str> = PERMITTED_KERNEL_WORKSPACE_DEPS.iter().copied().collect();

    eprintln!(
        "kernel_purity_gate event=workspace_deps observed={:?} permitted={:?}",
        observed, permitted
    );

    let mut forbidden: Vec<&str> = observed
        .iter()
        .map(|name| name.as_str())
        .filter(|name| !permitted.contains(name))
        .collect();
    forbidden.sort();

    if forbidden.is_empty() {
        eprintln!("kernel_purity_gate event=pass reason=kernel_workspace_deps_pure");
        return;
    }

    panic!(
        "Kernel purity gate failed: macaca-kernel has forbidden workspace dependencies: {forbidden:?}\n\
         Terminal invariant: kernel may depend only on {PERMITTED_KERNEL_WORKSPACE_DEPS:?}.\n\
         Move provider/runtime coupling behind ServiceRuntime, runtime-host providers, or SDK clients."
    );
}
