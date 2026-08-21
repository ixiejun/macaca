//! Device-camera provider ownership and direct-call boundary gates.

use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_NEUTRAL: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const PROVIDER_TOKENS: &[&str] = &[
    "DeviceCameraSystemServiceProvider",
    "device_camera_service_provider",
    "host_camera_adapter",
];
const DIRECT_CALL_SURFACES: &[&str] = &[
    "crates/facade/macaca-sdk/src",
    "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge",
];
const DIRECT_CALL_TOKENS: &[&str] = &[
    "DeviceCameraSystemServiceProvider",
    "device_camera_service_provider",
    "raw_frame",
    "media_bytes",
];

#[test]
fn camera_provider_strategies_stay_outside_provider_neutral_layers() {
    assert_no_tokens(
        PROVIDER_NEUTRAL,
        PROVIDER_TOKENS,
        "camera provider boundary violations",
    );
}

#[test]
fn camera_sdk_and_wasm_imports_cannot_bypass_service_runtime() {
    assert_no_tokens(
        DIRECT_CALL_SURFACES,
        DIRECT_CALL_TOKENS,
        "camera direct-provider bypasses",
    );
}

fn assert_no_tokens(surfaces: &[&str], forbidden: &[&str], failure_message: &str) {
    let root = workspace_root();
    let mut violations = Vec::new();
    for surface in surfaces {
        for source in rust_files(&root.join(surface)) {
            for (line_number, line) in read_source(&source).lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for token in forbidden {
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
        "{failure_message}:\n{}",
        violations.join("\n")
    );
}

fn workspace_root() -> PathBuf {
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

fn rust_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(path, &mut files);
    files.sort();
    files
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("boundary surface should be readable") {
        let path = entry.expect("directory entry should be readable").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn read_source(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}
