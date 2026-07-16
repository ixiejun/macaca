use macaca_proto::{
    compose_installed_domain_pack_catalog, foundation_config_pack_definition,
    foundation_filesystem_pack_definition, foundation_key_value_state_pack_definition,
    foundation_random_pack_definition, foundation_secrets_reference_pack_definition,
    foundation_session_state_pack_definition, foundation_time_pack_definition,
    industrial_reference_domain_pack_definitions, AppServiceContractConfig, DomainPackAvailability,
    DomainPackDefinition, TraceContext, FOUNDATION_CONFIG_COMMANDS, FOUNDATION_CONFIG_SERVICE_ID,
    FOUNDATION_FILESYSTEM_COMMANDS, FOUNDATION_FILESYSTEM_SERVICE_ID,
    FOUNDATION_KEY_VALUE_STATE_COMMANDS, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
    FOUNDATION_RANDOM_COMMANDS, FOUNDATION_RANDOM_SERVICE_ID,
    FOUNDATION_SECRETS_REFERENCE_COMMANDS, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
    FOUNDATION_SESSION_STATE_COMMANDS, FOUNDATION_SESSION_STATE_SERVICE_ID,
    FOUNDATION_TIME_COMMANDS, FOUNDATION_TIME_SERVICE_ID,
};

use crate::domain_pack_client::{
    CatalogBackedDomainPackClient, DomainPackResolveCommand, SystemDomainPackClient,
};

use super::*;

// The fixture keeps every foundation descriptor provider-neutral and simply
// flips availability so SDK command construction can be tested without runtime
// provider registration or any host side effect.
#[tokio::test]
async fn foundation_command_builders_cover_every_declared_command() {
    for (definition, service_id, commands) in callable_foundation_definitions() {
        let resolved = resolve_callable(definition.clone()).await;
        let catalog = DomainPackCommandCatalogBuilder::from_pack_definition(&definition).unwrap();

        assert_eq!(catalog.pack_id(), definition.pack_id);
        assert_eq!(catalog.command_specs().len(), commands.len());

        for command_name in commands {
            let command = catalog
                .command_builder(
                    *command_name,
                    serde_json::json!({"fixture_payload_ref": "redacted"}),
                    TraceContext::new(format!("trace-{}", command_name.replace('.', "-"))),
                )
                .unwrap()
                .build(&resolved)
                .unwrap();

            assert_eq!(command.service_id, service_id);
            assert_eq!(command.command_name, *command_name);
            assert!(command.trace.is_some());
        }
    }
}

#[tokio::test]
async fn industrial_catalog_command_builders_cover_every_sub_pack_descriptor() {
    let mut pack_count = 0usize;
    let mut command_count = 0usize;

    for definition in industrial_reference_domain_pack_definitions() {
        let resolved = resolve_callable(definition.clone()).await;
        let catalog = DomainPackCommandCatalogBuilder::from_pack_definition(&definition).unwrap();

        pack_count += 1;
        command_count += catalog.command_specs().len();

        for spec in catalog.command_specs() {
            let command = catalog
                .command_builder(
                    spec.command_name.as_str(),
                    serde_json::json!({"fixture_payload_ref": "redacted"}),
                    TraceContext::new(format!(
                        "trace-{}",
                        spec.command_name.replace(['.', '_'], "-")
                    )),
                )
                .unwrap()
                .build(&resolved)
                .unwrap();

            assert_eq!(command.service_id, spec.service_id);
            assert_eq!(command.command_name, spec.command_name);
            assert!(command.trace.is_some());
        }
    }

    assert_eq!(pack_count, 74);
    assert!(command_count > pack_count);
}

#[tokio::test]
async fn preview_unavailable_pack_catalog_does_not_become_callable() {
    let definition = foundation_random_pack_definition();
    let catalog = DomainPackCommandCatalogBuilder::from_pack_definition(&definition).unwrap();
    let resolved = resolve_preview_unavailable(definition).await;

    let rejected = catalog
        .command_builder(
            "random.bytes",
            serde_json::json!({"length": 16}),
            TraceContext::new("trace-preview-random"),
        )
        .unwrap()
        .build(&resolved);

    assert!(rejected.is_err());
}

#[test]
fn undeclared_descriptor_command_is_rejected_before_service_call() {
    let definition = callable(foundation_time_pack_definition());
    let catalog = DomainPackCommandCatalogBuilder::from_pack_definition(&definition).unwrap();

    let rejected = catalog.command_builder(
        "time.provider_native_handle",
        serde_json::json!({}),
        TraceContext::new("trace-undeclared-time"),
    );

    assert!(rejected.is_err());
}

async fn resolve_callable(definition: DomainPackDefinition) -> DomainPackResolveResult {
    resolve_with_definition(callable(definition)).await
}

async fn resolve_preview_unavailable(definition: DomainPackDefinition) -> DomainPackResolveResult {
    resolve_with_definition(definition).await
}

async fn resolve_with_definition(definition: DomainPackDefinition) -> DomainPackResolveResult {
    let pack_id = definition.pack_id.clone();
    let catalog = compose_installed_domain_pack_catalog([definition]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec![pack_id],
        ..Default::default()
    };

    client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .expect("domain-pack descriptor resolves")
}

fn callable(mut definition: DomainPackDefinition) -> DomainPackDefinition {
    definition.metadata.availability = DomainPackAvailability::Available;
    definition
}

fn callable_foundation_definitions(
) -> Vec<(DomainPackDefinition, &'static str, &'static [&'static str])> {
    vec![
        (
            callable(foundation_filesystem_pack_definition()),
            FOUNDATION_FILESYSTEM_SERVICE_ID,
            FOUNDATION_FILESYSTEM_COMMANDS,
        ),
        (
            callable(foundation_key_value_state_pack_definition()),
            FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            FOUNDATION_KEY_VALUE_STATE_COMMANDS,
        ),
        (
            callable(foundation_config_pack_definition()),
            FOUNDATION_CONFIG_SERVICE_ID,
            FOUNDATION_CONFIG_COMMANDS,
        ),
        (
            callable(foundation_secrets_reference_pack_definition()),
            FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
            FOUNDATION_SECRETS_REFERENCE_COMMANDS,
        ),
        (
            callable(foundation_session_state_pack_definition()),
            FOUNDATION_SESSION_STATE_SERVICE_ID,
            FOUNDATION_SESSION_STATE_COMMANDS,
        ),
        (
            callable(foundation_time_pack_definition()),
            FOUNDATION_TIME_SERVICE_ID,
            FOUNDATION_TIME_COMMANDS,
        ),
        (
            callable(foundation_random_pack_definition()),
            FOUNDATION_RANDOM_SERVICE_ID,
            FOUNDATION_RANDOM_COMMANDS,
        ),
    ]
}
