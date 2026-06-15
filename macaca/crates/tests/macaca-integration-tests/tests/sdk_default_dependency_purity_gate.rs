//! SDK production dependency purity gate.
//!
//! Every SDK feature is part of the production facade surface. Optional
//! features must not hide runtime-host, provider, application-framework, or
//! optional-module dependencies from the terminal architecture gate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

fn direct_sdk_workspace_deps_with_all_features() -> BTreeSet<String> {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-e",
            "normal",
            "-p",
            "macaca-sdk",
            "--all-features",
            "--depth",
            "1",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run cargo tree for SDK dependency purity gate");

    assert!(
        output.status.success(),
        "cargo tree failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let direct = trimmed
                .strip_prefix("├── ")
                .or_else(|| trimmed.strip_prefix("└── "))?;
            let crate_name = direct.split_whitespace().next()?;
            (crate_name.starts_with("macaca-") && crate_name != "macaca-sdk")
                .then(|| crate_name.to_string())
        })
        .collect()
}

#[test]
fn sdk_all_features_depend_only_on_proto_workspace_contracts() {
    let observed = direct_sdk_workspace_deps_with_all_features();
    let permitted = BTreeSet::from(["macaca-proto".to_string()]);
    let forbidden: Vec<_> = observed.difference(&permitted).cloned().collect();

    assert!(
        forbidden.is_empty(),
        "SDK production dependency purity gate failed: forbidden workspace dependencies enabled by SDK features: {forbidden:?}\n\
         Terminal invariant: every macaca-sdk production feature may depend only on macaca-proto among workspace crates.\n\
         Provider, runtime-host, application-framework, and optional-module behavior must stay behind protocol contracts and approved composition roots."
    );
}
