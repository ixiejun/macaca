//! Static ownership guard for application execution presentation surfaces.
//!
//! This module is an executable boundary Specification. Shell and frontend code
//! is allowed to adapt transport, parse requests, call SDK/SystemFacade clients,
//! render current state, and subscribe to replayable event streams. It is not
//! allowed to become the owner of provider lifecycle, authoritative EventLog
//! persistence, current-state projection, or approval/cancel state machines.
//! Keeping those predicates in code gives the proposal a repeatable audit gate
//! without adding application-specific production behavior.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// One ownership violation found in a shell or frontend file.
///
/// The diagnostic stores only path, line, and a stable rule id. It intentionally
/// does not print the source line because shell files can contain operator input
/// examples or endpoint payloads. Bounded evidence is enough for traceability.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ShellOwnershipViolation {
    path: PathBuf,
    line: usize,
    rule_id: &'static str,
    rationale: &'static str,
}

/// A forbidden semantic-owner pattern.
///
/// The gate uses the Specification pattern: each rule has a predicate that
/// detects one class of ownership leakage. The rules stay intentionally narrow
/// so presentation adapters can still carry DTO names and call focused clients
/// without failing the architecture gate.
struct ForbiddenPattern {
    rule_id: &'static str,
    rationale: &'static str,
    matches: fn(&str) -> bool,
}

/// Locate the repository root from the Macaca workspace root.
///
/// `CARGO_MANIFEST_DIR` points at the integration-test crate below
/// `macaca/crates/tests`. The frontend is a sibling of `macaca/`, so the
/// scanner first finds the Cargo workspace and then walks one parent up to the
/// repository root.
fn repo_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor
                    .parent()
                    .expect("macaca workspace should have repository parent")
                    .to_path_buf();
            }
        }
    }
    panic!("failed to locate repository root from CARGO_MANIFEST_DIR")
}

/// Return shell/frontend roots that currently exist in the checkout.
///
/// Missing roots are skipped so the gate remains usable in partial worktrees
/// while still scanning all presentation surfaces present in this repository.
fn presentation_roots(root: &Path) -> Vec<PathBuf> {
    [
        root.join("macaca/crates/shells/macaca-web/src"),
        root.join("macaca/crates/shells/macaca-cli/src"),
        root.join("frontend"),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect()
}

/// Collect Rust and TypeScript presentation files.
///
/// Generated/build artifacts are excluded because they can contain bundled code
/// or transformed strings that no longer represent authored ownership.
fn collect_presentation_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if matches!(
            name,
            "target" | "node_modules" | ".next" | "dist" | "coverage" | ".git"
        ) {
            continue;
        }
        if path.is_dir() {
            collect_presentation_files(&path, files);
            continue;
        }
        if matches!(
            path.extension().and_then(OsStr::to_str),
            Some("rs" | "ts" | "tsx")
        ) {
            files.push(path);
        }
    }
}

/// Skip comments and prose-only lines.
///
/// The proposal and the shell source legitimately document the boundary. The
/// executable guard should reject behavior, not explanatory text.
fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("*")
        || trimmed.starts_with("//!")
        || trimmed.starts_with("///")
}

/// Rules that indicate shell/frontend ownership of application execution.
fn forbidden_patterns() -> Vec<ForbiddenPattern> {
    vec![
        ForbiddenPattern {
            rule_id: "shell-no-application-execution-provider-impl",
            rationale: "Presentation code must not implement application execution providers; provider strategies belong to runtime-host/service modules.",
            matches: |line| {
                line.contains("impl ApplicationExecutionProvider")
                    || line.contains("struct ApplicationExecutionProvider")
                    || line.contains("class ApplicationExecutionProvider")
            },
        },
        ForbiddenPattern {
            rule_id: "shell-no-application-execution-event-store",
            rationale: "Presentation code must not construct the authoritative application execution EventStore; durable append/replay belongs to the service boundary.",
            matches: |line| {
                line.contains("ApplicationExecutionEventStore::new")
                    || line.contains("new ApplicationExecutionEventStore")
            },
        },
        ForbiddenPattern {
            rule_id: "shell-no-application-execution-projection",
            rationale: "Presentation code must not own current-state projection reducers; it should query service-owned projections or replay results.",
            matches: |line| {
                line.contains("project_application_execution_state")
                    || line.contains("reduceApplicationExecutionState")
                    || line.contains("applicationExecutionReducer")
            },
        },
        ForbiddenPattern {
            rule_id: "shell-no-application-execution-control-state-machine",
            rationale: "Presentation code must not implement approval/cancel/pause/resume state machines; it should send typed control commands and render outcomes.",
            matches: |line| {
                line.contains("ApplicationExecutionLifecycleState::WaitingForApproval")
                    || line.contains("ApplicationExecutionLifecycleState::Cancelling")
                    || line.contains("ApplicationExecutionLifecycleState::Paused")
                    || line.contains("ApplicationExecutionLifecycleState::Resuming")
            },
        },
        ForbiddenPattern {
            rule_id: "frontend-no-application-execution-authority",
            rationale: "Frontend code must not own application execution provider/event/control authority; browser state is only a render cache.",
            matches: |line| {
                line.contains("ApplicationExecutionProviderRegistry")
                    || line.contains("ApplicationExecutionProviderLease")
                    || line.contains("ApplicationExecutionEventStore")
            },
        },
    ]
}

/// Scan one file using the ownership predicates.
fn scan_file(
    root: &Path,
    path: &Path,
    patterns: &[ForbiddenPattern],
) -> Vec<ShellOwnershipViolation> {
    let relative = path
        .strip_prefix(root)
        .expect("scanned file should be below repository root")
        .to_string_lossy()
        .replace('\\', "/");
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut violations = Vec::new();

    for (index, line) in content.lines().enumerate() {
        if is_comment_only_line(line) {
            continue;
        }
        for pattern in patterns {
            if (pattern.matches)(line) {
                violations.push(ShellOwnershipViolation {
                    path: PathBuf::from(&relative),
                    line: index + 1,
                    rule_id: pattern.rule_id,
                    rationale: pattern.rationale,
                });
            }
        }
    }

    violations
}

/// Render deterministic, bounded violation evidence.
fn render_violations(violations: &[ShellOwnershipViolation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nfile={}:{}\nrule={}\nrationale={}\nreplacement=Use SDK/SystemFacade application execution clients, persisted replay/current-state queries, and typed control commands; keep provider/event/projection authority in service/runtime-host code.\n",
                violation.path.display(),
                violation.line,
                violation.rule_id,
                violation.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Assert that shell/frontend surfaces remain adapters for application execution.
///
/// The scan logs are deliberately concise: root count, file count, and violation
/// count give enough audit evidence for CI while avoiding raw payload output.
pub fn assert_shell_surfaces_do_not_own_application_execution_semantics() {
    let root = repo_root();
    let roots = presentation_roots(&root);
    let mut files = Vec::new();
    let patterns = forbidden_patterns();

    eprintln!(
        "application_execution_shell_ownership event=scan_start roots={}",
        roots.len()
    );
    for presentation_root in &roots {
        collect_presentation_files(presentation_root, &mut files);
    }
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_file(&root, file, &patterns));
    }
    violations.sort();
    eprintln!(
        "application_execution_shell_ownership event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "Application execution shell ownership violations were found:{}",
        render_violations(&violations)
    );
}
