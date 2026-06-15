//! P5 §7.2 — provider-neutral logging terminal gate implementation.
//!
//! Scans production Rust sources for tracing field names that encode application,
//! provider, or model identity as primary dimensions. OS orchestration layers must
//! log stable ids and service-neutral keys so audit replay stays application-agnostic.
//!
//! Design pattern: **Specification by Example** — forbidden field tokens and path
//! allowlists are the living policy; provider/model implementation crates may retain
//! vendor-specific tracing inside their bounded directories.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// One forbidden tracing field token and remediation hint.
struct ForbiddenTracingField {
    /// Substring that must not appear as a tracing structured field in scanned files.
    token: &'static str,
    /// Human-readable rationale for CI diagnostics.
    rationale: &'static str,
}

/// Production paths where provider/model/vendor tracing is expected (implementation layer).
const PATH_PREFIX_ALLOWLIST: &[&str] = &[
    "crates/services/macaca-llm/",
    "crates/services/macaca-memory/",
    "crates/packages/macaca-domain-pack-finance/",
    "crates/runtime/macaca-framework/src/model_impls/",
];

/// Tracing field names that must not appear outside the implementation allowlist.
///
/// Tokens use structured-field shapes (`field = %`) to avoid false positives on
/// local variables (`let llm_provider = ...`) or Rust comparisons (`agent.name ==`).
const FORBIDDEN_TRACING_FIELDS: &[ForbiddenTracingField] = &[
    ForbiddenTracingField {
        token: "app_name = %",
        rationale: "use application_id / app_id instead of manifest display name",
    },
    ForbiddenTracingField {
        token: "llm_provider = %",
        rationale: "use service_id + command + trace_id; provider identity belongs in macaca-llm",
    },
    ForbiddenTracingField {
        token: "llm_provider = self",
        rationale: "use service_id + command + trace_id; provider identity belongs in macaca-llm",
    },
    ForbiddenTracingField {
        token: "default_provider = %",
        rationale: "use config_scope or service_id instead of configured provider label",
    },
    ForbiddenTracingField {
        token: "embedding_provider = %",
        rationale: "use service_id=memory + embedding_configured flag; provider label belongs in macaca-memory",
    },
    ForbiddenTracingField {
        token: "embedding_model = %",
        rationale: "use embedding_dimensions / service_id instead of model name in OS logs",
    },
    ForbiddenTracingField {
        token: "agent.name = %",
        rationale: "use agent.id / agent_id instead of persona display name in spans",
    },
    ForbiddenTracingField {
        token: "model.name = %",
        rationale: "use service_id + command + trace_id instead of model display name in spans",
    },
    ForbiddenTracingField {
        token: "provider = %",
        rationale: "use service_id + command + trace_id; provider label belongs in macaca-llm/model_impls",
    },
    ForbiddenTracingField {
        token: "model = %",
        rationale: "use route_source / model_hint_present or service command result codes",
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

fn is_allowlisted_production_path(relative: &str) -> bool {
    PATH_PREFIX_ALLOWLIST
        .iter()
        .any(|prefix| relative.starts_with(prefix))
}

/// Unit/integration test modules colocated under `src/**/tests.rs` are not production surfaces.
fn is_colocated_test_module(relative: &str) -> bool {
    relative.ends_with("/tests.rs") || relative.ends_with("_tests.rs")
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*")
}

/// Returns true when the line is inside a `#[cfg(test)]` module (heuristic: skip test-only macros).
fn is_test_only_macro_line(line: &str) -> bool {
    line.contains("trace_agent_reply!")
        || line.contains("trace_model_chat!")
        || line.contains("trace_tool_exec!")
}

/// Scans non-allowlisted production sources for forbidden tracing field tokens.
fn assert_no_forbidden_tracing_fields(workspace: &Path) {
    eprintln!("provider_neutral_logging_terminal_gate event=scan_start");
    let crates_root = workspace.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let file_count = files.len();
    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let relative = relative_path(workspace, path.as_path());
        if is_allowlisted_production_path(&relative) || is_colocated_test_module(&relative) {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

        for (line_no, line) in content.lines().enumerate() {
            if is_comment_only_line(line) || is_test_only_macro_line(line) {
                continue;
            }
            for field in FORBIDDEN_TRACING_FIELDS {
                if line.contains(field.token) {
                    violations.push(format!(
                        "{relative}:{}: forbidden tracing field `{}` — {}",
                        line_no + 1,
                        field.token,
                        field.rationale
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        violations.sort();
        panic!(
            "provider-neutral logging terminal gate violations:\n{}",
            violations.join("\n")
        );
    }

    eprintln!("provider_neutral_logging_terminal_gate event=scan_pass files={file_count}");
}

/// Main gate entry — panics when OS-layer logs use provider/app/model primary keys.
pub fn assert_provider_neutral_logging_terminal_state() {
    let workspace = workspace_root();
    assert_no_forbidden_tracing_fields(&workspace);
    eprintln!("provider_neutral_logging_terminal_gate event=terminal_pass");
}
