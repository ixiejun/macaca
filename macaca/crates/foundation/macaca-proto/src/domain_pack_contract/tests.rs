use std::collections::BTreeMap;

use super::*;

#[test]
fn expansion_merges_packs_and_declared_services() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(DomainPackDefinition::with_metadata(
        "pack.example.v1",
        DomainPackMetadata {
            family_id: "example".into(),
            version: "v1".into(),
            ..Default::default()
        },
        [String::from("service.market_data")],
    ));
    let declaration = AppServiceContractConfig {
        use_packs: vec!["pack.example.v1".into()],
        required_services: vec!["service.custom.required".into()],
        optional_services: vec!["service.custom.optional".into()],
        service_policy_overrides: BTreeMap::new(),
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), &catalog);
    assert!(expanded.services.contains("service.market_data"));
    assert!(expanded.services.contains("service.custom.required"));
    assert!(expanded.services.contains("service.custom.optional"));
    assert!(expanded.unresolved_packs.is_empty());
    assert!(!expanded.capabilities_hash.is_empty());
    assert_eq!(
        expanded.service_sources.get("service.market_data"),
        Some(&"pack.example.v1".to_string())
    );
}

#[test]
fn pack_identity_and_hierarchy_validation_work() {
    let definition = DomainPackDefinition::with_metadata(
        "pack.finance.stock.v1",
        DomainPackMetadata {
            family_id: "finance".into(),
            parent_pack_id: Some("pack.finance.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            ..Default::default()
        },
        [String::from("service.market_data")],
    );
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert!(validate_domain_pack_parent("pack.finance.v1", "pack.finance.stock.v1").is_ok());
    assert!(validate_domain_pack_id("pack.finance.stock.v1").is_ok());
    assert!(validate_domain_pack_id("finance.stock.v1").is_err());
}

#[test]
fn expansion_tracks_optional_pack_degradation() {
    let catalog = InMemoryDomainPackCatalog::new();
    let declaration = AppServiceContractConfig {
        optional_packs: vec!["pack.knowledge.search.v1".into()],
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), &catalog);
    assert_eq!(
        expanded.unresolved_optional_packs,
        vec!["pack.knowledge.search.v1".to_string()]
    );
    assert_eq!(
        expanded.unresolved_packs,
        vec!["pack.knowledge.search.v1".to_string()]
    );
}

#[test]
fn expansion_tracks_unresolved_required_pack() {
    let catalog = InMemoryDomainPackCatalog::new();
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.developer.code.v1".into()],
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), &catalog);
    assert_eq!(
        expanded.unresolved_required_packs,
        vec!["pack.developer.code.v1".to_string()]
    );
}

#[test]
fn required_and_optional_pack_hash_is_stable() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(DomainPackDefinition::new(
        "pack.example.v1",
        [String::from("service.alpha"), String::from("service.beta")],
    ));
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.example.v1".into()],
        ..Default::default()
    };
    let first = expand_service_capabilities(Some(&declaration), &catalog);
    let second = expand_service_capabilities(Some(&declaration), &catalog);
    assert_eq!(first.capabilities_hash, second.capabilities_hash);
}

#[test]
fn service_contract_spec_validates_pack_override_keys() {
    let valid = AppServiceContractConfig {
        pack_policy_overrides: BTreeMap::from([("pack.example.v1".into(), Default::default())]),
        ..Default::default()
    };
    assert!(AppServiceContractSpec.validate(&valid).is_ok());

    let invalid = AppServiceContractConfig {
        pack_policy_overrides: BTreeMap::from([("example.v1".into(), Default::default())]),
        ..Default::default()
    };
    assert!(AppServiceContractSpec.validate(&invalid).is_err());
}

#[test]
fn parent_child_version_mismatch_is_rejected() {
    let definition = DomainPackDefinition::with_metadata(
        "pack.finance.stock.v2",
        DomainPackMetadata {
            family_id: "finance".into(),
            parent_pack_id: Some("pack.finance.v1".into()),
            version: "v2".into(),
            ..Default::default()
        },
        [String::from("service.market_data")],
    );
    assert!(DomainPackDefinitionSpec.validate(&definition).is_err());
}

#[test]
fn catalog_lists_registered_packs_in_stable_order() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(DomainPackDefinition::new(
        "pack.zeta.v1",
        [String::from("service.zeta")],
    ));
    catalog.register(DomainPackDefinition::new(
        "pack.alpha.v1",
        [String::from("service.alpha")],
    ));

    let ids = catalog
        .list()
        .into_iter()
        .map(|definition| definition.pack_id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["pack.alpha.v1", "pack.zeta.v1"]);
}

#[test]
fn reference_catalogs_are_descriptor_only_and_valid() {
    let definitions = reference_domain_pack_definitions();
    assert_eq!(definitions.len(), 3);
    for definition in definitions {
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert!(
            !definition.metadata.sdk.client_namespace.is_empty(),
            "reference packs must expose SDK discovery metadata"
        );
        assert!(
            !definition.metadata.permission_scopes.is_empty(),
            "reference packs must describe policy scopes"
        );
    }
}

#[test]
fn provider_snapshots_and_unavailable_diagnostics_are_structured() {
    let snapshot = DomainPackProviderSnapshot::registered(
        "pack.foundation.v1",
        "service.file",
        "trace-pack-foundation",
    );
    assert_eq!(snapshot.provider_class, "package");
    assert_eq!(snapshot.health, "registered");
    assert_eq!(snapshot.unavailable_reason, None);

    let unavailable = DomainPackUnavailableDiagnostic::new(
        "pack.absent.v1",
        true,
        "required_pack_unresolved",
        "required pack is absent or unavailable",
    );
    assert!(unavailable.required);
    assert_eq!(unavailable.reason_code, "required_pack_unresolved");
}
