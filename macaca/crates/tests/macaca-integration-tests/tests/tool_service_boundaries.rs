//! Boundary checks for the industrial tool-family provider implementation.
//!
//! The test reads the runtime-host source directly because the contract is
//! architectural: the provider catalog must remain inside the service-owned
//! runtime host, advertise generic extension paths, and avoid application-name
//! or provider-id control-flow shortcuts.

use std::fs;
use std::path::PathBuf;

/// Concatenate module sources for architectural boundary scanning.
fn read_module_sources(root: &PathBuf, relative_paths: &[&str]) -> String {
    relative_paths
        .iter()
        .map(|path| {
            fs::read_to_string(root.join(path))
                .unwrap_or_else(|_| panic!("tool family provider module should exist: {path}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn tool_family_providers_stay_inside_runtime_host_and_do_not_branch_on_apps() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = read_module_sources(
        &root,
        &[
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/mod.rs",
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/constants.rs",
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/family_catalog.rs",
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/descriptor_builder.rs",
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/inventory.rs",
            "crates/runtime/macaca-runtime-host/src/tool_family_providers/planning_factory.rs",
        ],
    );

    assert!(source.contains("service.tool.family"));
    assert!(source.contains("provider_path"));
    assert!(!source.contains("application_id =="));
    assert!(!source.contains("app_name"));
    assert!(!source.contains("match provider_id"));
    assert!(!source.contains("RAW_PROVIDER_PAYLOAD"));
}
