//! Static scope-control guard for the application execution protocol platform.
//!
//! This module is an executable governance specification rather than production
//! platform logic. The Specification pattern is intentionally used here: each
//! helper states one boundary predicate, and the final assertion composes those
//! predicates into a deterministic CI gate. The platform may carry application,
//! provider, model, workflow, driver, gateway, and business-domain identities as
//! data, but generic OS/service/runtime code must not branch on those identities
//! to select product-specific behavior.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Evidence for one forbidden product-identity branch.
///
/// The failure payload is deliberately structured so a reviewer can audit the
/// exact file, line, identity, and branch shape without reading the scanner
/// internals. The test logs only bounded metadata and never prints source lines,
/// which keeps CI output useful without leaking application payloads.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ScopeBranchViolation {
    path: PathBuf,
    line: usize,
    identity: String,
    branch_pattern: &'static str,
}

/// Find the Cargo workspace root from the integration-test manifest.
///
/// Integration tests can be launched from different working directories. Walking
/// ancestors avoids hardcoding repository paths and keeps the gate portable
/// across local development, CI, and future crate moves.
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

/// Recursively collect application-execution production Rust files below
/// `crates/`.
///
/// Test, fixture, and example code is intentionally excluded because those
/// areas often contain literal application names as negative examples. The gate
/// targets production ownership boundaries only. The 12.4 proposal task is
/// specifically about the application execution protocol platform, so the
/// scanner is deliberately scoped to files whose names contain
/// `application_execution`; broader legacy branch cleanup belongs to separate
/// governance gates and must not make this slice noisy.
fn collect_production_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if matches!(
            name,
            "target" | "tests" | "fixtures" | "examples" | ".git" | ".playwright-mcp"
        ) {
            continue;
        }
        if path.is_dir() {
            collect_production_rust_files(&path, files);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some("rs")
            && name.contains("application_execution")
            && !name.ends_with("_tests.rs")
        {
            files.push(path);
        }
    }
}

/// Return true when a path is an approved owner for identity metadata.
///
/// The application and foundation crates own protocol DTOs, manifest metadata,
/// and typed identity fields. They may validate or serialize those identities as
/// data. The LLM service allowance mirrors the existing migration surface for
/// provider/model strategy ownership; it is not an endorsement for new generic
/// application execution branches.
fn is_approved_scope_owner(relative: &str, identity: &str) -> bool {
    relative.starts_with("crates/application/")
        || relative.starts_with("crates/foundation/")
        || (matches!(identity, "provider_name" | "model_name")
            && relative.starts_with("crates/services/macaca-llm/src/"))
}

/// Skip comment-only lines so explanatory text cannot produce false positives.
fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
}

/// Recognize obvious literal branch shapes without implementing a Rust parser.
///
/// This gate intentionally checks high-signal patterns such as string equality,
/// containment, prefix/suffix checks, `matches!`, and `match`. DTO declarations,
/// telemetry field names, and ordinary descriptors are allowed because they do
/// not route behavior by product identity.
fn branch_pattern(line: &str) -> Option<&'static str> {
    [
        "== \"",
        "!= \"",
        ".contains(\"",
        ".starts_with(\"",
        ".ends_with(\"",
        "matches!(",
        "match ",
    ]
    .into_iter()
    .find(|pattern| line.contains(pattern))
}

/// Identity names that generic service/runtime code must not branch on.
///
/// The guard focuses on names and business-domain identifiers because those are
/// the high-risk signals for product-specific routing. Opaque provider,
/// application, and session ids are allowed to participate in generic policy
/// checks such as descriptor selection, allow/deny lists, and lease binding.
fn identity_names() -> [&'static str; 11] {
    [
        "application_name",
        "app_name",
        "workflow_name",
        "provider_name",
        "model_name",
        "driver_name",
        "gateway_name",
        "chain_name",
        "payment_name",
        "business_domain",
        "business_domain_id",
    ]
}

/// Literal product identifiers that must never become generic branch keys.
///
/// These strings are negative examples for the proposal requirement. Keeping
/// them inside the governance test prevents production OS layers from learning
/// about proving applications while still making the intended freeze explicit.
fn forbidden_product_literals() -> [&'static str; 5] {
    [
        "codex",
        "codex-wasm-workbench",
        "workbench",
        "macaca-workbench",
        "interactive-agent-workbench",
    ]
}

/// Scan one Rust source file for forbidden product-identity branch behavior.
///
/// The implementation keeps a small amount of state to skip inline
/// `#[cfg(test)] mod tests` blocks inside production files. This avoids making
/// production modules fail the gate for local test-only negative examples.
fn scan_scope_branches(root: &Path, path: &Path) -> Vec<ScopeBranchViolation> {
    let relative = path
        .strip_prefix(root)
        .expect("scanned file should be under workspace root")
        .to_string_lossy()
        .replace('\\', "/");
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut violations = Vec::new();
    let mut pending_cfg_test = false;
    let mut test_module_depth: Option<i32> = None;

    for (index, line) in content.lines().enumerate() {
        if let Some(depth) = test_module_depth {
            let next_depth = depth + brace_delta(line);
            if next_depth <= 0 {
                test_module_depth = None;
            } else {
                test_module_depth = Some(next_depth);
            }
            continue;
        }

        if is_comment_only_line(line) {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }
        if pending_cfg_test {
            if trimmed.starts_with("mod tests") || trimmed.starts_with("pub mod tests") {
                let depth = brace_delta(line);
                if depth > 0 {
                    test_module_depth = Some(depth);
                }
                pending_cfg_test = false;
                continue;
            }
            if !trimmed.starts_with("#[") && !trimmed.is_empty() {
                pending_cfg_test = false;
            }
        }

        let Some(pattern) = branch_pattern(line) else {
            continue;
        };

        for identity in identity_names() {
            if line.contains(identity)
                && is_identity_branch(line, identity, pattern)
                && !is_approved_scope_owner(&relative, identity)
            {
                violations.push(ScopeBranchViolation {
                    path: PathBuf::from(&relative),
                    line: index + 1,
                    identity: identity.to_string(),
                    branch_pattern: pattern,
                });
            }
        }

        if contains_forbidden_product_branch(line) && !is_approved_product_literal_owner(&relative)
        {
            violations.push(ScopeBranchViolation {
                path: PathBuf::from(&relative),
                line: index + 1,
                identity: "forbidden_product_literal".to_string(),
                branch_pattern: pattern,
            });
        }
    }

    violations
}

/// Compute brace depth changes for coarse `#[cfg(test)] mod tests` skipping.
fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

/// Return true when a recognized identity participates in a branch expression.
///
/// Non-`match` branch patterns are already high-signal once the identity name is
/// present on the same line. For `match`, the scanner is narrower so it does not
/// reject unrelated match expressions that mention an identity in a payload.
fn is_identity_branch(line: &str, identity: &str, pattern: &str) -> bool {
    if pattern != "match " {
        return true;
    }
    let trimmed = line.trim_start();
    trimmed.starts_with(&format!("match {identity}"))
        || trimmed.contains(&format!("match {identity}.as_str()"))
}

/// Return true when a line branches on a known product literal.
fn contains_forbidden_product_branch(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    forbidden_product_literals()
        .into_iter()
        .any(|literal| normalized.contains(literal))
}

/// Product literals are allowed only in application/foundation metadata owners.
///
/// The current proposal explicitly uses a proving application while preserving
/// generic OS boundaries. Foundation and application crates may carry those
/// identifiers as package or protocol metadata; service/runtime/kernel code may
/// not route behavior by them.
fn is_approved_product_literal_owner(relative: &str) -> bool {
    relative.starts_with("crates/application/") || relative.starts_with("crates/foundation/")
}

/// Render violation evidence with a stable rationale.
fn render_violations(violations: &[ScopeBranchViolation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nfile={}:{}\nidentity={}\nbranch_pattern={}\nrationale=Application execution must use manifest capabilities, provider descriptors, ServiceRuntime policy, and trace/audit evidence; generic layers must not branch on product identity literals.\n",
                violation.path.display(),
                violation.line,
                violation.identity,
                violation.branch_pattern
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Assert that production service/runtime code remains product-neutral.
///
/// The log lines are intentionally concise and sanitized. They provide a trace
/// for the governance gate itself: scan root, file count, and violation count.
/// Detailed per-violation evidence is emitted only on assertion failure.
pub fn assert_application_execution_scope_control() {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();

    eprintln!(
        "application_execution_scope_control event=scan_start root={}",
        crates_root.display()
    );
    collect_production_rust_files(&crates_root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_scope_branches(&root, file));
    }
    violations.sort();
    eprintln!(
        "application_execution_scope_control event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "Application execution scope-control violations were found:{}",
        render_violations(&violations)
    );
}
