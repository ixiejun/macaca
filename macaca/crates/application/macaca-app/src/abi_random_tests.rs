//! Foundation-random ABI projection tests.
//!
//! Every declared application form receives the same provider-neutral
//! `ServiceCall` import; this test exercises the YAML projection used by the
//! application framework and does not instantiate an RNG provider.

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
