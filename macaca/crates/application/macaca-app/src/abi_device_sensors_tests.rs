//! Application ABI proofs for provider-neutral sensor admission.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    device_sensors::device_sensors_pack_definition, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};
use std::sync::Arc;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = device_sensors_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn sensor_permissions_are_validated_before_abi_projection() {
    let accepted = AppLoader::parse_manifest_yaml(
        "name: sensors-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.sensors.v1\n  pack_permission_scopes:\n    pack.device.sensors.v1:\n      - device.sensors.read\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_ok());
    let rejected = AppLoader::parse_manifest_yaml(
        "name: sensors-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.sensors.v1\n  pack_permission_scopes:\n    pack.device.sensors.v1:\n      - device.sensors.native\n",
    )
    .unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog()))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.device.sensors.v1")
        .unwrap();
    assert!(projection.denied_commands.contains_key("sensors.read"));
}

#[test]
fn sensor_abi_projects_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: sensors-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.sensors.v1\n",
    )
    .unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(device_sensors_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.device.sensors.v1")
        .unwrap();
    assert!(projection.unavailable_commands.contains_key("sensors.read"));
}
