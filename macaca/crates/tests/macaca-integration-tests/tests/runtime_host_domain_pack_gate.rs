//! VC-hardcoded gate: base runtime-host must not embed finance/crypto domain logic.
//!
//! Domain packs are optional packages.  Production sources under
//! `macaca-runtime-host/src` therefore must not reference exchange names,
//! finance pack ids, or finance-specific command strings.

use std::fs;
use std::path::{Path, PathBuf};

/// Forbidden finance/crypto domain tokens in base runtime-host production code.
const FORBIDDEN_TOKENS: &[&str] = &[
    "Binance",
    "OKX",
    "Coindesk",
    "coindesk",
    "pack.finance",
    "finance.lookup",
    "finance.analyze",
    "api.binance.com",
    "okx.com",
];

fn workspace_root() -> PathBuf {
    for ancestor in PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("macaca workspace root not found");
}

fn runtime_host_sources(root: &Path) -> Vec<PathBuf> {
    let src_root = root.join("crates/runtime/macaca-runtime-host/src");
    let mut files = Vec::new();
    collect_rust_files(&src_root, &mut files);
    files.sort();
    files
}

fn boundary_sources(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative in [
        "crates/kernel",
        "crates/facade/macaca-sdk/src",
        "crates/shells",
        "crates/runtime/macaca-runtime-host/src",
    ] {
        collect_rust_files(&root.join(relative), &mut files);
    }
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", dir.display());
    });
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

#[test]
fn runtime_host_production_sources_contain_no_finance_domain_tokens() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for path in runtime_host_sources(&root) {
        let content = fs::read_to_string(&path).expect("runtime-host source readable");
        for (line_no, line) in content.lines().enumerate() {
            if is_comment_only_line(line) {
                continue;
            }
            for token in FORBIDDEN_TOKENS {
                if line.contains(token) {
                    let relative = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    violations.push(format!("{relative}:{}: contains `{token}`", line_no + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "base runtime-host must not embed finance domain strings:\n{}",
        violations.join("\n")
    );
}

#[test]
fn os_boundaries_do_not_import_optional_domain_pack_packages() {
    let root = workspace_root();
    let forbidden_import_tokens = [
        "macaca_domain_pack_finance",
        "macaca-domain-pack-finance",
        "finance_domain_pack_registrations",
        "finance_pack_catalog_definition",
    ];
    let mut violations = Vec::new();

    for path in boundary_sources(&root) {
        let content = fs::read_to_string(&path).expect("boundary source readable");
        for (line_no, line) in content.lines().enumerate() {
            if is_comment_only_line(line) {
                continue;
            }
            for token in forbidden_import_tokens {
                if line.contains(token) {
                    let relative = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    violations.push(format!("{relative}:{}: contains `{token}`", line_no + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "kernel, SDK, shells, and base runtime-host must use SDK/catalog descriptors instead of optional package imports:\n{}",
        violations.join("\n")
    );
}
