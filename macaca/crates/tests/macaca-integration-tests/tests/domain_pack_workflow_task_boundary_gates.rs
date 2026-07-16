//! Workflow-task provider boundary gates.
//!
//! Workflow task contracts are visible to every OS layer, but concrete task
//! providers belong exclusively to runtime-host or optional package composition
//! roots. This source-level gate prevents a future kernel, SDK, shell, or
//! generic application-framework change from bypassing that service boundary.

use std::fs;
use std::path::{Path, PathBuf};

/// Production source surface that may consume only provider-neutral contracts.
struct BoundarySurface {
    name: &'static str,
    relative_root: &'static str,
}

/// Concrete runtime-host task-provider symbols forbidden outside composition roots.
const FORBIDDEN_PROVIDER_TOKENS: &[&str] = &[
    "WorkflowTaskLifecycleSystemServiceProvider",
    "SharedWorkflowTaskLifecycleProvider",
    "workflow_task_service_provider",
];

/// Layers that must not construct a workflow task provider or workflow engine.
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
        if fs::read_to_string(&cargo_toml).is_ok_and(|content| content.contains("[workspace]")) {
            return ancestor.to_path_buf();
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("directory entry should be readable").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
}

#[test]
fn workflow_task_boundaries_do_not_import_or_construct_concrete_providers() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for surface in BOUNDARY_SURFACES {
        let mut files = Vec::new();
        collect_rust_files(&root.join(surface.relative_root), &mut files);
        files.sort();
        for file in files {
            let relative = file
                .strip_prefix(&root)
                .expect("boundary source should be under workspace root")
                .display();
            let content = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
            for (line_number, line) in content.lines().enumerate() {
                if is_comment_only_line(line) {
                    continue;
                }
                for token in FORBIDDEN_PROVIDER_TOKENS {
                    if line.contains(token) {
                        violations.push(format!(
                            "{relative}:{}: surface={} token={token}",
                            line_number + 1,
                            surface.name
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "workflow-task providers must remain behind runtime-host composition roots:\n{}",
        violations.join("\n")
    );
}
