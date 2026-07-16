//! AI domain-pack boundary gates.
//!
//! AI packs are serviceized optional capabilities. Kernel, SDK, presentation
//! shells, and the generic application framework may inspect provider-neutral
//! descriptors, declarations, and diagnostics, but they must not import or
//! construct concrete provider replacement adapters. Those adapters belong in
//! runtime-host composition roots or optional package crates.

use std::fs;
use std::path::{Path, PathBuf};

/// A production source surface that must remain provider-neutral.
struct BoundarySurface {
    name: &'static str,
    relative_root: &'static str,
}

/// Concrete provider/replacement construction tokens that are not allowed in
/// kernel, SDK, shell, or generic application-framework production sources.
const FORBIDDEN_PROVIDER_TOKENS: &[&str] = &[
    "DomainPackUnavailableSystemServiceProvider",
    "DomainPackMockSystemServiceProvider",
    "unavailable_domain_pack_provider_registration",
    "mock_domain_pack_provider_registration",
    "domain_pack_provider_replacement",
];

/// Provider-neutral surfaces may mention AI descriptors, but concrete provider
/// ownership must stay outside these roots.
const BOUNDARY_SURFACES: &[BoundarySurface] = &[
    BoundarySurface {
        name: "kernel",
        relative_root: "crates/kernel",
    },
    BoundarySurface {
        name: "sdk",
        relative_root: "crates/facade/macaca-sdk/src",
    },
    BoundarySurface {
        name: "shells",
        relative_root: "crates/shells",
    },
    BoundarySurface {
        name: "application-framework",
        relative_root: "crates/application/macaca-app/src",
    },
];

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

fn source_files_for_surface(root: &Path, surface: &BoundarySurface) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(&root.join(surface.relative_root), &mut files);
    files.sort();
    files
}

#[test]
fn ai_domain_pack_boundaries_do_not_import_or_construct_concrete_providers() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for surface in BOUNDARY_SURFACES {
        for file in source_files_for_surface(&root, surface) {
            let relative = file
                .strip_prefix(&root)
                .expect("boundary source should be under workspace root")
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));

            for (line_index, line) in content.lines().enumerate() {
                if is_comment_only_line(line) {
                    continue;
                }
                for token in FORBIDDEN_PROVIDER_TOKENS {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{}: surface={} token={}",
                            relative,
                            line_index + 1,
                            surface.name,
                            token
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "AI domain-pack concrete providers must stay behind runtime-host/package composition roots:\n{}",
        violations.join("\n")
    );
}
