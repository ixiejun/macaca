//! Application ABI proofs for provider-neutral auth-handoff discovery.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    identity_auth_handoff::identity_auth_handoff_pack_definition, DomainPackAvailability,
    InMemoryDomainPackCatalog,
};
use std::sync::Arc;

#[test]
fn auth_handoff_abi_projects_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: auth-handoff-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.identity.auth.handoff.v1\n",
    )
    .unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(identity_auth_handoff_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == "pack.identity.auth.handoff.v1")
        .unwrap();
    assert!(projection
        .unavailable_commands
        .contains_key("auth_handoff.start_handoff"));
}

#[test]
fn auth_handoff_abi_rejects_unknown_permission_scope() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: auth-handoff-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.identity.auth.handoff.v1\n  pack_permission_scopes:\n    pack.identity.auth.handoff.v1:\n      - identity.auth.native\n",
    )
    .unwrap();
    let mut definition = identity_auth_handoff_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    assert!(YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap_err()
        .to_string()
        .contains("auth handoff permission scope"));
}
