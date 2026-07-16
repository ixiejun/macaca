//! Parser engines and file-format adapters must remain runtime-host details.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
];
const FORBIDDEN: &[&str] = &[
    "DocumentParsingSystemServiceProvider",
    "document_parsing_service_provider",
    "TextractParserProvider",
    "AzureDocumentParserProvider",
    "GoogleDocumentParserProvider",
    "TikaParserAdapter",
    "UnstructuredParserAdapter",
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
fn files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
#[test]
fn document_parsing_boundaries_do_not_import_or_construct_concrete_providers() {
    let root = root();
    let mut violations = Vec::new();
    for surface in SURFACES {
        let mut sources = Vec::new();
        files(&root.join(surface), &mut sources);
        for source in sources {
            for (line_number, line) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if !line.trim_start().starts_with("//") {
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
    }
    assert!(
        violations.is_empty(),
        "document parsing provider boundary violations:\n{}",
        violations.join("\n")
    );
}
