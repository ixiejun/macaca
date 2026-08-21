//! Session-state application admission and effective capability proofs.

use std::sync::Arc;

use macaca_proto::{ApplicationImport, DomainPackAvailability, InMemoryDomainPackCatalog};

use super::*;
use crate::loader::AppLoader;

fn available_catalog() -> InMemoryDomainPackCatalog {
    let mut definition = macaca_proto::foundation_session_state_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    catalog
}

fn projection(
    descriptor: &ApplicationAbiDescriptor,
) -> &macaca_proto::DomainPackEffectiveCapabilityProjection {
    descriptor
        .service_capabilities
        .capability_projections
        .iter()
        .find(|item| item.pack_id == macaca_proto::FOUNDATION_SESSION_STATE_PACK_ID)
        .unwrap()
}

#[test]
fn all_application_forms_use_the_same_session_state_service_call_projection() {
    for (entry_kind, layer) in [
        ("yaml", "L3Declarative"),
        ("wasm", "L2Wasm"),
        ("genui", "L2Wasm"),
        ("headless", "L1Native"),
    ] {
        let manifest = AppLoader::parse_manifest_yaml(&format!(
            "name: session-state-{entry_kind}\nlayer: {layer}\nservice_contract:\n  optional_packs:\n    - pack.foundation.session.state.v1\n  pack_permission_scopes:\n    pack.foundation.session.state.v1:\n      - session_state.read\n      - session_state.write\n      - session_state.delete\n      - session_state.list\n      - session_state.checkpoint\n      - session_state.restore\n      - session_state.compact\n      - session_state.clear\n      - session_state.export\n      - session_state.inspect_recovery\n"
        ))
        .unwrap();
        let descriptor = YamlApplicationAbiAdapter::new(manifest)
            .with_catalog(Arc::new(available_catalog()))
            .load()
            .unwrap()
            .descriptor;
        let capability = projection(&descriptor);
        assert!(capability.callable_commands.contains("session_state.get"));
        assert!(capability
            .callable_commands
            .contains("session_state.restore_checkpoint"));
        assert!(descriptor
            .declaration
            .imports
            .contains(&ApplicationImport::ServiceCall));
        assert!(!descriptor
            .declaration
            .imports
            .iter()
            .any(|import| import.as_name().contains("session_state_provider")));
    }
}

#[test]
fn session_state_projection_distinguishes_permission_denial_and_unavailability() {
    let denied = AppLoader::parse_manifest_yaml(
        "name: session-state-denied\nlayer: L3Declarative\nservice_contract:\n  optional_packs:\n    - pack.foundation.session.state.v1\n  pack_permission_scopes:\n    pack.foundation.session.state.v1:\n      - session_state.unknown\n",
    )
    .unwrap();
    let descriptor = YamlApplicationAbiAdapter::new(denied)
        .with_catalog(Arc::new(available_catalog()))
        .load()
        .unwrap()
        .descriptor;
    let denied = projection(&descriptor);
    assert!(!denied.denied_commands.is_empty());
    assert!(!denied.replay_refs.is_empty());

    let unavailable = AppLoader::parse_manifest_yaml(
        "name: session-state-unavailable\nlayer: L2Wasm\nservice_contract:\n  optional_packs:\n    - pack.foundation.session.state.v1\n",
    )
    .unwrap();
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(macaca_proto::foundation_session_state_pack_definition());
    let descriptor = YamlApplicationAbiAdapter::new(unavailable)
        .with_catalog(Arc::new(catalog))
        .load()
        .unwrap()
        .descriptor;
    let unavailable = projection(&descriptor);
    assert!(unavailable.callable_commands.is_empty());
    assert_eq!(
        unavailable
            .unavailable_commands
            .get("session_state.get")
            .map(String::as_str),
        Some("session_state_provider_not_installed")
    );
}
