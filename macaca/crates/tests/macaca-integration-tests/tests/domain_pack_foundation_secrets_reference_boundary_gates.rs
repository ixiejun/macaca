//! Foundation secrets-reference provider boundary gates.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "macaca-foundation-secrets-reference",
    "macaca_foundation_secrets_reference",
    "MockSecretsReferenceProvider",
    "UnavailableSecretsReferenceProvider",
];

#[test]
fn provider_neutral_layers_do_not_import_secret_provider_crates() {
    let root = root();
    assert_no_tokens(
        &root,
        [
            root.join("crates/kernel"),
            root.join("crates/facade/macaca-sdk"),
            root.join("crates/shells"),
            root.join("crates/application/macaca-app"),
        ],
    );
}

#[test]
fn secrets_sdk_and_wasm_imports_use_generic_service_runtime() {
    let root = root();
    assert_no_tokens(
        &root,
        [
            root.join("crates/facade/macaca-sdk/src/foundation_secrets_reference_client.rs"),
            root.join(
                "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge",
            ),
        ],
    );
}

fn assert_no_tokens<I>(root: &Path, surfaces: I)
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut violations = Vec::new();
    for surface in surfaces {
        let mut sources = Vec::new();
        collect(&surface, &mut sources);
        for source in sources {
            for (line, content) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if content.trim_start().starts_with("//") {
                    continue;
                }
                for token in FORBIDDEN {
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
        "secrets-reference boundary violations"
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

fn collect(path: &Path, output: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            collect(&entry.unwrap().path(), output);
        }
    } else if matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "toml")
    ) {
        output.push(path.to_path_buf());
    }
}
