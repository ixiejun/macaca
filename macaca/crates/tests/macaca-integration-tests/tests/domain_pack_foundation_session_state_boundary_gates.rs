//! Foundation session-state provider ownership and redaction gate.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
    "crates/runtime/macaca-framework/src",
    "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider",
];
const FORBIDDEN: &[&str] = &[
    "FoundationSessionStateSystemServiceProvider",
    "EmbeddedFoundationSessionStateProvider",
    "foundation_session_state_service_provider",
    "foundation_session_state_embedded_provider",
    "raw_state",
];

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

fn rust_files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("surface directory") {
        let path = entry.expect("surface entry").path();
        if path.is_dir() {
            rust_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

#[test]
fn session_state_provider_stays_behind_service_boundary() {
    let root = root();
    let mut violations = Vec::new();
    for surface in SURFACES {
        let mut sources = Vec::new();
        rust_files(&root.join(surface), &mut sources);
        for source in sources {
            for (line_number, line) in fs::read_to_string(&source)
                .expect("source file")
                .lines()
                .enumerate()
            {
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
        "session-state boundary violations:\n{}",
        violations.join("\n")
    );
}
