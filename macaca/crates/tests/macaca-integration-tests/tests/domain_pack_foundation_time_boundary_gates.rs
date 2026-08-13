//! Foundation-time provider boundary and observability gates.
//!
//! These static checks preserve the serviceization boundary: application-facing
//! layers may build traced `ServiceCall` commands, but only service/runtime
//! composition can import concrete host or deterministic clock providers.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const FORBIDDEN: &[&str] = &[
    "HostTimeProvider",
    "FrozenTimeProvider",
    "UnavailableTimeProvider",
];

/// Prevent SDK, WASM/application, shell, and kernel code from bypassing time services.
#[test]
fn foundation_time_boundaries_do_not_construct_concrete_clock_providers() {
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
        "foundation time boundary violations:\n{}",
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
