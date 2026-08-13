//! Workflow-schedule provider boundary gates.
//!
//! Schedule descriptors and DTOs are available to every OS layer, while concrete
//! recurrence engines and providers remain behind runtime-host/service composition.
//! This gate protects the Adapter/Bridge boundary from accidental shell, SDK,
//! kernel, or application-framework provider construction.

use std::fs;
use std::path::{Path, PathBuf};

/// Concrete scheduler implementation tokens forbidden outside service composition roots.
const FORBIDDEN_PROVIDER_TOKENS: &[&str] = &[
    "InProcessSchedulerProvider",
    "UnavailableSchedulerProvider",
    "DefaultScheduleCalculator",
    "bootstrap_local_scheduler_service",
];

/// Provider-neutral layers permitted to use only the typed schedule contracts.
const BOUNDARY_SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];

/// Ensure generic OS layers do not import or construct a scheduling provider.
#[test]
fn workflow_schedule_boundaries_do_not_construct_concrete_providers() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for surface in BOUNDARY_SURFACES {
        for file in rust_files(&root.join(surface)) {
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
            for (line_number, line) in content.lines().enumerate() {
                if is_comment(line) {
                    continue;
                }
                for token in FORBIDDEN_PROVIDER_TOKENS {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{} token={token}",
                            file.strip_prefix(&root).unwrap_or(&file).display(),
                            line_number + 1,
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "workflow schedule provider boundary violation:\n{}",
        violations.join("\n")
    );
}

/// Locate the workspace dynamically so the gate is portable across hosts.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| {
            fs::read_to_string(path.join("Cargo.toml"))
                .is_ok_and(|manifest| manifest.contains("[workspace]"))
        })
        .expect("workspace root must be discoverable")
        .to_path_buf()
}

/// Collect sources deterministically, keeping failures reproducible in CI.
fn rust_files(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_rust_files(path, &mut result);
    result.sort();
    result
}

/// Traverse only Rust files from a known source surface.
fn collect_rust_files(path: &Path, result: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
    {
        let path = entry.expect("directory entry must be readable").path();
        if path.is_dir() {
            collect_rust_files(&path, result);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            result.push(path);
        }
    }
}

/// Ignore explanatory comments because they do not create a dependency edge.
fn is_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}
