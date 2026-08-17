//! Foundation-filesystem provider boundary gates.

use std::fs;
use std::path::{Path, PathBuf};

const PROVIDERS: &[&str] = &["MockFilesystemProvider", "UnavailableFilesystemProvider"];

#[test]
fn provider_neutral_layers_do_not_construct_foundation_filesystem_providers() {
    let root = root();
    let surfaces = [
        "crates/kernel",
        "crates/facade/macaca-sdk/src",
        "crates/shells",
        "crates/application/macaca-app/src",
    ];
    assert_no_tokens(
        &root,
        surfaces.iter().map(|path| root.join(path)),
        PROVIDERS,
    );
}

#[test]
fn filesystem_sdk_and_wasm_imports_cannot_bypass_the_service_runtime() {
    let root = root();
    assert_no_tokens(
        &root,
        [
            root.join("crates/facade/macaca-sdk/src/foundation_filesystem_client.rs"),
            root.join(
                "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge",
            ),
        ],
        PROVIDERS,
    );
}

fn assert_no_tokens<I>(root: &Path, surfaces: I, tokens: &[&str])
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut violations = Vec::new();
    for surface in surfaces {
        let mut sources = Vec::new();
        if surface.is_dir() {
            files(&surface, &mut sources);
        } else {
            sources.push(surface);
        }
        for source in sources {
            for (line, content) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if !content.trim_start().starts_with("//") {
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
    }
    assert!(
        violations.is_empty(),
        "foundation filesystem boundary violations:\n{}",
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
