//! Shell workspace dependency purity gate (P5 §4.4.4 / §6.1.7).
//!
//! Presentation shells (`macaca-web`, `macaca-cli`) must not accumulate direct workspace
//! edges to runtime, service, or application crates. Semantic ownership stays behind
//! `macaca-sdk` focused clients and `macaca-proto` DTO contracts.
//!
//! Design pattern: **Specification by Example** — `cargo metadata` is the executable
//! contract, mirroring `kernel_purity_gate` and `route_c_dependency_boundaries`.
//! Both CLI and Web shells are at **terminal purity** (proto + sdk only); the web
//! migration allowlist must remain empty so CI cannot regress into tolerated debt.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::allowlist::{
    PERMITTED_SHELL_WORKSPACE_DEPS, WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT,
};

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
        "shell_dependency_purity_gate event=metadata_start workspace={}",
        workspace_root().display()
    );
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute cargo metadata for shell dependency purity gate");
    assert!(
        output.status.success(),
        "cargo metadata failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "shell_dependency_purity_gate event=metadata_complete bytes={}",
        output.stdout.len()
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should emit valid JSON")
}

/// Returns sorted workspace `macaca-*` crate names that `package_name` depends on
/// through normal or build production edges (dev-dependencies are ignored).
fn shell_workspace_dependency_names(metadata: &Value, package_name: &str) -> BTreeSet<String> {
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

    let shell_package = packages
        .iter()
        .find(|package| package["name"].as_str() == Some(package_name))
        .unwrap_or_else(|| panic!("{package_name} must exist in workspace metadata"));

    let mut deps = BTreeSet::new();
    for dependency in shell_package["dependencies"]
        .as_array()
        .expect("shell dependencies should be an array")
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

fn assert_shell_terminal_purity(shell_crate: &str, observed: &BTreeSet<String>) {
    let permitted: BTreeSet<&str> = PERMITTED_SHELL_WORKSPACE_DEPS.iter().copied().collect();
    let forbidden: Vec<&str> = observed
        .iter()
        .map(|name| name.as_str())
        .filter(|name| !permitted.contains(name))
        .collect();

    if forbidden.is_empty() {
        eprintln!(
            "shell_dependency_purity_gate event=terminal_pass shell={shell_crate} reason=workspace_deps_pure"
        );
        return;
    }

    panic!(
        "Shell dependency purity gate failed for {shell_crate}: forbidden workspace dependencies: {forbidden:?}\n\
         Terminal invariant: shell may depend only on {PERMITTED_SHELL_WORKSPACE_DEPS:?}.\n\
         Route capability calls through macaca-sdk clients instead of adding direct crate edges."
    );
}

/// P5 §6.1.7 / §9.4 — explicit terminal assertion that web-shell migration allowlist is empty.
///
/// Mirrors `assert_route_c_allowlist_terminal_state` and
/// `assert_os_layer_file_size_allowlist_terminal_state`: CI must not regress into
/// tolerated workspace-dependency debt while the boundary scan reports zero violations.
/// Design pattern: **Specification by Example** with a dedicated terminal assertion
/// separate from the per-shell purity scan.
pub fn assert_web_shell_workspace_dependency_allowlist_terminal_state() {
    let row_count = WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT.len();
    eprintln!(
        "shell_dependency_purity_gate event=terminal_allowlist_check shell=macaca-web rows={row_count}"
    );
    if WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT.is_empty() {
        eprintln!(
            "shell_dependency_purity_gate event=terminal_allowlist_pass shell=macaca-web reason=zero_rows"
        );
        return;
    }

    let mut diagnostics = String::from(
        "Shell dependency purity gate failed: web migration allowlist must be empty at terminal state.\n\
         Remove each forbidden workspace edge from macaca-web/Cargo.toml instead of tolerating it.\n",
    );
    for row in WEB_SHELL_WORKSPACE_DEPENDENCY_DEBT {
        diagnostics.push_str(&format!(
            "  - {} (rationale: {}; replacement: {})\n",
            row.crate_name, row.rationale, row.replacement
        ));
    }
    panic!("{diagnostics}");
}

/// CLI shell: hard terminal assertion (proto + sdk only).
pub fn assert_cli_shell_workspace_dependency_purity() {
    let metadata = run_cargo_metadata();
    let observed = shell_workspace_dependency_names(&metadata, "macaca-cli");
    eprintln!(
        "shell_dependency_purity_gate event=cli_workspace_deps observed={observed:?}"
    );
    assert_shell_terminal_purity("macaca-cli", &observed);
}

/// Web shell: hard terminal assertion (proto + sdk only).
///
/// At §9.4 terminal state the web presentation shell must not depend directly on any
/// workspace crate other than `macaca-proto` and `macaca-sdk`. Capability calls route
/// through SDK focused clients; transitive edges behind `macaca-sdk` are permitted.
pub fn assert_web_shell_workspace_dependency_purity() {
    assert_web_shell_workspace_dependency_allowlist_terminal_state();

    let metadata = run_cargo_metadata();
    let observed = shell_workspace_dependency_names(&metadata, "macaca-web");
    eprintln!(
        "shell_dependency_purity_gate event=web_workspace_deps observed={observed:?}"
    );
    assert_shell_terminal_purity("macaca-web", &observed);
}
