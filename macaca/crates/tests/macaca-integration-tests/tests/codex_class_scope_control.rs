//! Codex-class workbench scope-control gate.
//!
//! The Codex-class application support proposal intentionally builds generic
//! Interactive Agent Workbench services. This executable governance test keeps
//! that scope honest: production Rust below the application layer must not add
//! branches that special-case application names, workflow names, provider
//! names, model names, gateway names, chain names, payment names, or business
//! domains. Those identities may appear as DTO fields, descriptors, telemetry
//! labels, or application-framework data, but lower layers must route through
//! service contracts and policy rather than product-specific `if` statements.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// A source branch that compares a routing identity to a literal string.
/// Keeping the evidence structured makes CI failures actionable and auditable.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ScopeBranchViolation {
    path: PathBuf,
    line: usize,
    identity: &'static str,
    branch_pattern: &'static str,
}

/// Locate the Rust workspace without assuming how many path parents the test
/// binary has. This keeps the gate stable when the integration-test crate moves.
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

/// Recursively collect production Rust files. Test, fixture, and example code
/// is intentionally excluded because those files often contain literal product
/// names as negative examples for boundary checks.
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
        if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

/// Application-framework code is the allowed owner for application metadata and
/// workflow identifiers. The LLM service is a current migration surface for
/// provider/model routing strategy; future hardening work moves more of that
/// behavior into descriptor data, but this phase records it as non-blocking
/// legacy ownership rather than failing the Codex-class workbench gate.
fn is_approved_scope_owner(relative: &str, identity: &str) -> bool {
    relative.starts_with("crates/application/")
        || relative.starts_with("crates/foundation/")
        || (matches!(identity, "provider_name" | "model_name")
            && relative.starts_with("crates/services/macaca-llm/src/"))
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
}

/// Detect literal identity branches without trying to parse Rust. The gate is a
/// conservative freeze: it targets obvious product-specific branch shapes while
/// leaving DTO declarations, descriptor strings, and ordinary telemetry alone.
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

fn identity_names() -> [&'static str; 10] {
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
    ]
}

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
                    identity,
                    branch_pattern: pattern,
                });
            }
        }
    }

    violations
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn is_identity_branch(line: &str, identity: &str, pattern: &str) -> bool {
    if pattern != "match " {
        return true;
    }
    let trimmed = line.trim_start();
    trimmed.starts_with(&format!("match {identity}"))
        || trimmed.contains(&format!("match {identity}.as_str()"))
}

fn render_violations(violations: &[ScopeBranchViolation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nfile={}:{}\nidentity={}\nbranch_pattern={}\nrationale=Codex-class workbench support must use manifest capabilities, service descriptors, and policy; lower layers must not branch on product identity literals.\n",
                violation.path.display(),
                violation.line,
                violation.identity,
                violation.branch_pattern
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn codex_class_workbench_scope_control_rejects_product_identity_branches() {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();

    eprintln!(
        "codex_class_scope_control event=scan_start root={}",
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
        "codex_class_scope_control event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "Codex-class workbench scope-control violations were found:{}",
        render_violations(&violations)
    );
}
