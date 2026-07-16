use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn pack_permission_declarations_are_granted_only_from_descriptor_allowlists() {
    let mut definition = communication_notification_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(definition);
    let declaration = AppServiceContractConfig {
        required_packs: vec![COMMUNICATION_NOTIFICATION_PACK_ID.into()],
        pack_permission_scopes: BTreeMap::from([(
            COMMUNICATION_NOTIFICATION_PACK_ID.into(),
            BTreeSet::from(["notification.publish".into()]),
        )]),
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), &catalog);
    assert!(expanded
        .services
        .contains(COMMUNICATION_NOTIFICATION_SERVICE_ID));
    assert_eq!(
        expanded
            .granted_pack_permission_scopes
            .get(COMMUNICATION_NOTIFICATION_PACK_ID),
        Some(&BTreeSet::from(["notification.publish".into()]))
    );
}

#[test]
fn every_industrial_pack_accepts_and_rejects_declared_permission_scopes() {
    for mut definition in industrial_reference_domain_pack_definitions() {
        definition.metadata.availability = DomainPackAvailability::Available;
        let pack_id = definition.pack_id.clone();
        let scopes = definition.metadata.permission_scopes.clone();
        let mut catalog = InMemoryDomainPackCatalog::new();
        catalog.register(definition);
        let accepted = AppServiceContractConfig {
            required_packs: vec![pack_id.clone()],
            pack_permission_scopes: BTreeMap::from([(pack_id.clone(), scopes.clone())]),
            ..Default::default()
        };
        let expanded = expand_service_capabilities(Some(&accepted), &catalog);
        assert_eq!(
            expanded.granted_pack_permission_scopes.get(&pack_id),
            Some(&scopes)
        );
        let rejected = AppServiceContractConfig {
            required_packs: vec![pack_id.clone()],
            pack_permission_scopes: BTreeMap::from([(
                pack_id.clone(),
                BTreeSet::from(["scope.not_declared".into()]),
            )]),
            ..Default::default()
        };
        let expanded = expand_service_capabilities(Some(&rejected), &catalog);
        assert_eq!(
            expanded.unavailable_pack_reasons.get(&pack_id),
            Some(&"permission_scope_not_declared".to_string())
        );
    }
}
