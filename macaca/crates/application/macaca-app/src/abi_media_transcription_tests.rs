//! Application ABI admission proofs for descriptor-owned transcription scopes.

use std::sync::Arc;

use macaca_proto::{
    media_transcription::media_transcription_pack_definition, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};

use super::*;
use crate::loader::AppLoader;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = media_transcription_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn transcription_permission_scopes_are_validated_before_abi_projection() {
    let accepted = AppLoader::parse_manifest_yaml(
        "name: transcription-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.media.transcription.v1\n  pack_permission_scopes:\n    pack.media.transcription.v1:\n      - transcription.stream\n      - transcription.job.read\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_ok());

    let rejected = AppLoader::parse_manifest_yaml(
        "name: transcription-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.media.transcription.v1\n  pack_permission_scopes:\n    pack.media.transcription.v1:\n      - transcription.provider.native\n",
    )
    .unwrap();
    let error = YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog()))
        .load()
        .unwrap_err();
    assert!(error.to_string().contains("transcription permission scope"));
}
