//! Application ABI admission proofs for descriptor-owned audio scopes.

use std::sync::Arc;

use macaca_proto::{
    media_audio::media_audio_pack_definition, DomainPackAvailability, InMemoryDomainPackCatalog,
};

use super::*;
use crate::loader::AppLoader;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = media_audio_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn audio_permissions_are_validated_before_abi_projection() {
    let accepted = AppLoader::parse_manifest_yaml(
        "name: audio-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.media.audio.v1\n  pack_permission_scopes:\n    pack.media.audio.v1:\n      - audio.export\n      - audio.artifact.read\n",
    ).unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_ok());

    let rejected = AppLoader::parse_manifest_yaml(
        "name: audio-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.media.audio.v1\n  pack_permission_scopes:\n    pack.media.audio.v1:\n      - audio.provider.native\n",
    ).unwrap();
    assert!(YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog()))
        .load()
        .unwrap_err()
        .to_string()
        .contains("audio permission scope"));
}

#[test]
fn audio_abi_projects_commands_and_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: audio-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.media.audio.v1\n",
    ).unwrap();
    let mut unavailable_catalog = InMemoryDomainPackCatalog::new();
    unavailable_catalog.register(media_audio_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(unavailable_catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.media.audio.v1")
        .unwrap();
    assert!(projection
        .unavailable_commands
        .contains_key("audio.export_request"));
}
