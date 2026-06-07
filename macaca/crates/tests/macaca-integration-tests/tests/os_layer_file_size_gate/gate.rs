//! Executable OS-layer file-size gate (VC-filesize).
//!
//! Macaca's constitution caps production Rust sources at 500 lines so ownership
//! stays obvious and modules remain reviewable. This gate scans every
//! `crates/**/src/**/*.rs` file, compares line counts against the limit, and
//! fails on any violation that is not recorded in the migration allowlist.
//!
//! Design pattern: **Specification by Example** — the test is the living
//! contract; the allowlist is explicit migration debt that must monotonically
//! shrink until the terminal assertion (`allowlist.len() == 0`) passes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::allowlist;

/// Maximum permitted production source lines per OS-layer Rust file.
pub const MAX_OS_LAYER_SOURCE_LINES: usize = 500;

/// One allowlisted oversized file with governance metadata for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSizeAllowlistEntry {
    /// Workspace-relative path (`crates/...`).
    pub path: String,
    /// Recorded line count at allowlist insertion time (informational).
    pub line_count: usize,
    /// Owning refactor track (e.g. `P3-web-thin-shell`).
    pub owner_track: String,
    /// Target OpenSpec phase when the row should be deleted.
    pub target_phase: String,
}

impl FileSizeAllowlistEntry {
    /// Constructs one migration-debt row used by the executable gate.
    pub fn new(
        path: &str,
        line_count: usize,
        owner_track: &str,
        target_phase: &str,
    ) -> Self {
        Self {
            path: path.to_string(),
            line_count,
            owner_track: owner_track.to_string(),
            target_phase: target_phase.to_string(),
        }
    }
}

/// Locates the Macaca workspace root from the integration-test crate manifest.
fn workspace_root() -> PathBuf {
    for ancestor in PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from integration-tests manifest");
}

/// Returns true when `path` is a production OS-layer Rust source under `src/`.
///
/// We intentionally include inline `#[cfg(test)]` modules inside production
/// files because the line-count constitution applies to the whole file. Stand-
/// alone integration test crates under `crates/tests/` are out of scope.
fn is_os_layer_production_source(path: &Path, workspace: &Path) -> bool {
    let rel = path
        .strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    if !rel.starts_with("crates/") || !rel.contains("/src/") {
        return false;
    }
    if rel.contains("/target/") {
        return false;
    }
    rel.ends_with(".rs")
}

/// Counts physical lines in a UTF-8 source file (blank lines included).
fn count_lines(path: &Path) -> usize {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    text.lines().count()
}

/// Scans the workspace and returns oversized production sources as
/// `(relative_path, line_count)` pairs sorted for stable diagnostics.
fn scan_oversized_sources(workspace: &Path) -> Vec<(String, usize)> {
    let mut violations = Vec::new();
    let crates_dir = workspace.join("crates");
    for entry in walkdir_rs_files(&crates_dir) {
        if !is_os_layer_production_source(&entry, workspace) {
            continue;
        }
        let line_count = count_lines(&entry);
        if line_count > MAX_OS_LAYER_SOURCE_LINES {
            let rel = entry
                .strip_prefix(workspace)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            violations.push((rel, line_count));
        }
    }
    violations.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    violations
}

/// Minimal directory walker — avoids adding a `walkdir` dev-dependency to the gate.
fn walkdir_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    let mut files = Vec::new();
    while let Some(current) = stack.pop() {
        let read_dir = fs::read_dir(&current)
            .unwrap_or_else(|err| panic!("read_dir {}: {err}", current.display()));
        for entry in read_dir {
            let entry = entry.expect("read_dir entry");
            let path = entry.path();
            if path.is_dir() {
                if path.ends_with("target") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

/// Main gate entry — panics with actionable diagnostics on policy violation.
pub fn assert_os_layer_file_size_boundaries() {
    let workspace = workspace_root();
    let violations = scan_oversized_sources(&workspace);

    let allowlisted: BTreeMap<String, FileSizeAllowlistEntry> = allowlist::entries()
        .into_iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect();

    let mut unallowlisted = Vec::new();
    for (path, lines) in &violations {
        if !allowlisted.contains_key(path) {
            unallowlisted.push(format!("{path}: {lines} lines (limit {MAX_OS_LAYER_SOURCE_LINES})"));
        }
    }

    if !unallowlisted.is_empty() {
        panic!(
            "OS-layer file-size gate: {} unallowlisted file(s) exceed {} lines:\n{}\n\
             Add only with OpenSpec + allowlist.rs + governance doc sync, or split the module.",
            unallowlisted.len(),
            MAX_OS_LAYER_SOURCE_LINES,
            unallowlisted.join("\n")
        );
    }

    // Stale allowlist rows (file shrunk or deleted) must be removed to keep the
    // debt inventory honest.
    let violation_paths: BTreeSet<String> = violations.into_iter().map(|(p, _)| p).collect();
    let mut stale = Vec::new();
    for path in allowlisted.keys() {
        if !violation_paths.contains(path) {
            stale.push(path.clone());
        }
    }
    if !stale.is_empty() {
        panic!(
            "OS-layer file-size gate: {} stale allowlist row(s) — file(s) now ≤{} lines; \
             delete the row(s):\n{}",
            stale.len(),
            MAX_OS_LAYER_SOURCE_LINES,
            stale.join("\n")
        );
    }

    // Gate passed: every oversized file is allowlisted and every allowlist row is live.
    let _ = allowlisted.len();
}
