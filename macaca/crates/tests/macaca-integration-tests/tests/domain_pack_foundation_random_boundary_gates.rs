//! Foundation-random provider boundary gate.
//!
//! Provider-neutral layers may use `ServiceCall` and random DTOs, but host CSPRNG
//! and deterministic provider Strategies belong to service/runtime composition.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const FORBIDDEN: &[&str] = &[
    "HostRandomProvider",
    "DeterministicRandomProvider",
    "UnavailableRandomProvider",
    "getrandom::getrandom",
];

/// Protect provider composition from kernel, SDK, shell, and app-framework imports.
#[test]
fn foundation_random_boundaries_do_not_construct_rng_providers() {
    let root = root();
    let mut violations = Vec::new();
    for surface in SURFACES {
        let mut sources = Vec::new();
        files(&root.join(surface), &mut sources);
        sources.sort();
        for source in sources {
            for (line_number, line) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for token in FORBIDDEN {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{}:{token}",
                            source.strip_prefix(&root).unwrap().display(),
                            line_number + 1
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "foundation random boundary violations:\n{}",
        violations.join("\n")
    );
}

/// SDK helpers and production WASM host imports must route through the generic
/// service boundary. This source gate rejects native entropy access and concrete
/// provider construction outside the runtime composition layer.
#[test]
fn foundation_random_sdk_and_wasm_imports_cannot_bypass_service_runtime() {
    let root = root();
    let surfaces = [
        root.join("crates/facade/macaca-sdk/src/foundation_random_client.rs"),
        root.join(
            "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge",
        ),
    ];
    let forbidden = [
        "HostRandomProvider",
        "DeterministicRandomProvider",
        "getrandom::",
    ];
    let mut violations = Vec::new();
    for surface in surfaces {
        let mut sources = Vec::new();
        if surface.is_dir() {
            files(&surface, &mut sources);
        } else {
            sources.push(surface);
        }
        for source in sources {
            for (line_number, line) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if !line.trim_start().starts_with("//") {
                    for token in &forbidden {
                        if line.contains(token) {
                            violations.push(format!(
                                "{}:{}:{token}",
                                source.strip_prefix(&root).unwrap().display(),
                                line_number + 1
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "random direct-provider bypasses:\n{}",
        violations.join("\n")
    );
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|path| {
            fs::read_to_string(path.join("Cargo.toml"))
                .ok()
                .filter(|text| text.contains("[workspace]"))
                .map(|_| path.to_path_buf())
        })
        .expect("workspace root")
}

fn files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, output);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            output.push(path);
        }
    }
}
