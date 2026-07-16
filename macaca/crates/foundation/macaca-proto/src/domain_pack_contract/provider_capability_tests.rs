use super::*;

#[test]
fn callable_pack_projects_callable_commands_provider_flags_and_replay_refs() {
    let mut definition = foundation_random_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    let catalog = compose_installed_domain_pack_catalog([definition.clone()]);
    let expanded = expand_service_capabilities(
        Some(&AppServiceContractConfig {
            required_packs: vec![definition.pack_id.clone()],
            ..Default::default()
        }),
        catalog.as_ref(),
    );

    let projection = expanded
        .capability_projections
        .iter()
        .find(|projection| projection.pack_id == definition.pack_id)
        .expect("callable pack should expose a capability projection");

    assert_eq!(
        projection.callable_commands,
        definition
            .metadata
            .service_command_schemas
            .get(&projection.service_id)
            .cloned()
            .unwrap()
    );
    assert!(projection.denied_commands.is_empty());
    assert!(projection.unavailable_commands.is_empty());
    assert!(projection.unavailable_features.is_empty());
    assert!(projection
        .provider_capability_flags
        .contains_key("host-csprng"));
    assert!(projection
        .replay_refs
        .contains(&definition.stable_descriptor_hash()));
}

#[test]
fn catalog_snapshot_hash_is_stable() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let first = snapshot_domain_pack_catalog(catalog.as_ref());
    let second = snapshot_domain_pack_catalog(catalog.as_ref());

    assert_eq!(first.catalog_hash, second.catalog_hash);
    assert_eq!(first.replay_schema, "pack.catalog.snapshot.v1");
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

#[test]
fn provider_capability_reports_use_bounded_normalized_states() {
    let states = [
        ("registered", DomainPackProviderCapabilityState::Available),
        ("available", DomainPackProviderCapabilityState::Available),
        ("healthy", DomainPackProviderCapabilityState::Available),
        ("degraded", DomainPackProviderCapabilityState::Degraded),
        ("preview", DomainPackProviderCapabilityState::Preview),
        (
            "unsupported",
            DomainPackProviderCapabilityState::Unsupported,
        ),
        ("retired", DomainPackProviderCapabilityState::Retired),
        (
            "provider-secret-leak-attempt",
            DomainPackProviderCapabilityState::Unavailable,
        ),
    ];

    for (health, expected_state) in states {
        let snapshot = DomainPackProviderSnapshot {
            pack_id: "pack.ai.llm.v1".into(),
            service_id: "service.ai.llm".into(),
            provider_class: "mock".into(),
            health: health.into(),
            unavailable_reason: None,
            trace_id: Some("trace-provider-capability".into()),
        };
        let report = snapshot.capability_report();

        assert_eq!(report.state, expected_state);
        assert_eq!(report.pack_id, "pack.ai.llm.v1");
        assert_eq!(report.service_id, "service.ai.llm");
        assert_eq!(report.provider_class, "mock");
        assert_eq!(
            report.trace_id.as_deref(),
            Some("trace-provider-capability")
        );
    }
}

#[test]
fn unavailable_provider_capability_report_preserves_sanitized_reason() {
    let snapshot = DomainPackProviderSnapshot::unavailable(
        "pack.workflow.review.v1",
        "service.workflow.review",
        "provider_not_installed",
    );
    let report = snapshot.capability_report();

    assert_eq!(report.state, DomainPackProviderCapabilityState::Unavailable);
    assert_eq!(report.reason_code, "provider_not_installed");
    assert_eq!(report.provider_class, "unavailable");
    assert!(report.trace_id.is_none());
}
