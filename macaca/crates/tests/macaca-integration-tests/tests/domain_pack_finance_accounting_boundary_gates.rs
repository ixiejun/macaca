//! Finance-accounting boundary gates.
//!
//! These checks keep the generic OS layers provider-neutral and ensure the
//! accounting command surface does not absorb adjacent money workflows.

use std::fs;
use std::path::{Path, PathBuf};

const SURFACES: &[&str] = &[
    "crates/kernel",
    "crates/facade/macaca-sdk/src",
    "crates/shells",
    "crates/application/macaca-app/src",
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
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            output.push(path);
        }
    }
}

#[test]
fn accounting_provider_is_runtime_host_only() {
    let workspace = root();
    let forbidden = [
        "FinanceAccountingSystemServiceProvider",
        "finance_accounting_service_provider",
    ];
    let mut violations = Vec::new();
    for surface in SURFACES {
        let mut sources = Vec::new();
        files(&workspace.join(surface), &mut sources);
        for source in sources {
            for (line, text) in fs::read_to_string(&source).unwrap().lines().enumerate() {
                if !text.trim_start().starts_with("//")
                    && forbidden.iter().any(|token| text.contains(token))
                {
                    violations.push(format!("{}:{}", source.display(), line + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "accounting provider boundary violations: {violations:?}"
    );
}

#[test]
fn accounting_commands_exclude_adjacent_financial_workflows() {
    let source = fs::read_to_string(
        root()
            .join("crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting.rs"),
    )
    .unwrap();
    let commands = source
        .lines()
        .filter_map(|line| line.split('"').nth(1))
        .filter(|command| command.starts_with("accounting."))
        .collect::<Vec<_>>();
    for excluded in [
        "invoice",
        "payment",
        "transfer",
        "tax",
        "payroll",
        "portfolio",
    ] {
        assert!(
            commands.iter().all(|command| !command.contains(excluded)),
            "accounting command surface must exclude {excluded}"
        );
    }
}
