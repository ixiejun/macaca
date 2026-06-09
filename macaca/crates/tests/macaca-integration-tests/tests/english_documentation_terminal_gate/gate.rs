//! P5 §7.1 — English module documentation terminal gate implementation.
//!
//! Validates that P5 migrated modules and terminal gate sources include a
//! module-level `//!` doc block with enough English prose to explain what the
//! file does and how it participates in the microkernel refactor.
//!
//! Design pattern: **Specification by Example + Registry** — explicit file
//! registry for split runtime-host modules plus directory scans for web bootstrap
//! and integration-test gate trees; co-located `*_tests.rs` fixtures are excluded.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Minimum non-empty `//!` lines required at the top of each scanned source file.
const MIN_MODULE_DOC_LINES: usize = 3;

/// Minimum total characters in the module doc block (body text only).
const MIN_MODULE_DOC_CHARS: usize = 80;

/// Directory prefixes scanned recursively for production Rust sources.
const SCAN_DIRECTORY_PREFIXES: &[&str] = &[
    "crates/shells/macaca-web/src/composition_bootstrap/",
    "crates/tests/macaca-integration-tests/tests/p5_dod_terminal_gate_matrix/",
];

/// Explicit production/migration files outside directory scans (split modules).
const REGISTRY_SOURCE_FILES: &[&str] = &[
    "crates/runtime/macaca-runtime-host/src/factory.rs",
    "crates/runtime/macaca-runtime-host/src/transport.rs",
    "crates/runtime/macaca-runtime-host/src/lease.rs",
    "crates/foundation/macaca-proto/src/audit_redaction.rs",
    "crates/runtime/macaca-framework/src/tracing_utils.rs",
    "crates/application/macaca-agent/src/in_process_execution.rs",
    "crates/application/macaca-app/src/service_admission.rs",
];

/// Terminal gate facade + implementation files (integration-test VC surface).
const TERMINAL_GATE_SOURCE_FILES: &[&str] = &[
    "crates/tests/macaca-integration-tests/tests/unified_audit_replay_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/unified_audit_replay_terminal_gate/gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_redaction_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_redaction_terminal_gate/gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_blind_spot_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/audit_blind_spot_terminal_gate/gate.rs",
    "crates/tests/macaca-integration-tests/tests/openspec_validate_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/p5_dod_terminal_gate_matrix.rs",
    "crates/tests/macaca-integration-tests/tests/p5_dod_terminal_gate_matrix/gate.rs",
    "crates/tests/macaca-integration-tests/tests/provider_neutral_logging_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/provider_neutral_logging_terminal_gate/gate.rs",
    "crates/tests/macaca-integration-tests/tests/english_documentation_terminal_gate.rs",
    "crates/tests/macaca-integration-tests/tests/english_documentation_terminal_gate/gate.rs",
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

/// Returns true when the path is a co-located unit/integration test module.
fn is_test_only_source(relative: &str) -> bool {
    relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("/tests.rs")
}

/// Collects `.rs` files under `dir` recursively, returning workspace-relative paths.
fn collect_rust_sources_under(dir: &Path, workspace: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|error| {
        panic!(
            "english_documentation_terminal_gate: failed to read dir {}: {error}",
            dir.display()
        )
    });

    for entry in entries {
        let entry = entry.expect("english_documentation_terminal_gate: read_dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources_under(&path, workspace, out);
            continue;
        }
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let relative = path
            .strip_prefix(workspace)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_test_only_source(&relative) {
            continue;
        }
        out.push(path);
    }
}

/// Parses the leading `//!` block and returns (line_count, char_count) of doc body.
fn module_doc_stats(content: &str) -> (usize, usize) {
    let mut lines = 0usize;
    let mut chars = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//!") {
            break;
        }
        let body = trimmed.strip_prefix("//!").unwrap_or("").trim_start();
        if !body.is_empty() {
            lines += 1;
            chars += body.len();
        }
    }

    (lines, chars)
}

/// Validates one source file has sufficient English module documentation.
fn assert_file_has_module_documentation(workspace: &Path, path: &Path) {
    let relative = path
        .strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    let content = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "english_documentation_terminal_gate: failed to read {}: {error}",
            path.display()
        )
    });

    let (doc_lines, doc_chars) = module_doc_stats(&content);
    let first_line = content.lines().next().unwrap_or("");

    assert!(
        first_line.trim_start().starts_with("//!"),
        "english_documentation_terminal_gate: missing module doc block at top of {relative} \
         (expected `//!` explaining purpose + runtime behavior + trade-offs)"
    );
    assert!(
        doc_lines >= MIN_MODULE_DOC_LINES,
        "english_documentation_terminal_gate: insufficient module doc lines in {relative} \
         (found {doc_lines}, need >= {MIN_MODULE_DOC_LINES})"
    );
    assert!(
        doc_chars >= MIN_MODULE_DOC_CHARS,
        "english_documentation_terminal_gate: insufficient module doc detail in {relative} \
         (found {doc_chars} chars, need >= {MIN_MODULE_DOC_CHARS})"
    );
}

/// Main gate entry — panics when any scanned P5 module lacks English module docs.
pub fn assert_english_documentation_terminal_state() {
    let workspace = workspace_root();
    let mut files: Vec<PathBuf> = Vec::new();

    eprintln!("english_documentation_terminal_gate event=scan_start");

    for prefix in SCAN_DIRECTORY_PREFIXES {
        let dir = workspace.join(prefix);
        assert!(
            dir.is_dir(),
            "english_documentation_terminal_gate: scan directory missing: {}",
            dir.display()
        );
        collect_rust_sources_under(&dir, &workspace, &mut files);
    }

    for relative in REGISTRY_SOURCE_FILES
        .iter()
        .chain(TERMINAL_GATE_SOURCE_FILES.iter())
    {
        let path = workspace.join(relative);
        assert!(
            path.is_file(),
            "english_documentation_terminal_gate: registry file missing: {}",
            path.display()
        );
        files.push(path);
    }

    files.sort();
    files.dedup();

    let file_count = files.len();
    for path in &files {
        assert_file_has_module_documentation(&workspace, path);
    }

    eprintln!(
        "english_documentation_terminal_gate event=scan_pass files={file_count}"
    );
    eprintln!("english_documentation_terminal_gate event=terminal_pass");
}
