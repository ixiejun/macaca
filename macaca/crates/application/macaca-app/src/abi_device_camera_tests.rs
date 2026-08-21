//! Application ABI proofs for descriptor-owned camera discovery and scopes.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    device_camera::device_camera_pack_definition, DomainPackAvailability, InMemoryDomainPackCatalog,
};
use std::sync::Arc;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = device_camera_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn camera_permissions_are_validated_before_abi_projection() {
    let accepted = AppLoader::parse_manifest_yaml("name: camera-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.camera.v1\n  pack_permission_scopes:\n    pack.device.camera.v1:\n      - device.camera.capture_photo\n").unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_ok());
    let rejected = AppLoader::parse_manifest_yaml("name: camera-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.camera.v1\n  pack_permission_scopes:\n    pack.device.camera.v1:\n      - device.camera.native\n").unwrap();
    assert!(YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog()))
        .load()
        .unwrap_err()
        .to_string()
        .contains("camera permission scope"));
}

#[test]
fn camera_abi_projects_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml("name: camera-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.camera.v1\n").unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(device_camera_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.device.camera.v1")
        .unwrap();
    assert!(projection
        .unavailable_commands
        .contains_key("camera.capture_photo"));
}
