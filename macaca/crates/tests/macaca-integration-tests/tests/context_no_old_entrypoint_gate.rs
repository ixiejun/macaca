//! Context engine old-entrypoint regression gate.
//!
//! Context assembly is a provider-neutral service capability. The default
//! behavior is the passthrough Strategy, not an old-path rollback API. This
//! gate prevents the retired context engine names and constructors from
//! reappearing in production sources that own or consume the context boundary.

use std::path::{Path, PathBuf};

/// One retired context entrypoint token and its stable replacement.
struct RetiredContextEntrypoint {
    token: String,
    replacement: &'static str,
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

fn read_workspace_file(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn joined_token(parts: &[&str]) -> String {
    parts.concat()
}

fn collect_context_boundary_sources() -> String {
    let files = [
        "crates/services/macaca-context/src/engine/mod.rs",
        "crates/services/macaca-context/src/engine/types.rs",
        "crates/services/macaca-context/src/engine/facade.rs",
        "crates/services/macaca-context/src/engine/registry.rs",
        "crates/services/macaca-context/src/engine/passthrough.rs",
        "crates/services/macaca-context/src/engine/helpers.rs",
        "crates/services/macaca-context/src/composer/facade.rs",
        "crates/services/macaca-context/src/composer/assembly_policy.rs",
        "crates/services/macaca-context/src/adapter.rs",
        "crates/services/macaca-context/src/lib.rs",
        "crates/runtime/macaca-host-composition/src/context_client.rs",
        "crates/shells/macaca-web/src/external_context_adapter.rs",
        "crates/shells/macaca-web/src/workspace_knowledge_digest_capability.rs",
    ];

    files
        .into_iter()
        .map(read_workspace_file)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn context_boundary_uses_passthrough_entrypoints_instead_of_old_names() {
    let source = collect_context_boundary_sources();
    let retired = [
        RetiredContextEntrypoint {
            token: "LegacyContextEngine".to_string(),
            replacement: "PassthroughContextEngine",
        },
        RetiredContextEntrypoint {
            token: "LEGACY_ENGINE_ID".to_string(),
            replacement: "PASSTHROUGH_ENGINE_ID",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["ContextAssembleInput::", "leg", "acy"]),
            replacement: "ContextAssembleInput::unscoped",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["ContextEngineSelection::", "leg", "acy"]),
            replacement: "ContextEngineSelection::passthrough_default",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["ContextRuntimeFacade::", "leg", "acy"]),
            replacement: "ContextRuntimeFacade::passthrough",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["ContextManagerFacade::", "leg", "acy"]),
            replacement: "ContextManagerFacade::passthrough",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["ContextFacade::", "leg", "acy"]),
            replacement: "ContextFacade::passthrough",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["with_", "leg", "acy"]),
            replacement: "with_passthrough",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["resolve_or_", "leg", "acy"]),
            replacement: "resolve_or_passthrough",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["leg", "acy_governance_only"]),
            replacement: "ContextFacadeAssemblyPolicy::from_context_config_parts",
        },
        RetiredContextEntrypoint {
            token: joined_token(&["build_", "leg", "acy_report"]),
            replacement: "build_passthrough_report",
        },
    ];

    let violations = retired
        .iter()
        .filter(|entrypoint| source.contains(&entrypoint.token))
        .map(|entrypoint| format!("{} -> {}", entrypoint.token, entrypoint.replacement))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "context boundary must not reintroduce old engine entrypoints:\n{}",
        violations.join("\n")
    );
}
