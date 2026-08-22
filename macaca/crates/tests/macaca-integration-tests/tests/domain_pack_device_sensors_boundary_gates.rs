//! Sensor provider ownership and direct-call boundary gates.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const FORBIDDEN: &[&str] = &[
    "DeviceSensorsSystemServiceProvider",
    "device_sensors_service_provider",
    "host_sensor_adapter",
    "raw_sample_vector",
];

#[test]
fn sensor_providers_stay_outside_provider_neutral_layers() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for surface in SURFACES {
        for source in rust_files(&root.join(surface)) {
            for (line_number, line) in read_source(&source).lines().enumerate() {
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
        "sensor boundary violations:\n{}",
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
