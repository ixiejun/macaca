//! P5 §7.4 — audit blind-spot terminal gate implementation.
//!
//! After bypass retirement, every serviceized execution path must emit structured
//! `service.call` evidence that can be replayed by `trace_id` or `session_id` via
//! `system.service_audit` (`service.audit.replay.trace` / `.session`).
//!
//! Design pattern: **Facade + Command + Specification by Example**
//! - Static checks encode the replay command surface and bootstrap wiring contract.
//! - A production scan rejects router construction without an audit sink attachment.
//! - `cargo test` subprocesses prove replay behavior on the in-memory audit sink.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Canonical audit modules that own replay storage and command adapters.
const REQUIRED_AUDIT_MODULES: &[&str] = &[
    "crates/runtime/macaca-runtime-host/src/service_call_audit.rs",
    "crates/runtime/macaca-runtime-host/src/service_call_audit_service_provider.rs",
    "crates/runtime/macaca-runtime-host/src/service_audit_runtime_bundle.rs",
];

/// Replay command constants that must remain stable for external audit tooling.
const REQUIRED_REPLAY_COMMAND_MARKERS: &[&str] = &[
    "service.audit.replay.trace",
    "service.audit.replay.session",
    "system.service_audit",
];

/// Web composition root that must bind the shared audit bundle at startup.
const WEB_BOOTSTRAP_SURFACE: &str =
    "crates/shells/macaca-web/src/composition_bootstrap/application_discovery.rs";

/// Substrings the web bootstrap must retain so replay and routing share one sink.
const REQUIRED_BOOTSTRAP_MARKERS: &[&str] = &[
    "ServiceAuditRuntimeBundle",
    "audit_service_provider_instance",
    "wasm_host_import_bridge",
    "SERVICE_CALL_AUDIT_SERVICE_ID",
];

/// Service router must emit lifecycle stages and expose replay query helpers.
const SERVICE_ROUTER_SURFACE: &str = "crates/runtime/macaca-runtime-host/src/service_router.rs";

const REQUIRED_ROUTER_MARKERS: &[&str] = &[
    "emit_audit_event",
    "service_call_requested",
    "replay_audit_by_trace_id",
    "replay_audit_by_session_id",
];

/// Files allowed to mention `audit_sink: None` (builder default only).
const AUDIT_SINK_NONE_ALLOWLIST: &[&str] =
    &["crates/runtime/macaca-runtime-host/src/service_router.rs"];

/// Marker substrings that indicate an intentional audit bypass in production code.
const FORBIDDEN_BYPASS_MARKERS: &[&str] = &[
    "skip_service_call_audit",
    "disable_service_call_audit",
    "without_audit_sink",
    "no_audit_emit",
];

/// Downstream crate test invocation executed by this gate.
struct CrateTestInvocation {
    /// Cargo package name (`-p` argument).
    package: &'static str,
    /// Test name filter passed to `cargo test`.
    filter: &'static str,
}

const DOWNSTREAM_INVOCATIONS: &[CrateTestInvocation] = &[
    CrateTestInvocation {
        package: "macaca-runtime-host",
        filter: "service_call_audit",
    },
    CrateTestInvocation {
        package: "macaca-runtime-host",
        filter: "service_audit_runtime_bundle",
    },
    CrateTestInvocation {
        package: "macaca-runtime-host",
        filter: "service_call_audit_provider",
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

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "tests" | "fixtures" | "examples" | ".git" | ".playwright-mcp"
    )
}

fn is_production_scan_candidate(relative: &str) -> bool {
    if relative.contains("/tests/") || relative.ends_with("_tests.rs") {
        return false;
    }
    relative.starts_with("crates/shells/")
        || relative.starts_with("crates/runtime/macaca-runtime-host/src/")
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if should_skip_dir(name) {
            continue;
        }
        if path.is_dir() {
            collect_rust_files(&path, files);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

/// Verifies canonical audit modules export replay commands before subprocess work.
fn assert_replay_command_surface_present(workspace: &Path) {
    eprintln!("audit_blind_spot_terminal_gate event=replay_surface_check");
    for relative in REQUIRED_AUDIT_MODULES {
        let path = workspace.join(relative);
        assert!(
            path.is_file(),
            "audit replay module missing: {}",
            path.display()
        );
    }

    let provider = std::fs::read_to_string(
        workspace
            .join("crates/runtime/macaca-runtime-host/src/service_call_audit_service_provider.rs"),
    )
    .expect("service_call_audit_service_provider.rs should be readable");
    for marker in REQUIRED_REPLAY_COMMAND_MARKERS {
        assert!(
            provider.contains(marker),
            "replay command marker missing from provider: {marker}"
        );
    }

    let lib =
        std::fs::read_to_string(workspace.join("crates/runtime/macaca-runtime-host/src/lib.rs"))
            .expect("macaca-runtime-host lib.rs should be readable");
    for module in [
        "pub mod service_call_audit",
        "pub mod service_call_audit_service_provider",
        "pub mod service_audit_runtime_bundle",
    ] {
        assert!(
            lib.contains(module),
            "macaca-runtime-host lib.rs must export {module}"
        );
    }

    eprintln!("audit_blind_spot_terminal_gate event=replay_surface_pass");
}

/// Web startup must register the shared audit bundle and replay system service.
fn assert_web_bootstrap_wires_shared_audit_bundle(workspace: &Path) {
    eprintln!("audit_blind_spot_terminal_gate event=bootstrap_wiring_check");
    let path = workspace.join(WEB_BOOTSTRAP_SURFACE);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    for marker in REQUIRED_BOOTSTRAP_MARKERS {
        assert!(
            content.contains(marker),
            "web bootstrap must wire audit replay surface: missing `{marker}` in {}",
            WEB_BOOTSTRAP_SURFACE
        );
    }
    eprintln!("audit_blind_spot_terminal_gate event=bootstrap_wiring_pass");
}

/// Router lifecycle must always emit audit events and expose replay query helpers.
fn assert_service_router_emits_replayable_evidence(workspace: &Path) {
    eprintln!("audit_blind_spot_terminal_gate event=router_contract_check");
    let path = workspace.join(SERVICE_ROUTER_SURFACE);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    for marker in REQUIRED_ROUTER_MARKERS {
        assert!(
            content.contains(marker),
            "service router audit contract missing `{marker}`"
        );
    }
    eprintln!("audit_blind_spot_terminal_gate event=router_contract_pass");
}

/// Rejects production router construction without `with_audit_sink` attachment.
fn assert_no_unaudited_service_router_construction(workspace: &Path) {
    eprintln!("audit_blind_spot_terminal_gate event=router_bypass_scan_start");
    let crates_root = workspace.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut violations: Vec<String> = Vec::new();
    for path in files {
        let relative = relative_path(workspace, &path);
        if !is_production_scan_candidate(&relative) {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

        if content.contains("ServiceRouter::new") && !content.contains("with_audit_sink") {
            violations.push(format!(
                "{relative}: ServiceRouter::new without with_audit_sink — service.call evidence would be unaudited"
            ));
        }

        if content.contains("audit_sink: None")
            && !AUDIT_SINK_NONE_ALLOWLIST.contains(&relative.as_str())
        {
            violations.push(format!(
                "{relative}: audit_sink: None outside builder default — replay would be blind"
            ));
        }

        for marker in FORBIDDEN_BYPASS_MARKERS {
            if content.contains(marker) {
                violations.push(format!(
                    "{relative}: forbidden audit bypass marker `{marker}`"
                ));
            }
        }
    }

    if !violations.is_empty() {
        violations.sort();
        panic!(
            "audit blind spot terminal gate: unaudited execution patterns detected:\n{}",
            violations.join("\n")
        );
    }

    eprintln!("audit_blind_spot_terminal_gate event=router_bypass_scan_pass");
}

/// Runs one filtered `cargo test` subprocess and panics with stderr on failure.
fn run_crate_test(workspace: &Path, package: &str, filter: &str) {
    eprintln!(
        "audit_blind_spot_terminal_gate event=subprocess_start package={package} filter={filter}"
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
            "audit_blind_spot_terminal_gate event=subprocess_pass package={package} filter={filter}"
        );
        return;
    }

    panic!(
        "audit blind spot terminal gate: `cargo test -p {package} {filter}` failed \
         (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Main gate entry — panics when replay wiring or behavioral contracts regress.
pub fn assert_audit_blind_spot_terminal_state() {
    let workspace = workspace_root();
    assert_replay_command_surface_present(&workspace);
    assert_web_bootstrap_wires_shared_audit_bundle(&workspace);
    assert_service_router_emits_replayable_evidence(&workspace);
    assert_no_unaudited_service_router_construction(&workspace);

    for invocation in DOWNSTREAM_INVOCATIONS {
        run_crate_test(workspace.as_path(), invocation.package, invocation.filter);
    }

    eprintln!("audit_blind_spot_terminal_gate event=terminal_pass");
}
