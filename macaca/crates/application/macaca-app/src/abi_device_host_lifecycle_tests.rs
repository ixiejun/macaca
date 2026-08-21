//! Application ABI proofs for host-lifecycle descriptor discovery and scopes.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    device_foreground_background_host::device_foreground_background_host_pack_definition,
    DomainPackAvailability, InMemoryDomainPackCatalog,
};
use std::sync::Arc;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = device_foreground_background_host_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn host_lifecycle_permissions_are_validated_before_abi_projection() {
    let accepted = AppLoader::parse_manifest_yaml("name: lifecycle-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.foreground_background_host.v1\n  pack_permission_scopes:\n    pack.device.foreground_background_host.v1:\n      - device.host_lifecycle.background\n").unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_ok());
    let rejected = AppLoader::parse_manifest_yaml("name: lifecycle-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.foreground_background_host.v1\n  pack_permission_scopes:\n    pack.device.foreground_background_host.v1:\n      - device.host_lifecycle.native\n").unwrap();
    assert!(YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog()))
        .load()
        .unwrap_err()
        .to_string()
        .contains("host lifecycle permission scope"));
}

#[test]
fn host_lifecycle_abi_projects_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml("name: lifecycle-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.foreground_background_host.v1\n").unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(device_foreground_background_host_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.device.foreground_background_host.v1")
        .unwrap();
    assert!(projection
        .unavailable_commands
        .contains_key("host_lifecycle.request_background_lease"));
}
