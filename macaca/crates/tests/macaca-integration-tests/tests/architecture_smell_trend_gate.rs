//! Non-failing architecture-smell trend diagnostics.
//!
//! This test is intentionally advisory: it emits deterministic, sanitized
//! rule-id records for CI logs without blocking local development. Hard
//! boundary gates remain in the dedicated serviceization, shell ownership, and
//! file-size tests.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const RULE_FILE_HEADROOM: &str = "ARCH-SMELL-FILE-HEADROOM";
const RULE_STATIC_STATE: &str = "ARCH-SMELL-STATIC-STATE";
const RULE_PROVIDER_NEAR_LIMIT: &str = "ARCH-SMELL-PROVIDER-NEAR-LIMIT";

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SmellDiagnostic {
    rule_id: &'static str,
    path: String,
    metric: String,
}

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let candidate = ancestor.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            if text.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate workspace root")
}

fn collect_rust_sources(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry should be readable").path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if matches!(name, "target" | ".git" | "fixtures") {
            continue;
        }
        if path.is_dir() {
            collect_rust_sources(&path, files);
        } else if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn architecture_smell_trend_report_is_deterministic_and_non_failing() {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_sources(&crates_root, &mut files);
    files.sort();

    let mut diagnostics = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("source should live under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        let line_count = content.lines().count();
        if line_count >= 450 {
            diagnostics.push(SmellDiagnostic {
                rule_id: RULE_FILE_HEADROOM,
                path: relative.clone(),
                metric: format!("lines={line_count} hard_limit=500"),
            });
        }
        if relative.contains("runtime-host/src/") && relative.ends_with("_service_provider.rs") {
            if line_count >= 425 {
                diagnostics.push(SmellDiagnostic {
                    rule_id: RULE_PROVIDER_NEAR_LIMIT,
                    path: relative.clone(),
                    metric: format!("lines={line_count} advisory_limit=450"),
                });
            }
        }
        let static_count =
            content.matches("static ").count() + content.matches("OnceLock<").count();
        if static_count > 0 {
            diagnostics.push(SmellDiagnostic {
                rule_id: RULE_STATIC_STATE,
                path: relative,
                metric: format!("static_or_once_lock_tokens={static_count}"),
            });
        }
    }
    diagnostics.sort();

    eprintln!(
        "architecture_smell_trend event=report_start diagnostic_count={} non_failing=true",
        diagnostics.len()
    );
    for diagnostic in diagnostics {
        eprintln!(
            "architecture_smell_trend rule_id={} path={} metric={}",
            diagnostic.rule_id, diagnostic.path, diagnostic.metric
        );
    }
    eprintln!("architecture_smell_trend event=report_complete");
}
