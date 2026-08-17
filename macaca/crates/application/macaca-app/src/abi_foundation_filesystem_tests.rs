//! Foundation-filesystem ABI effective-capability projection tests.
//!
//! All application forms share the generic service-call import; no application
//! receives a filesystem provider, content resolver, or host path handle.

use std::sync::Arc;

use macaca_proto::{ApplicationImport, DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

fn projection(
    descriptor: &ApplicationAbiDescriptor,
) -> &macaca_proto::DomainPackEffectiveCapabilityProjection {
    descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|item| item.pack_id == "pack.foundation.filesystem.v1")
        .unwrap()
}

fn available_catalog() -> InMemoryDomainPackCatalog {
    let mut definition = macaca_proto::foundation_filesystem_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn all_application_forms_project_filesystem_commands_through_service_call() {
    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: filesystem-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.filesystem.v1\n  pack_permission_scopes:\n    pack.foundation.filesystem.v1:\n      - filesystem.read\n      - filesystem.write\n      - filesystem.append\n      - filesystem.list\n      - filesystem.metadata\n      - filesystem.copy\n      - filesystem.move\n      - filesystem.delete\n      - filesystem.watch\n      - filesystem.temp\n      - filesystem.snapshot\n      - filesystem.restore\n"
        ))
        .unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(available_catalog()))
            .load()
            .unwrap()
            .descriptor;
        let capability = projection(&descriptor);
        assert!(capability
            .callable_commands
            .contains("filesystem.read_file"));
        assert!(capability
            .callable_commands
            .contains("filesystem.restore_snapshot"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(!descriptor
            .declaration
            .imports
            .iter()
            .any(|import| import.as_name().contains("filesystem")));
    }
}

#[test]
fn filesystem_projection_separates_permission_denial_from_unavailability() {
    let denied_manifest = AppLoader::parse_manifest_yaml(
        "name: filesystem-denied\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.filesystem.v1\n  pack_permission_scopes:\n    pack.foundation.filesystem.v1:\n      - filesystem.not_declared\n",
    )
    .unwrap();
    let denied_descriptor = YamlApplicationAbiAdapter::new(denied_manifest)
        .with_catalog(Arc::new(available_catalog()))
        .load()
        .unwrap()
        .descriptor;
    let denied = projection(&denied_descriptor);
    assert_eq!(
        denied
            .denied_commands
            .get("filesystem.read_file")
            .map(String::as_str),
        Some("permission_scope_not_declared")
    );
    assert!(denied.provider_capability_flags.contains_key("mock"));
    assert!(!denied.replay_refs.is_empty());

    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(macaca_proto::foundation_filesystem_pack_definition());
    let unavailable_manifest = AppLoader::parse_manifest_yaml(
        "name: filesystem-unavailable\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.foundation.filesystem.v1\n",
    )
    .unwrap();
    let unavailable_descriptor = YamlApplicationAbiAdapter::new(unavailable_manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let unavailable = projection(&unavailable_descriptor);
    assert!(unavailable.callable_commands.is_empty());
    assert_eq!(
        unavailable
            .unavailable_commands
            .get("filesystem.read_file")
            .map(String::as_str),
        Some("filesystem_provider_not_installed")
    );
}

#[test]
fn filesystem_root_declarations_are_admitted_before_abi_construction() {
    let catalog = available_catalog();
    let accepted = AppLoader::parse_manifest_yaml(
        "name: filesystem-roots\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.filesystem.v1\n  filesystem_roots:\n    - root_id: workspace\n      root_kind: app_workspace\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_ok());

    let invalid = AppLoader::parse_manifest_yaml(
        "name: invalid-filesystem-root\nlayer: L3Declarative\nservice_contract:\n  filesystem_roots:\n    - root_id: /private/path\n      root_kind: app_workspace\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(invalid)
        .with_catalog(Arc::new(catalog))
        .load()
        .is_err());
}
