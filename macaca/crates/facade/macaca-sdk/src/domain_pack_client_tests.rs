use macaca_proto::{
    compose_installed_domain_pack_catalog, foundation_pack_definition,
    reference_domain_pack_definitions, TraceContext, FOUNDATION_CONFIG_PACK_ID,
    FOUNDATION_CONFIG_SERVICE_ID, FOUNDATION_FILESYSTEM_PACK_ID, FOUNDATION_FILESYSTEM_SERVICE_ID,
    FOUNDATION_KEY_VALUE_STATE_PACK_ID, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
    FOUNDATION_RANDOM_PACK_ID, FOUNDATION_RANDOM_SERVICE_ID, FOUNDATION_SECRETS_REFERENCE_PACK_ID,
    FOUNDATION_SECRETS_REFERENCE_SERVICE_ID, FOUNDATION_SESSION_STATE_PACK_ID,
    FOUNDATION_SESSION_STATE_SERVICE_ID, FOUNDATION_TIME_PACK_ID, FOUNDATION_TIME_SERVICE_ID,
};

use super::*;

// These tests live beside domain_pack_client.rs so the production SDK Facade
// remains below the repository's source-size ceiling. They still compile as a
// child module of domain_pack_client and can validate private SDK helper paths
// without widening the public API.

#[tokio::test]
async fn catalog_client_lists_reference_pack_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let result = client
        .list_packs(&DomainPackListCommand {
            scope: "sdk-test".into(),
        })
        .await
        .unwrap();

    assert!(result
        .packs
        .iter()
        .any(|pack| pack.pack_id == "pack.foundation.v1"));
}

#[tokio::test]
async fn catalog_client_resolves_required_and_optional_unavailable() {
    let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.foundation.v1".into(), "pack.absent.v1".into()],
        optional_packs: vec!["pack.optional.v1".into()],
        ..Default::default()
    };

    let result = client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .unwrap();

    assert!(result
        .effective
        .resolved_packs
        .contains(&"pack.foundation.v1".to_string()));
    assert!(result
        .unavailable
        .iter()
        .any(|diagnostic| { diagnostic.pack_id == "pack.absent.v1" && diagnostic.required }));
    assert!(result
        .unavailable
        .iter()
        .any(|diagnostic| { diagnostic.pack_id == "pack.optional.v1" && !diagnostic.required }));
}

#[tokio::test]
async fn resolved_pack_service_invocation_stays_trace_addressable() {
    let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.foundation.v1".into()],
        ..Default::default()
    };

    let result = client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .unwrap();
    let service_id = result
        .effective
        .services
        .iter()
        .next()
        .expect("foundation pack exposes at least one service")
        .clone();
    let command = result
        .service_call_command(
            service_id.clone(),
            result
                .effective
                .service_command_schemas
                .get(&service_id)
                .and_then(|commands| commands.iter().next())
                .expect("foundation service exposes command schema")
                .clone(),
            serde_json::json!({}),
            TraceContext::new("trace-pack-service-call"),
        )
        .unwrap();

    assert_eq!(command.service_id, service_id);
    assert_eq!(
        command.trace.as_ref().map(|trace| trace.trace_id.as_str()),
        Some("trace-pack-service-call")
    );
}

#[tokio::test]
async fn catalog_client_explains_unavailable_specialized_office_pack() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new("pack.office.pdf.v1").expect("valid industrial pack id"),
        )
        .await
        .unwrap();

    let pack = inspect.pack.expect("industrial descriptor is discoverable");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "office_pdf_provider_not_installed"
    );
    assert!(pack
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/office/pdf"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_random_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_RANDOM_PACK_ID)
                .expect("valid foundation random pack id"),
        )
        .await
        .unwrap();

    let pack = inspect.pack.expect("foundation random descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "random_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_RANDOM_SERVICE_ID)
        .is_some_and(|commands| commands.contains("random.bytes")
            && commands.contains("random.provider_capabilities")));
    assert!(pack
        .metadata
        .provider_descriptors
        .contains_key("deterministic-test"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_filesystem_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_FILESYSTEM_PACK_ID)
                .expect("valid foundation filesystem pack id"),
        )
        .await
        .unwrap();

    let pack = inspect
        .pack
        .expect("foundation filesystem descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "filesystem_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_FILESYSTEM_SERVICE_ID)
        .is_some_and(|commands| commands.contains("filesystem.read_file")
            && commands.contains("filesystem.snapshot_tree")));
    assert!(pack
        .metadata
        .provider_descriptors
        .contains_key("local-scoped-workspace"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_key_value_state_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_KEY_VALUE_STATE_PACK_ID)
                .expect("valid foundation key-value state pack id"),
        )
        .await
        .unwrap();

    let pack = inspect
        .pack
        .expect("foundation key-value state descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "key_value_state_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID)
        .is_some_and(
            |commands| commands.contains("kv.get") && commands.contains("kv.compact_namespace")
        ));
    assert!(pack
        .metadata
        .provider_descriptors
        .contains_key("embedded-durable"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_config_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_CONFIG_PACK_ID)
                .expect("valid foundation config pack id"),
        )
        .await
        .unwrap();

    let pack = inspect.pack.expect("foundation config descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "config_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_CONFIG_SERVICE_ID)
        .is_some_and(|commands| commands.contains("config.get")
            && commands.contains("config.export_redacted")));
    assert!(pack
        .metadata
        .provider_descriptors
        .contains_key("environment"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_secrets_reference_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_SECRETS_REFERENCE_PACK_ID)
                .expect("valid foundation secrets-reference pack id"),
        )
        .await
        .unwrap();

    let pack = inspect
        .pack
        .expect("foundation secrets-reference descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "secrets_reference_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_SECRETS_REFERENCE_SERVICE_ID)
        .is_some_and(|commands| commands.contains("secrets.inspect_reference")
            && commands.contains("secrets.audit_access")));
    assert!(pack.metadata.provider_descriptors.contains_key("vault"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_time_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_TIME_PACK_ID)
                .expect("valid foundation time pack id"),
        )
        .await
        .unwrap();

    let pack = inspect.pack.expect("foundation time descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "time_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_TIME_SERVICE_ID)
        .is_some_and(
            |commands| commands.contains("time.now") && commands.contains("time.clock_health")
        ));
    assert!(pack
        .metadata
        .provider_descriptors
        .contains_key("frozen-test-clock"));
}

#[tokio::test]
async fn catalog_client_discovers_foundation_session_state_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let inspect = client
        .inspect_pack(
            &DomainPackInspectCommand::new(FOUNDATION_SESSION_STATE_PACK_ID)
                .expect("valid foundation session-state pack id"),
        )
        .await
        .unwrap();

    let pack = inspect
        .pack
        .expect("foundation session-state descriptor exists");
    assert!(!pack.is_callable());
    assert_eq!(
        pack.metadata.diagnostics.unavailable_reason,
        "session_state_provider_not_installed"
    );
    assert!(pack
        .metadata
        .service_command_schemas
        .get(FOUNDATION_SESSION_STATE_SERVICE_ID)
        .is_some_and(|commands| commands.contains("session_state.get")
            && commands.contains("session_state.inspect_recovery")));
    assert!(pack.metadata.provider_descriptors.contains_key("embedded"));
    assert!(pack
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/session-state"));
}

#[tokio::test]
async fn undeclared_pack_command_is_rejected_before_service_call() {
    let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.foundation.v1".into()],
        ..Default::default()
    };
    let result = client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .unwrap();
    let service_id = result
        .effective
        .services
        .iter()
        .next()
        .expect("foundation pack exposes at least one service")
        .clone();

    let rejected = result.service_call_command(
        service_id,
        "undeclared.command.v1",
        serde_json::json!({}),
        TraceContext::new("trace-pack-command-rejected"),
    );

    assert!(rejected.is_err());
}

#[tokio::test]
async fn domain_pack_service_call_builder_only_builds_declared_traced_commands() {
    let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec!["pack.foundation.v1".into()],
        ..Default::default()
    };
    let result = client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .unwrap();
    let service_id = result
        .effective
        .services
        .iter()
        .next()
        .expect("foundation pack exposes at least one service")
        .clone();
    let declared_command = result
        .effective
        .service_command_schemas
        .get(&service_id)
        .and_then(|commands| commands.iter().next())
        .expect("foundation service exposes command schema")
        .clone();

    let command = DomainPackServiceCallBuilder::new(
        service_id.clone(),
        declared_command.clone(),
        serde_json::json!({"bounded": true}),
        TraceContext::new("trace-pack-builder"),
    )
    .unwrap()
    .build(&result)
    .unwrap();

    assert_eq!(command.service_id, service_id);
    assert_eq!(command.command_name, declared_command);
    assert_eq!(
        command.trace.as_ref().map(|trace| trace.trace_id.as_str()),
        Some("trace-pack-builder")
    );

    let rejected = DomainPackServiceCallBuilder::new(
        command.service_id,
        "undeclared.command.v1",
        serde_json::json!({}),
        TraceContext::new("trace-pack-builder-rejected"),
    )
    .unwrap()
    .build(&result);
    assert!(rejected.is_err());
}
