//! Tests proving foundation config SDK helpers use only the declared service boundary.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    TraceContext, FOUNDATION_CONFIG_PACK_ID, FOUNDATION_CONFIG_SERVICE_ID,
};

use super::{
    config_describe_schema_command, config_effective_command, config_export_redacted_command,
    config_get_command, config_get_many_command, config_list_keys_command,
    config_provenance_command, config_reload_command, config_snapshot_command,
    config_unavailable_diagnostics_command, config_validate_command, config_watch_command,
    ConfigDomainPackCommandBuildOutcome,
};
use crate::domain_pack_client::SystemDomainPackClient;
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> crate::DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_config_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_CONFIG_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn config_helpers_build_only_canonical_traced_service_calls() {
    let helpers = vec![
        (
            "config.describe_schema",
            config_describe_schema_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-schema"),
            ),
        ),
        (
            "config.get",
            config_get_command(serde_json::json!({}), TraceContext::new("trace-config-get")),
        ),
        (
            "config.get_many",
            config_get_many_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-many"),
            ),
        ),
        (
            "config.list_keys",
            config_list_keys_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-list"),
            ),
        ),
        (
            "config.resolve_effective",
            config_effective_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-effective"),
            ),
        ),
        (
            "config.reload",
            config_reload_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-reload"),
            ),
        ),
        (
            "config.snapshot",
            config_snapshot_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-snapshot"),
            ),
        ),
        (
            "config.validate",
            config_validate_command(
                serde_json::json!({"candidate_ref":"artifact:candidate"}),
                TraceContext::new("trace-config-validate"),
            ),
        ),
        (
            "config.explain_provenance",
            config_provenance_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-provenance"),
            ),
        ),
        (
            "config.watch",
            config_watch_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-watch"),
            ),
        ),
        (
            "config.export_redacted",
            config_export_redacted_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-export"),
            ),
        ),
        (
            "config.describe_schema",
            config_unavailable_diagnostics_command(
                serde_json::json!({}),
                TraceContext::new("trace-config-diagnostics"),
            ),
        ),
    ];
    for (name, helper) in helpers {
        let command = helper.build(&resolved().await).unwrap();
        assert_eq!(command.service_id, FOUNDATION_CONFIG_SERVICE_ID);
        assert_eq!(command.command_name, name);
        assert!(command.trace.is_some());
    }
}

#[tokio::test]
async fn rejected_config_preflight_cannot_create_a_service_call() {
    let outcome = config_reload_command(
        serde_json::json!({"source":"remote"}),
        TraceContext::new("trace-config-preflight-rejected"),
    )
    .build_after_preflight(
        &resolved().await,
        Err(macaca_proto::ConfigAdmissionFailure::ApprovalRequired),
    )
    .unwrap();
    assert_eq!(
        outcome,
        ConfigDomainPackCommandBuildOutcome::Rejected(
            macaca_proto::ConfigAdmissionFailure::ApprovalRequired
        )
    );
}
