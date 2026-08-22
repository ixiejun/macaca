//! Application ABI proofs for provider-neutral local-files discovery.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    device_local_files::device_local_files_pack_definition, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};
use std::sync::Arc;

#[test]
fn local_files_abi_projects_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: local-files-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.local.files.v1\n",
    )
    .unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(device_local_files_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.device.local.files.v1")
        .unwrap();
    assert!(projection
        .unavailable_commands
        .contains_key("local_files.read"));
}

#[test]
fn local_files_abi_rejects_unknown_permission_scope() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: local-files-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.device.local.files.v1\n  pack_permission_scopes:\n    pack.device.local.files.v1:\n      - device.local_files.native\n",
    )
    .unwrap();
    let mut definition = device_local_files_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    assert!(YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap_err()
        .to_string()
        .contains("local files permission scope"));
}
