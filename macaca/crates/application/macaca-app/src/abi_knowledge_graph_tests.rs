//! Application ABI proofs for provider-neutral knowledge-graph discovery.

use super::*;
use crate::loader::AppLoader;
use macaca_proto::{
    knowledge_graph_pack_definition, DomainPackAvailability, InMemoryDomainPackCatalog,
    KNOWLEDGE_GRAPH_PACK_ID,
};
use std::sync::Arc;

#[test]
fn graph_abi_projects_declared_commands_and_unavailable_diagnostics() {
    let manifest = AppLoader::parse_manifest_yaml(
        "name: graph-optional\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.knowledge.graph.v1\n",
    )
    .unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(knowledge_graph_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(manifest)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let projection = descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|entry| entry.pack_id == KNOWLEDGE_GRAPH_PACK_ID)
        .unwrap();
    assert!(projection.unavailable_commands.contains_key("graph.query"));
    assert!(
        projection.unavailable_commands.contains_key("graph.query")
            || projection.denied_commands.contains_key("graph.query")
    );
}

#[test]
fn graph_abi_accepts_only_descriptor_owned_permission_scopes() {
    let accepted = AppLoader::parse_manifest_yaml(
        "name: graph-admitted\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.knowledge.graph.v1\n  pack_permission_scopes:\n    pack.knowledge.graph.v1:\n      - graph.query\n",
    )
    .unwrap();
    let mut definition = knowledge_graph_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    assert!(YamlApplicationAbiAdapter::new(accepted)
        .with_catalog(Arc::new(catalog.clone()))
        .load()
        .is_ok());

    let rejected = AppLoader::parse_manifest_yaml(
        "name: graph-rejected\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.knowledge.graph.v1\n  pack_permission_scopes:\n    pack.knowledge.graph.v1:\n      - graph.native_database\n",
    )
    .unwrap();
    assert!(YamlApplicationAbiAdapter::new(rejected)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap_err()
        .to_string()
        .contains("graph permission scope"));
}
