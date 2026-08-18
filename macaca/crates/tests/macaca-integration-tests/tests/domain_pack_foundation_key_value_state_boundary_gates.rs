//! Foundation key-value state provider boundary gates.

use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_TOKENS: &[&str] = &[
    "macaca-foundation-key-value-state",
    "macaca_foundation_key_value_state",
    "MockKeyValueStateProvider",
    "UnavailableKeyValueStateProvider",
    "KeyValueStateProviderFactory",
];

/// Kernel, SDK, shells, and the app framework use protocol DTOs and service calls only.
#[test]
fn provider_neutral_layers_do_not_import_key_value_state_providers() {
    let root = root();
    assert_no_tokens(
        &root,
        [
            root.join("crates/kernel"),
            root.join("crates/facade/macaca-sdk"),
            root.join("crates/shells"),
            root.join("crates/application/macaca-app"),
        ],
        PROVIDER_TOKENS,
    );
}

/// SDK helpers and production WASM imports must use the traced service runtime boundary.
#[test]
fn key_value_sdk_and_wasm_imports_cannot_bypass_service_runtime() {
    let root = root();
    assert_no_tokens(
        &root,
        [
            root.join("crates/facade/macaca-sdk/src/foundation_key_value_state_client.rs"),
            root.join("crates/facade/macaca-sdk/src/foundation_key_value_state_watch.rs"),
            root.join(
                "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge",
            ),
        ],
        PROVIDER_TOKENS,
    );
}

fn assert_no_tokens<I>(root: &Path, surfaces: I, tokens: &[&str])
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut violations = Vec::new();
    for surface in surfaces {
        let mut files_to_check = Vec::new();
        collect_files(&surface, &mut files_to_check);
        for source in files_to_check {
            for (line, content) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if content.trim_start().starts_with("//") {
                    continue;
                }
                for token in tokens {
                    if content.contains(token) {
                        violations.push(format!(
                            "{}:{}:{token}",
                            source.strip_prefix(root).unwrap().display(),
                            line + 1
                        ));
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "foundation key-value state boundary violations:\n{}",
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
        .unwrap()
}

fn collect_files(path: &Path, output: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            collect_files(&entry.unwrap().path(), output);
        }
    } else if matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "toml")
    ) {
        output.push(path.to_path_buf());
    }
}
