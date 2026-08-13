//! Workflow-review domain-pack boundary gates.
//!
//! Kernel, SDK, shells, and the generic application framework may inspect
//! provider-neutral descriptors but must not construct a review provider.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_PROVIDER_TOKENS: &[&str] = &[
    "ReviewSystemServiceProvider::local",
    "ReviewSystemServiceProvider::mock",
    "ReviewSystemServiceProvider::unavailable",
    "LocalReviewProvider",
    "bootstrap_local_review_service",
];
const BOUNDARY_SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];

/// Scan provider-neutral production roots for prohibited provider construction.
#[test]
fn workflow_review_boundaries_do_not_construct_concrete_providers() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for surface in BOUNDARY_SURFACES {
        for file in rust_files(&root.join(surface)) {
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
            for (line_index, line) in content.lines().enumerate() {
                if is_comment(line) {
                    continue;
                }
                for token in FORBIDDEN_PROVIDER_TOKENS {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{} token={token}",
                            file.strip_prefix(&root).unwrap_or(&file).display(),
                            line_index + 1,
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "review provider boundary violation:\n{}",
        violations.join("\n")
    );
}

/// Locate the Rust workspace without relying on machine-specific paths.
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

/// Recursively collect Rust sources in deterministic order for reproducible gates.
fn rust_files(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    collect_rust_files(path, &mut result);
    result.sort();
    result
}

/// Add Rust source descendants while retaining only files relevant to the gate.
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

/// Avoid failing a boundary gate on explanatory documentation comments.
fn is_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}
