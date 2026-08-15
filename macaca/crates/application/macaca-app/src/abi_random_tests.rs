//! Foundation-random ABI projection tests.
//!
//! Every declared application form receives the same provider-neutral
//! `ServiceCall` import; these tests exercise the YAML projection used by the
//! application framework and do not instantiate an RNG provider.

use std::sync::Arc;

use macaca_proto::{ApplicationImport, DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

#[test]
fn application_abi_projects_foundation_random_through_service_call_only() {
    let manifest = AppLoader::parse_manifest_yaml(
        r#"
name: random-abi-fixture
layer: L2Wasm
service_contract:
  optional_packs:
    - pack.foundation.random.v1
  pack_permission_scopes:
    pack.foundation.random.v1:
      - random.generate
      - random.identifier
      - random.token
      - random.nonce
      - random.health
      - random.test_seed
"#,
    )
    .unwrap();
    let mut random = macaca_proto::foundation_random_pack_definition();
    random.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(random);

    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.foundation.random.v1")
        .expect("declared random pack must produce an ABI projection");
    for command in [
        "random.bytes",
        "random.uuid_v4",
        "random.token",
        "random.test_stream_bytes",
    ] {
        assert!(projection.callable_commands.contains(command));
    }
    assert!(descriptor
        .declaration
        .imports
        .contains(&ApplicationImport::ServiceCall));
}

#[test]
fn all_application_entry_forms_project_random_commands_through_service_call() {
    let mut random = macaca_proto::foundation_random_pack_definition();
    random.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(random);

    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: random-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.random.v1\n  pack_permission_scopes:\n    pack.foundation.random.v1:\n      - random.generate\n      - random.identifier\n      - random.token\n      - random.nonce\n      - random.health\n      - random.test_seed\n"
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
            .find(|projection| projection.pack_id == "pack.foundation.random.v1")
            .expect("declared random pack must produce a capability projection");
        assert!(projection.callable_commands.contains("random.bytes"));
        assert!(projection
            .callable_commands
            .contains("random.provider_capabilities"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(
            !descriptor
                .declaration
                .imports
                .iter()
                .any(|import| import.as_name().contains("random")),
            "{entry_kind} must not receive a provider-specific random import"
        );
    }
}

#[test]
fn random_capability_projection_distinguishes_denied_and_unavailable_states() {
    let mut available = macaca_proto::foundation_random_pack_definition();
    available.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(available);
    let denied_manifest = AppLoader::parse_manifest_yaml(
        "name: random-denied\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.random.v1\n  pack_permission_scopes:\n    pack.foundation.random.v1:\n      - random.not_declared\n",
    ).unwrap();
    let denied = YamlApplicationAbiAdapter::new(denied_manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let denied_projection = denied
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.foundation.random.v1")
        .unwrap();
    assert_eq!(
        denied_projection
            .denied_commands
            .get("random.bytes")
            .map(String::as_str),
        Some("permission_scope_not_declared")
    );
    assert!(denied_projection.unavailable_commands.is_empty());
    assert!(denied_projection
        .provider_capability_flags
        .contains_key("host-csprng"));
    assert!(!denied_projection.replay_refs.is_empty());

    let mut unavailable_catalog = InMemoryDomainPackCatalog::new();
    unavailable_catalog.register(macaca_proto::foundation_random_pack_definition());
    let unavailable_manifest = AppLoader::parse_manifest_yaml(
        "name: random-unavailable\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.random.v1\n",
    ).unwrap();
    let unavailable = YamlApplicationAbiAdapter::new(unavailable_manifest)
        .with_catalog(Arc::new(unavailable_catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = unavailable
        .service_capabilities
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == "pack.foundation.random.v1")
        .unwrap();
    assert!(projection.callable_commands.is_empty());
    assert!(projection.denied_commands.is_empty());
    assert_eq!(
        projection
            .unavailable_commands
            .get("random.bytes")
            .map(String::as_str),
        Some("random_provider_not_installed")
    );
}
