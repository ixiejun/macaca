//! Foundation-time application ABI projection tests.
//!
//! YAML is the manifest serialization shared by declarative, WASM, GenUI, and
//! headless application entries. The ABI therefore projects every declared time
//! command to the same `ServiceCall` import rather than creating runtime-specific
//! clock handles or application-owned execution branches.

use std::sync::Arc;

use macaca_proto::{ApplicationImport, DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

#[test]
fn all_application_entry_forms_project_time_commands_through_service_call() {
    let mut time = macaca_proto::foundation_time_pack_definition();
    time.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(time);

    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: time-abi-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.time.v1\n  pack_permission_scopes:\n    pack.foundation.time.v1:\n      - time.read\n      - time.monotonic\n      - time.timezone\n      - time.calendar\n      - time.format\n      - time.parse\n      - time.timer\n      - time.deadline\n"
        )).unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(catalog.clone()))
            .load()
            .unwrap()
            .descriptor;
        let projection = descriptor
            .service_capabilities
            .capability_projections
            .iter()
            .find(|projection| projection.pack_id == "pack.foundation.time.v1")
            .expect("declared time pack must produce a capability projection");
        for command in [
            "time.now",
            "time.monotonic_now",
            "time.clock_health",
            "time.create_timer",
            "time.cancel_timer",
            "time.evaluate_deadline",
        ] {
            assert!(
                projection.callable_commands.contains(command),
                "{entry_kind} must expose {command}"
            );
        }
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(!descriptor
            .declaration
            .imports
            .iter()
            .any(|import| import.as_name().contains("time")));
    }
}

#[test]
fn unavailable_time_pack_projects_diagnostics_without_provider_handles() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(macaca_proto::foundation_time_pack_definition());
    let manifest = AppLoader::parse_manifest_yaml("name: time-unavailable\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.time.v1\n").unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.foundation.time.v1")
        .unwrap();
    assert!(projection.callable_commands.is_empty());
    assert!(!projection.unavailable_features.is_empty());
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}
