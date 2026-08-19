//! Secret-reference manifest admission and capability projection proofs.

use std::sync::Arc;

use macaca_proto::{DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

fn catalog() -> InMemoryDomainPackCatalog {
    let mut definition = macaca_proto::foundation_secrets_reference_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn secret_reference_declarations_use_generic_service_call_projection() {
    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: secret-reference-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.secrets.reference.v1\n  secret_reference_declarations:\n    - reference_id: secret-ref\n      provider_class: mock\n      version_hint: current\n"
        )).unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(catalog()))
            .load()
            .unwrap()
            .descriptor;
        let projection = descriptor
            .service_capabilities
            .capability_projections
            .iter()
            .find(|projection| {
                projection.pack_id == macaca_proto::FOUNDATION_SECRETS_REFERENCE_PACK_ID
            })
            .unwrap();
        assert!(projection
            .callable_commands
            .contains("secrets.inspect_reference"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&macaca_proto::ApplicationImport::ServiceCall));
    }
}

#[test]
fn undeclared_secret_reference_metadata_is_rejected_before_abi_projection() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: undeclared-secret-reference\nlayer: L3Declarative\nservice_contract:\n  secret_reference_declarations:\n    - reference_id: secret-ref\n      provider_class: mock\n      version_hint: current\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog()))
        .load()
        .is_err());
}
