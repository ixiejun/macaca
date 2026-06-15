//! SDK provider re-export boundary gate.
//!
//! The SDK may expose focused clients, provider-neutral DTOs, and explicit
//! composition contracts. It must not re-export whole provider/runtime crates,
//! because broad aliases hide ownership debt and let shells bypass service
//! clients without review.

use std::path::{Path, PathBuf};

/// One forbidden SDK source pattern with the architectural reason it is denied.
struct ForbiddenSdkReexport {
    token: &'static str,
    rationale: &'static str,
}

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

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn forbidden_reexports() -> Vec<ForbiddenSdkReexport> {
    vec![
        ForbiddenSdkReexport {
            token: "pub mod runtime_host",
            rationale: "SDK must not expose runtime-host as a public module",
        },
        ForbiddenSdkReexport {
            token: "focused_runtime_surfaces",
            rationale: "SDK must not preserve runtime-host surfaces through narrowed module names",
        },
        ForbiddenSdkReexport {
            token: "runtime-host-bootstrap",
            rationale: "SDK features must not hide runtime-host production dependencies",
        },
        ForbiddenSdkReexport {
            token: "macaca-runtime-host",
            rationale: "SDK Cargo metadata must not depend on runtime-host",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_runtime_host::*",
            rationale: "SDK runtime-host exposure must be explicit and reviewable",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_framework::*",
            rationale: "SDK framework exposure must use narrow provider-neutral modules",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_app::*",
            rationale: "SDK application exposure must use Application ABI clients or explicit DTOs",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_context::*",
            rationale: "SDK context exposure must use focused context clients or explicit DTOs",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_runtime_host as ",
            rationale: "SDK must not expose runtime-host through crate alias re-exports",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_framework as ",
            rationale: "SDK must not expose framework through crate alias re-exports",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_app as ",
            rationale: "SDK must not expose application internals through crate alias re-exports",
        },
        ForbiddenSdkReexport {
            token: "pub use macaca_context as ",
            rationale: "SDK must not expose context internals through crate alias re-exports",
        },
    ]
}

#[test]
fn sdk_does_not_expose_broad_provider_or_runtime_reexports() {
    let root = workspace_root();
    let sdk_src = root.join("crates/facade/macaca-sdk/src");
    let mut files = Vec::new();
    collect_rust_files(&sdk_src, &mut files);
    files.push(root.join("crates/facade/macaca-sdk/Cargo.toml"));
    files.sort();

    let forbidden = forbidden_reexports();
    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("SDK source should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in content.lines().enumerate() {
            for pattern in &forbidden {
                if line.contains(pattern.token) {
                    violations.push(format!(
                        "{}:{}: {} ({})",
                        relative,
                        index + 1,
                        pattern.token,
                        pattern.rationale
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "SDK provider/runtime broad re-export violations:\n{}",
        violations.join("\n")
    );
}
