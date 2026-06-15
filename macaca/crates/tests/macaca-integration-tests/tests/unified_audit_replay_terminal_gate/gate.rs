//! P5 §9.1 — unified audit replay single-chain terminal gate.
//!
//! YAML and WASM agent execution must each converge on exactly one
//! `service.agent_execution` evidence chain per session (see `audit-replay-baseline.md`).
//! Crate-level contract tests encode the static topology; runtime-host replay tests
//! exercise the in-memory audit sink contract without live LLM credentials.
//!
//! Design pattern: **Facade + Command** — this integration gate orchestrates downstream
//! crate test suites via `cargo test` subprocesses so CI has a single named VC entry
//! point (`unified_audit_replay_terminal_gate`) aligned with protocol / file-size gates.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Relative paths to contract modules that must exist for audit-replay convergence.
const REQUIRED_CONTRACT_SOURCES: &[&str] = &[
    "crates/shells/macaca-web/src/unified_audit_replay_convergence_tests.rs",
    "crates/runtime/macaca-runtime-host/src/unified_audit_replay_convergence_tests.rs",
];

/// One downstream crate test invocation executed by this gate.
struct CrateTestInvocation {
    /// Cargo package name (`-p` argument).
    package: &'static str,
    /// Test name filter passed to `cargo test`.
    filter: &'static str,
}

const DOWNSTREAM_INVOCATIONS: &[CrateTestInvocation] = &[
    CrateTestInvocation {
        package: "macaca-web",
        filter: "unified_audit_replay_convergence",
    },
    CrateTestInvocation {
        package: "macaca-runtime-host",
        filter: "unified_audit_replay_convergence",
    },
];

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

/// Verifies contract source files are present before spawning compile-heavy subprocesses.
fn assert_contract_sources_present(workspace: &Path) {
    eprintln!("unified_audit_replay_terminal_gate event=contract_source_check");
    for relative in REQUIRED_CONTRACT_SOURCES {
        let path = workspace.join(relative);
        assert!(
            path.is_file(),
            "unified audit replay contract source missing: {}",
            path.display()
        );
    }
    eprintln!(
        "unified_audit_replay_terminal_gate event=contract_source_pass count={}",
        REQUIRED_CONTRACT_SOURCES.len()
    );
}

/// Runs one filtered `cargo test` subprocess and panics with stderr on failure.
fn run_crate_test(workspace: &Path, package: &str, filter: &str) {
    eprintln!(
        "unified_audit_replay_terminal_gate event=subprocess_start package={package} filter={filter}"
    );
    let output = Command::new("cargo")
        .args(["test", "-p", package, filter, "--", "--nocapture"])
        .current_dir(workspace)
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to spawn cargo test -p {package} {filter}: {error}")
        });

    if output.status.success() {
        eprintln!(
            "unified_audit_replay_terminal_gate event=subprocess_pass package={package} filter={filter}"
        );
        return;
    }

    panic!(
        "unified audit replay terminal gate: `cargo test -p {package} {filter}` failed \
         (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Main gate entry — panics when contract sources are missing or downstream tests fail.
pub fn assert_unified_audit_replay_single_chain_terminal_state() {
    let workspace = workspace_root();
    assert_contract_sources_present(&workspace);

    for invocation in DOWNSTREAM_INVOCATIONS {
        run_crate_test(workspace.as_path(), invocation.package, invocation.filter);
    }

    eprintln!("unified_audit_replay_terminal_gate event=terminal_pass");
}
