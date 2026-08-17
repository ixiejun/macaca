//! Key-value state namespace admission proofs at the common application ABI boundary.

use std::sync::Arc;

use macaca_proto::{DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

fn available_catalog() -> InMemoryDomainPackCatalog {
    let mut definition = macaca_proto::foundation_key_value_state_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

#[test]
fn key_value_namespaces_are_admitted_before_abi_construction() {
    let catalog = available_catalog();
    let accepted = AppLoader::parse_manifest_yaml(
        "name: key-value-state\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  key_value_namespaces:\n    - namespace: preferences\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_ok());

    let undeclared = AppLoader::parse_manifest_yaml(
        "name: undeclared-key-value-state\nlayer: L3Declarative\nservice_contract:\n  key_value_namespaces:\n    - namespace: preferences\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(undeclared)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_err());

    let invalid = AppLoader::parse_manifest_yaml(
        "name: invalid-key-value-state\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.key.value.state.v1\n  key_value_namespaces:\n    - namespace: ../../private\n      tenant_ref: tenant:example\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(invalid)
        .with_catalog(Arc::new(catalog))
        .load()
        .is_err());
}
