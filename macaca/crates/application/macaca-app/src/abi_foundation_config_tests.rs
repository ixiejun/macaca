//! Foundation-config ABI projection tests.
//!
//! The projection is descriptor data shared by declarative, WASM, GenUI, and
//! headless applications. No entry form obtains a config provider, raw value,
//! environment handle, or source location from the application ABI.

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
        .find(|item| item.pack_id == "pack.foundation.config.v1")
        .unwrap()
}

#[test]
fn all_application_entry_forms_project_config_through_service_call() {
    let mut config = macaca_proto::foundation_config_pack_definition();
    config.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(config);
    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: config-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.config.v1\n  pack_permission_scopes:\n    pack.foundation.config.v1:\n      - config.read\n      - config.list\n      - config.validate\n      - config.watch\n      - config.reload\n      - config.snapshot\n      - config.export\n"
        )).unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(catalog.clone()))
            .load()
            .unwrap()
            .descriptor;
        let capability = projection(&descriptor);
        assert!(capability
            .callable_commands
            .contains("config.resolve_effective"));
        assert!(capability
            .callable_commands
            .contains("config.export_redacted"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(
            !descriptor
                .declaration
                .imports
                .iter()
                .any(|import| import.as_name().contains("config")),
            "{entry_kind} must not receive a config-specific runtime import"
        );
    }
}

#[test]
fn config_projection_separates_permission_denial_from_provider_unavailability() {
    let mut available = macaca_proto::foundation_config_pack_definition();
    available.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(available);
    let denied_manifest = AppLoader::parse_manifest_yaml(
        "name: config-denied\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.config.v1\n  pack_permission_scopes:\n    pack.foundation.config.v1:\n      - config.unknown\n"
    ).unwrap();
    let denied = YamlApplicationAbiAdapter::new(denied_manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let denied = projection(&denied);
    assert_eq!(
        denied.denied_commands.get("config.get").map(String::as_str),
        Some("permission_scope_not_declared")
    );
    assert!(denied.unavailable_commands.is_empty());
    assert!(denied.provider_capability_flags.contains_key("mock"));
    assert!(!denied.replay_refs.is_empty());

    let mut unavailable_catalog = InMemoryDomainPackCatalog::new();
    unavailable_catalog.register(macaca_proto::foundation_config_pack_definition());
    let unavailable_manifest = AppLoader::parse_manifest_yaml(
        "name: config-unavailable\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.config.v1\n"
    ).unwrap();
    let unavailable = YamlApplicationAbiAdapter::new(unavailable_manifest)
        .with_catalog(Arc::new(unavailable_catalog))
        .load()
        .unwrap()
        .descriptor;
    let unavailable = projection(&unavailable);
    assert!(unavailable.callable_commands.is_empty());
    assert!(unavailable.denied_commands.is_empty());
    assert_eq!(
        unavailable
            .unavailable_commands
            .get("config.get")
            .map(String::as_str),
        Some("config_provider_not_installed")
    );
}

#[test]
fn required_config_declarations_fail_admission_when_no_provider_is_available() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(macaca_proto::foundation_config_pack_definition());
    let manifest = AppLoader::parse_manifest_yaml(
        "name: required-config\nlayer: L3Declarative\nservice_contract:\n  required_packs:\n    - pack.foundation.config.v1\n",
    )
    .unwrap();
    let result = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load();
    assert!(
        result.is_err(),
        "required unavailable config must block admission"
    );
}
