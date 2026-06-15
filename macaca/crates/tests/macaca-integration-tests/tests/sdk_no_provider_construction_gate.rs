//! SDK provider construction boundary gate.
//!
//! The SDK is a Facade and client layer. It may build typed commands, manifests,
//! and unavailable/service-backed clients, but it must not instantiate concrete
//! runtime providers, service runtimes, persistence backends, wallets, or chain
//! clients. Those composition decisions belong to runtime-host or external
//! composition roots.

use std::path::{Path, PathBuf};

/// One denied construction token with the expected ownership boundary.
struct ForbiddenConstruction {
    token: &'static str,
    owner: &'static str,
}

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

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn forbidden_constructions() -> Vec<ForbiddenConstruction> {
    vec![
        ForbiddenConstruction {
            token: "ServiceRuntime::new(",
            owner: "runtime-host composition root",
        },
        ForbiddenConstruction {
            token: "RedbStore::new(",
            owner: "runtime-host persistence bootstrap",
        },
        ForbiddenConstruction {
            token: "InMemoryEntitlementStore::new(",
            owner: "entitlement service provider",
        },
        ForbiddenConstruction {
            token: "InMemoryPaymentStore::new(",
            owner: "payment service provider",
        },
        ForbiddenConstruction {
            token: "McpRuntimeFacade::load_default(",
            owner: "runtime-host MCP bootstrap",
        },
        ForbiddenConstruction {
            token: "AlertSystemServiceProvider::",
            owner: "alert service bootstrap",
        },
        ForbiddenConstruction {
            token: "LlmSystemServiceProvider::",
            owner: "LLM service bootstrap",
        },
        ForbiddenConstruction {
            token: "MemorySystemServiceProvider::",
            owner: "memory service bootstrap",
        },
        ForbiddenConstruction {
            token: "ContextSystemServiceProvider::",
            owner: "context service bootstrap",
        },
        ForbiddenConstruction {
            token: "EvmSystemServiceProvider::",
            owner: "EVM optional service bootstrap",
        },
        ForbiddenConstruction {
            token: "Web3SystemServiceProvider::",
            owner: "Web3 optional service bootstrap",
        },
    ]
}

#[test]
fn sdk_does_not_construct_runtime_providers_or_backends() {
    let root = workspace_root();
    let sdk_src = root.join("crates/facade/macaca-sdk/src");
    let mut files = Vec::new();
    collect_rust_files(&sdk_src, &mut files);
    files.sort();

    let forbidden = forbidden_constructions();
    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .expect("SDK source should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in content.lines().enumerate() {
            for forbidden in &forbidden {
                if line.contains(forbidden.token) {
                    violations.push(format!(
                        "{}:{}: {} must be owned by {}",
                        relative,
                        index + 1,
                        forbidden.token,
                        forbidden.owner
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "SDK provider/runtime construction violations:\n{}",
        violations.join("\n")
    );
}
