//! Deterministic scanner for serviceization escape-hatch policy tests.
//!
//! The scanner is deliberately token-based because these tests are architectural
//! tripwires, not a Rust parser. Token families are declared in `tokens.rs`, and
//! this module owns only traversal, production/test filtering, and stable output.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::support::workspace_root;
use super::tokens::{forbidden_tokens, ForbiddenToken};

/// Controls whether approved test surfaces suppress token hits during scan.
#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    /// When true, hits inside `is_approved_terminal_exception_surface` are
    /// ignored by the gate. When false, every production hit is recorded for the
    /// raw debt inventory baseline.
    pub honor_terminal_exception_surfaces: bool,
}

/// A deterministic violation rendered in sorted order for stable CI output.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Violation {
    pub family: &'static str,
    pub path: PathBuf,
    pub line: usize,
    pub token: &'static str,
    pub rationale: &'static str,
}

pub fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
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

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "tests" | "fixtures" | "examples" | ".git" | ".playwright-mcp"
    )
}

/// Returns true when a Rust path is test/fixture code even if it lives under
/// `src/`.
pub fn is_non_production_rust_source(relative: &str) -> bool {
    relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
        || relative.ends_with("/serviceization_escape_hatches.rs")
}

fn is_provider_model_routing_canonical_owner(relative: &str) -> bool {
    relative.starts_with("crates/services/macaca-llm/src/")
}

fn is_approved_terminal_exception_surface(relative: &str, token: &ForbiddenToken) -> bool {
    if relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
    {
        return true;
    }

    match token.family {
        "hardcoded-agent-role" => matches!(
            relative,
            "crates/application/macaca-app/src/consumption.rs"
                | "crates/application/macaca-app/src/service_projection.rs"
                | "crates/foundation/macaca-proto/src/orchestration.rs"
                | "crates/foundation/macaca-proto/src/types/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/app_executor/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/bus.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/callback.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/event_factory.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/fork_manager/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/mod.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/queue.rs"
                | "crates/runtime/macaca-runtime-host/src/executor/router.rs"
                | "crates/kernel/macaca-kernel/src/orchestrator.rs"
                | "crates/runtime/macaca-framework/src/construction.rs"
                | "crates/runtime/macaca-runtime-host/src/agent_context_service_provider.rs"
                | "crates/runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
                | "crates/services/macaca-memory/src/core/tests.rs"
                | "crates/services/macaca-task/src/claim_diagnostics.rs"
                | "crates/services/macaca-task/src/decompose.rs"
                | "crates/services/macaca-task/src/dependency.rs"
                | "crates/services/macaca-task/src/lifecycle.rs"
                | "crates/services/macaca-task/src/plan_loop/mod.rs"
                | "crates/services/macaca-task/src/scheduler.rs"
                | "crates/services/macaca-task/src/todo_board/tests.rs"
                | "crates/services/macaca-task/src/todo_store.rs"
                | "crates/shells/macaca-web/src/capability_catalog.rs"
                | "crates/shells/macaca-web/src/framework_toolkit/mod.rs"
                | "crates/shells/macaca-web/src/framework_toolkit/builder.rs"
                | "crates/shells/macaca-web/src/loop_manager/mod.rs"
                | "crates/shells/macaca-web/src/orchestration_tools.rs"
                | "crates/shells/macaca-web/src/session/mod.rs"
                | "crates/shells/macaca-web/src/workspace.rs"
                | "crates/shells/macaca-web/src/workspace_knowledge_digest_capability.rs"
        ),
        "kernel-non-kernel-module" => relative.starts_with("crates/kernel/macaca-kernel/src/"),
        _ => false,
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*")
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

pub fn violation_fingerprint(violation: &Violation) -> String {
    format!(
        "{}|{}|{}",
        violation.family,
        violation.path.display(),
        violation.token
    )
}

/// Scan all production Rust files under `crates/` and return sorted violations.
pub fn collect_production_violations(options: ScanOptions) -> Vec<Violation> {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let tokens = forbidden_tokens();
    let mut files = Vec::new();

    eprintln!(
        "serviceization_escape_hatches event=scan_start honor_terminal_exception_surfaces={} root={}",
        options.honor_terminal_exception_surfaces,
        crates_root.display()
    );
    collect_rust_files(&crates_root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_file(
            &root,
            file,
            &tokens,
            options.honor_terminal_exception_surfaces,
        ));
    }
    violations.sort();
    eprintln!(
        "serviceization_escape_hatches event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );
    violations
}

pub fn scan_file(
    root: &Path,
    path: &Path,
    tokens: &[ForbiddenToken],
    honor_terminal_exception_surfaces: bool,
) -> Vec<Violation> {
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

        for token in tokens {
            if !line.contains(token.token)
                || ((token.family == "hardcoded-agent-role"
                    || token.family == "provider-model-routing-name")
                    && is_comment_only_line(line))
                || (token.family == "provider-model-routing-name"
                    && is_provider_model_routing_canonical_owner(&relative))
                || (honor_terminal_exception_surfaces
                    && is_approved_terminal_exception_surface(&relative, token))
            {
                continue;
            }
            violations.push(Violation {
                family: token.family,
                path: PathBuf::from(&relative),
                line: index + 1,
                token: token.token,
                rationale: token.rationale,
            });
        }
    }

    violations
}

pub fn render_violations(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nfamily={}\nfile={}:{}\ntoken={}\nrationale={}\nprocess=Move the caller behind a service client/facade, or update the terminal specification with a justified exception.\n",
                violation.family,
                violation.path.display(),
                violation.line,
                violation.token,
                violation.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("")
}
