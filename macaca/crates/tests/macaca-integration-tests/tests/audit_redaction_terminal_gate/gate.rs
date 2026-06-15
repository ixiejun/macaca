//! P5 §7.3 — canonical audit redaction terminal gate implementation.
//!
//! Enforces that OS-layer audit/trace/snapshot export paths delegate to
//! `macaca_proto::audit_redaction` instead of duplicating local sanitizer logic.
//!
//! Design pattern: **Specification by Example** — required contract sources and
//! forbidden duplicate-function shapes are the living redaction policy; **Command**
//! subprocess delegation runs proto unit tests for behavioral proof.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Canonical redaction module that owns export vs inline-redaction strategies.
const CANONICAL_MODULE: &str = "crates/foundation/macaca-proto/src/audit_redaction.rs";

/// Production audit export surfaces that MUST wire `audit_redaction` imports.
const REQUIRED_WIRED_SURFACES: &[&str] = &[
    "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge/routing_support.rs",
    "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/guest_harness_support.rs",
    "crates/runtime/macaca-runtime-host/src/application_execution_event_store.rs",
    "crates/runtime/macaca-runtime-host/src/execution_control_runtime.rs",
    "crates/runtime/macaca-runtime-host/src/execution_control_service_provider.rs",
];

/// Thin delegation wrappers may keep a local `fn sanitize_json` name when the body
/// forwards to the canonical harness/export API (Adapter pattern).
const APPROVED_DELEGATION_WRAPPERS: &[&str] =
    &["crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/guest_harness_support.rs"];

/// Marker substrings that indicate a duplicate redaction implementation (not a delegate).
const DUPLICATE_IMPL_MARKERS: &[&str] = &[
    "EXPORT_OMIT_KEY_MARKERS",
    "INLINE_SENSITIVE_KEY_MARKERS",
    "fn is_safe_metadata_key(key: &str)",
    "fn is_sensitive_key(key: &str)",
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

/// Verifies canonical module and lib export exist before scanning dependents.
fn assert_canonical_module_present(workspace: &Path) {
    eprintln!("audit_redaction_terminal_gate event=canonical_module_check");
    let module_path = workspace.join(CANONICAL_MODULE);
    assert!(
        module_path.is_file(),
        "canonical audit_redaction module missing: {}",
        module_path.display()
    );

    let lib = std::fs::read_to_string(workspace.join("crates/foundation/macaca-proto/src/lib.rs"))
        .expect("macaca-proto lib.rs should be readable");
    assert!(
        lib.contains("pub mod audit_redaction"),
        "macaca-proto lib.rs must export pub mod audit_redaction"
    );
    eprintln!("audit_redaction_terminal_gate event=canonical_module_pass");
}

/// Each known audit export surface must reference `audit_redaction` in its imports.
fn assert_required_surfaces_wired(workspace: &Path) {
    eprintln!("audit_redaction_terminal_gate event=required_surface_check");
    for relative in REQUIRED_WIRED_SURFACES {
        let path = workspace.join(relative);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            content.contains("audit_redaction"),
            "audit export surface must import audit_redaction: {relative}"
        );
    }
    eprintln!(
        "audit_redaction_terminal_gate event=required_surface_pass count={}",
        REQUIRED_WIRED_SURFACES.len()
    );
}

/// Returns true when a local `fn sanitize_json` is an approved thin delegate.
fn is_approved_delegation_wrapper(relative: &str, content: &str) -> bool {
    if !APPROVED_DELEGATION_WRAPPERS.contains(&relative) {
        return false;
    }
    content.contains("sanitize_json_for_harness")
        || content.contains("audit_redaction::sanitize_json")
}

/// Scans production sources for duplicate sanitizer implementations outside proto.
fn assert_no_duplicate_local_sanitizers(workspace: &Path) {
    eprintln!("audit_redaction_terminal_gate event=duplicate_scan_start");
    let crates_root = workspace.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let file_count = files.len();
    let mut violations: Vec<String> = Vec::new();
    for path in files {
        let relative = relative_path(workspace, &path);
        if relative == CANONICAL_MODULE {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

        if content.contains("fn sanitize_json(")
            && !is_approved_delegation_wrapper(&relative, &content)
        {
            violations.push(format!(
                "{relative}: local fn sanitize_json must delegate to macaca_proto::audit_redaction"
            ));
        }

        for marker in DUPLICATE_IMPL_MARKERS {
            if content.contains(marker) {
                violations.push(format!(
                    "{relative}: duplicate redaction marker `{marker}` — use audit_redaction"
                ));
            }
        }
    }

    if !violations.is_empty() {
        violations.sort();
        panic!(
            "audit redaction terminal gate: duplicate local sanitizer implementations:\n{}",
            violations.join("\n")
        );
    }

    eprintln!("audit_redaction_terminal_gate event=duplicate_scan_pass files={file_count}");
}

/// Runs proto unit tests as behavioral proof of the canonical redaction API.
fn run_proto_redaction_unit_tests(workspace: &Path) {
    eprintln!("audit_redaction_terminal_gate event=subprocess_start package=macaca-proto filter=audit_redaction");
    let output = Command::new("cargo")
        .args([
            "test",
            "-p",
            "macaca-proto",
            "audit_redaction",
            "--",
            "--nocapture",
        ])
        .current_dir(workspace)
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to spawn cargo test -p macaca-proto audit_redaction: {error}")
        });

    assert!(
        output.status.success(),
        "audit redaction proto unit tests failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("audit_redaction_terminal_gate event=subprocess_pass");
}

/// Main gate entry — panics when canonical wiring or duplicate sanitizers are detected.
pub fn assert_audit_redaction_terminal_state() {
    let workspace = workspace_root();
    assert_canonical_module_present(&workspace);
    assert_required_surfaces_wired(&workspace);
    assert_no_duplicate_local_sanitizers(&workspace);
    run_proto_redaction_unit_tests(&workspace);
    eprintln!("audit_redaction_terminal_gate event=terminal_pass");
}
