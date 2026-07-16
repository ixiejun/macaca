use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[path = "domain_pack_catalog_projection_tests.rs"]
mod domain_pack_catalog_projection_tests;

#[path = "ai_contract_validation_tests.rs"]
mod ai_contract_validation_tests;
#[path = "ai_preflight_tests.rs"]
mod ai_preflight_tests;
#[path = "ai_tests.rs"]
mod ai_tests;
#[path = "commerce_post_purchase_tests.rs"]
mod commerce_post_purchase_tests;
#[path = "commerce_preflight_tests.rs"]
mod commerce_preflight_tests;
#[path = "commerce_tests.rs"]
mod commerce_tests;
#[path = "communication_calendar_preflight_tests.rs"]
mod communication_calendar_preflight_tests;
#[path = "communication_email_preflight_tests.rs"]
mod communication_email_preflight_tests;
#[path = "communication_inbox_preflight_tests.rs"]
mod communication_inbox_preflight_tests;
#[path = "communication_messaging_preflight_tests.rs"]
mod communication_messaging_preflight_tests;
#[path = "communication_notification_preflight_tests.rs"]
mod communication_notification_preflight_tests;
#[path = "communication_preflight_tests.rs"]
mod communication_preflight_tests;
#[path = "communication_tests.rs"]
mod communication_tests;
#[path = "developer_tests.rs"]
mod developer_tests;
#[path = "device_preflight_tests.rs"]
mod device_preflight_tests;
#[path = "device_tests.rs"]
mod device_tests;
#[path = "finance_accounting_tests.rs"]
mod finance_accounting_tests;
#[path = "finance_invoice_preflight_tests.rs"]
mod finance_invoice_preflight_tests;
#[path = "finance_portfolio_preflight_tests.rs"]
mod finance_portfolio_preflight_tests;
#[path = "finance_tests.rs"]
mod finance_tests;
#[path = "foundation_config_tests.rs"]
mod foundation_config_tests;
#[path = "foundation_filesystem_tests.rs"]
mod foundation_filesystem_tests;
#[path = "foundation_key_value_state_tests.rs"]
mod foundation_key_value_state_tests;
#[path = "foundation_preflight_tests.rs"]
mod foundation_preflight_tests;
#[path = "foundation_random_tests.rs"]
mod foundation_random_tests;
#[path = "foundation_secrets_reference_tests.rs"]
mod foundation_secrets_reference_tests;
#[path = "foundation_session_state_tests.rs"]
mod foundation_session_state_tests;
#[path = "foundation_time_tests.rs"]
mod foundation_time_tests;
#[path = "identity_preflight_tests.rs"]
mod identity_preflight_tests;
#[path = "identity_tests.rs"]
mod identity_tests;
#[path = "knowledge_tests.rs"]
mod knowledge_tests;
#[path = "location_tests.rs"]
mod location_tests;
#[path = "media_tests.rs"]
mod media_tests;
#[path = "office_tests.rs"]
mod office_tests;
#[path = "permission_admission_tests.rs"]
mod permission_admission_tests;
#[path = "provider_capability_tests.rs"]
mod provider_capability_tests;
#[path = "provider_descriptor_tests.rs"]
mod provider_descriptor_tests;
#[path = "workflow_preflight_tests.rs"]
mod workflow_preflight_tests;
#[path = "workflow_task_approval_spec_tests.rs"]
mod workflow_task_approval_spec_tests;
#[path = "workflow_task_dispatch_gate_tests.rs"]
mod workflow_task_dispatch_gate_tests;
#[path = "workflow_task_lifecycle_event_tests.rs"]
mod workflow_task_lifecycle_event_tests;
#[path = "workflow_task_lifecycle_spec_tests.rs"]
mod workflow_task_lifecycle_spec_tests;
#[path = "workflow_task_resource_spec_tests.rs"]
mod workflow_task_resource_spec_tests;
#[path = "workflow_task_transition_tests.rs"]
mod workflow_task_transition_tests;
#[path = "workflow_tests.rs"]
mod workflow_tests;

#[test]
fn expansion_merges_packs_and_declared_services() {
    let mut catalog = InMemoryDomainPackCatalog::new();
    catalog.register(DomainPackDefinition::with_metadata(
        "pack.example.v1",
        DomainPackMetadata {
            family_id: "example".into(),
            version: "v1".into(),
            service_command_schemas: BTreeMap::from([(
                "service.market_data".into(),
                BTreeSet::from(["market.quote.v1".into()]),
            )]),
            service_result_schemas: BTreeMap::from([(
                "service.market_data".into(),
                BTreeSet::from(["market.quote.result.v1".into()]),
            )]),
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
            service_command_schemas: BTreeMap::from([(
                "service.market_data".into(),
                BTreeSet::from(["finance.stock.quote.v1".into()]),
            )]),
            service_result_schemas: BTreeMap::from([(
                "service.market_data".into(),
                BTreeSet::from(["finance.stock.quote.result.v1".into()]),
            )]),
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
    assert_eq!(definitions.len(), 77);
    for definition in definitions {
        assert!(
            DomainPackDefinitionSpec.validate(&definition).is_ok(),
            "reference pack {} failed descriptor validation",
            definition.pack_id
        );
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
fn industrial_reference_catalog_contains_required_sub_pack_taxonomy() {
    let definitions = industrial_reference_domain_pack_definitions();
    assert_eq!(definitions.len(), 74);
    let ids = definitions
        .iter()
        .map(|definition| definition.pack_id.as_str())
        .collect::<BTreeSet<_>>();

    for expected in [
        "pack.foundation.filesystem.v1",
        "pack.communication.email.v1",
        "pack.knowledge.search.v1",
        "pack.developer.repository.v1",
        "pack.office.pdf.v1",
        "pack.media.transcription.v1",
        "pack.finance.market.data.v1",
        "pack.commerce.payment.intent.v1",
        "pack.identity.auth.handoff.v1",
        "pack.location.place.search.v1",
        "pack.device.foreground_background_host.v1",
        "pack.ai.model.evaluation.v1",
        "pack.workflow.recovery.v1",
    ] {
        assert!(
            ids.contains(expected),
            "missing industrial sub-pack {expected}"
        );
    }
    assert!(definitions
        .iter()
        .all(|definition| !definition.is_callable()));
}

#[test]
fn domain_pack_descriptor_hash_is_stable_and_descriptor_owned() {
    let definition = DomainPackDefinition::with_metadata(
        "pack.ai.llm.v1",
        DomainPackMetadata {
            family_id: "ai".into(),
            parent_pack_id: Some("pack.ai.v1".into()),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            permission_scopes: BTreeSet::from(["ai.llm.invoke".into()]),
            diagnostics: DomainPackDiagnostics {
                replay_schema: "trace.domain_pack.ai.llm.v1".into(),
                ..Default::default()
            },
            compatibility: DomainPackCompatibility {
                version_range: "^1".into(),
                parent_version_range: "^1".into(),
                service_version_ranges: BTreeMap::new(),
            },
            ..Default::default()
        },
        Vec::<String>::new(),
    );
    let same_definition = definition.clone();
    let mut changed_definition = definition.clone();
    changed_definition
        .metadata
        .permission_scopes
        .insert("ai.llm.budget".into());

    assert_eq!(
        definition.stable_descriptor_hash(),
        same_definition.stable_descriptor_hash(),
        "descriptor hashes must be deterministic for replay and audit evidence"
    );
    assert_ne!(
        definition.stable_descriptor_hash(),
        changed_definition.stable_descriptor_hash(),
        "descriptor hashes must change when provider-neutral schema metadata changes"
    );
}

#[test]
fn industrial_reference_descriptor_hashes_and_compatibility_are_valid() {
    let definitions = industrial_reference_domain_pack_definitions();
    let mut hashes = BTreeSet::new();

    for definition in definitions {
        assert!(
            DomainPackDefinitionSpec.validate(&definition).is_ok(),
            "industrial descriptor {} must satisfy compatibility validation",
            definition.pack_id
        );
        let hash = definition.stable_descriptor_hash();
        assert!(
            !hash.is_empty(),
            "industrial descriptor {} must expose a stable descriptor hash",
            definition.pack_id
        );
        assert!(
            hashes.insert(hash),
            "industrial descriptor {} produced a duplicate descriptor hash",
            definition.pack_id
        );
    }
}

#[test]
fn unavailable_industrial_required_pack_blocks_expansion() {
    let catalog =
        compose_installed_domain_pack_catalog(industrial_reference_domain_pack_definitions());
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.office.pdf.v1".into()],
        optional_packs: vec!["pack.media.video.v1".into()],
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), catalog.as_ref());

    assert_eq!(expanded.resolved_packs, Vec::<String>::new());
    assert_eq!(
        expanded.unresolved_required_packs,
        vec!["pack.office.pdf.v1"]
    );
    assert_eq!(
        expanded.unresolved_optional_packs,
        vec!["pack.media.video.v1"]
    );
    assert!(expanded.services.is_empty());
    assert!(expanded
        .unavailable_pack_reasons
        .contains_key("pack.office.pdf.v1"));
}

#[test]
fn every_industrial_preview_pack_blocks_required_and_degrades_optional() {
    let definitions = industrial_reference_domain_pack_definitions();
    let catalog = compose_installed_domain_pack_catalog(definitions.clone());

    for definition in definitions {
        let required = expand_service_capabilities(
            Some(&AppServiceContractConfig {
                required_packs: vec![definition.pack_id.clone()],
                ..Default::default()
            }),
            catalog.as_ref(),
        );
        assert_eq!(required.resolved_packs, Vec::<String>::new());
        assert_eq!(
            required.unresolved_required_packs,
            vec![definition.pack_id.clone()]
        );
        assert!(required.unresolved_optional_packs.is_empty());
        assert!(required.services.is_empty());
        assert!(!required.capabilities_hash.is_empty());
        assert!(required
            .unavailable_pack_reasons
            .contains_key(&definition.pack_id));

        let optional = expand_service_capabilities(
            Some(&AppServiceContractConfig {
                optional_packs: vec![definition.pack_id.clone()],
                ..Default::default()
            }),
            catalog.as_ref(),
        );
        assert_eq!(optional.resolved_packs, Vec::<String>::new());
        assert!(optional.unresolved_required_packs.is_empty());
        assert_eq!(
            optional.unresolved_optional_packs,
            vec![definition.pack_id.clone()]
        );
        assert!(optional.services.is_empty());
        assert!(!optional.capabilities_hash.is_empty());
        assert!(optional
            .unavailable_pack_reasons
            .contains_key(&definition.pack_id));
    }
}
