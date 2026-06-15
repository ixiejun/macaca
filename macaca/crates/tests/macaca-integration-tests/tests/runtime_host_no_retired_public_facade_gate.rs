//! Runtime-host public facade debt gate.
//!
//! The runtime host may expose stable Facade and service bootstrap APIs, but it
//! must not re-export implementation managers or migration-era bootstrap names.
//! This gate intentionally focuses on public facade debt that has already been
//! removed by the active OpenSpec change; repository-wide historical cleanup is
//! covered by the later zero-debt terminal gate.

use std::path::{Path, PathBuf};

/// A forbidden public-surface token with a stable replacement.
struct ForbiddenPublicFacadeToken {
    token: &'static str,
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

#[test]
fn runtime_host_public_facade_exposes_only_stable_mcp_and_entitlement_surfaces() {
    let source =
        read_workspace_file("crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs");
    let forbidden_tokens = [
        ForbiddenPublicFacadeToken {
            token: "McpRuntimeManager",
            replacement: "McpRuntimeFacade or typed service.mcp commands",
        },
        ForbiddenPublicFacadeToken {
            token: "EntitlementRuntimeFacade",
            replacement: "EntitlementSystemServiceProvider or SystemEntitlementClient",
        },
    ];

    let violations = forbidden_tokens
        .iter()
        .filter(|forbidden| source.contains(forbidden.token))
        .map(|forbidden| format!("{} -> {}", forbidden.token, forbidden.replacement))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "runtime-host public facade must not expose retired implementation facades:\n{}",
        violations.join("\n")
    );
}

#[test]
fn runtime_host_public_facade_uses_stable_optional_service_bootstrap_names() {
    let public_api =
        read_workspace_file("crates/runtime/macaca-runtime-host/src/runtime_host_public_api.rs");
    let crate_root = read_workspace_file("crates/runtime/macaca-runtime-host/src/lib.rs");
    let web_bootstrap = read_workspace_file(
        "crates/shells/macaca-web/src/composition_bootstrap/service_runtime_wiring.rs",
    );
    let combined_source = format!("{public_api}\n{crate_root}\n{web_bootstrap}");
    let forbidden_tokens = [
        ForbiddenPublicFacadeToken {
            token: concat!("route", "_c_bootstrap"),
            replacement: "optional_service_bootstrap",
        },
        ForbiddenPublicFacadeToken {
            token: concat!("bootstrap_route", "_c_optional_services"),
            replacement: "bootstrap_optional_services",
        },
        ForbiddenPublicFacadeToken {
            token: "RouteCOptionalServicesBootstrap",
            replacement: "OptionalServiceBootstrap",
        },
        ForbiddenPublicFacadeToken {
            token: "RouteCHostRuntimeBundle",
            replacement: "OptionalServiceRuntimeBundle",
        },
        ForbiddenPublicFacadeToken {
            token: "RouteCBootstrapDiagnostic",
            replacement: "OptionalServiceBootstrapDiagnostic",
        },
    ];

    let violations = forbidden_tokens
        .iter()
        .filter(|forbidden| combined_source.contains(forbidden.token))
        .map(|forbidden| format!("{} -> {}", forbidden.token, forbidden.replacement))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "optional service bootstrap public surface must use stable names:\n{}",
        violations.join("\n")
    );
}
