//! Key-value state namespace admission proofs at the common application ABI boundary.

use std::sync::Arc;

use macaca_proto::{ApplicationImport, DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

fn available_catalog() -> InMemoryDomainPackCatalog {
    let mut definition = macaca_proto::foundation_key_value_state_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

fn projection(
    descriptor: &ApplicationAbiDescriptor,
) -> &macaca_proto::DomainPackEffectiveCapabilityProjection {
    descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|item| item.pack_id == "pack.foundation.key.value.state.v1")
        .unwrap()
}

#[test]
fn all_application_forms_project_key_value_commands_through_service_call() {
    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: key-value-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  pack_permission_scopes:\n    pack.foundation.key.value.state.v1:\n      - state.read\n      - state.write\n      - state.delete\n      - state.list\n      - state.watch\n      - state.ttl\n      - state.counter\n      - state.snapshot\n      - state.restore\n      - state.migrate\n      - state.compact\n"
        )).unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(available_catalog()))
            .load()
            .unwrap()
            .descriptor;
        let capability = projection(&descriptor);
        assert!(capability.callable_commands.contains("kv.get"));
        assert!(capability
            .callable_commands
            .contains("kv.compact_namespace"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(!descriptor
            .declaration
            .imports
            .iter()
            .any(|import| import.as_name().contains("key_value")));
    }
}

#[test]
fn key_value_projection_separates_permission_denial_from_unavailability() {
    let denied = AppLoader::parse_manifest_yaml(
        "name: key-value-denied\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  pack_permission_scopes:\n    pack.foundation.key.value.state.v1:\n      - state.unknown\n",
    ).unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(denied)
        .with_catalog(Arc::new(available_catalog()))
        .load()
        .unwrap()
        .descriptor;
    let denied = projection(&descriptor);
    assert_eq!(
        denied.denied_commands.get("kv.get").map(String::as_str),
        Some("permission_scope_not_declared")
    );
    assert!(denied.provider_capability_flags.contains_key("mock"));
    assert!(!denied.replay_refs.is_empty());

    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(macaca_proto::foundation_key_value_state_pack_definition());
    let unavailable = AppLoader::parse_manifest_yaml(
        "name: key-value-unavailable\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n",
    ).unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(unavailable)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let unavailable = projection(&descriptor);
    assert!(unavailable.callable_commands.is_empty());
    assert_eq!(
        unavailable
            .unavailable_commands
            .get("kv.get")
            .map(String::as_str),
        Some("key_value_state_provider_not_installed")
    );
}

#[test]
fn key_value_namespaces_are_admitted_before_abi_construction() {
    let catalog = available_catalog();
    let accepted = AppLoader::parse_manifest_yaml(
        "name: key-value-state\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  key_value_namespaces:\n    - namespace: preferences\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_ok());

    let undeclared = AppLoader::parse_manifest_yaml(
        "name: undeclared-key-value-state\nlayer: L3Declarative\nservice_contract:\n  key_value_namespaces:\n    - namespace: preferences\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(undeclared)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_err());

    let invalid = AppLoader::parse_manifest_yaml(
        "name: invalid-key-value-state\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  key_value_namespaces:\n    - namespace: ../../private\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(invalid)
        .with_catalog(Arc::new(catalog))
        .load()
        .is_err());
}
