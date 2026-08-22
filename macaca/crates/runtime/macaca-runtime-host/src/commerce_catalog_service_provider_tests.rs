use super::commerce_catalog_service_provider::{
    CommerceCatalogRuntimeEventKind, CommerceCatalogSystemServiceProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_catalog::COMMERCE_CATALOG_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, TraceContext};
use serde_json::json;

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn catalog_mock_dispatches_commands_and_redacts_payloads() {
    let provider = CommerceCatalogSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let result = provider
        .call(command(
            "catalog.search_catalog",
            json!({"query":"secret-provider-dsl"}),
        ))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("catalog:reference"));
    assert!(!result.output.to_string().contains("secret-provider-dsl"));
    let event = events.recv().await.unwrap();
    assert!(!event.command.contains("secret"));
}

#[tokio::test]
async fn catalog_preconditions_fail_before_reference_retention() {
    let provider = CommerceCatalogSystemServiceProvider::mock();
    let denied = provider
        .call(command(
            "catalog.product_request",
            json!({"approval_required":true}),
        ))
        .await;
    assert!(matches!(denied, Err(ServiceError::DisabledByPolicy(_))));
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn catalog_unavailable_is_explicit_for_every_command() {
    let provider = CommerceCatalogSystemServiceProvider::unavailable("provider_not_installed");
    for name in COMMERCE_CATALOG_COMMANDS {
        assert!(matches!(
            provider.call(command(name, json!({}))).await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
}

#[tokio::test]
async fn catalog_snapshot_and_events_are_replay_safe() {
    let provider =
        CommerceCatalogSystemServiceProvider::mock_with_commands(["catalog.inspect_provider"]);
    let result = provider
        .call(command("catalog.inspect_provider", json!({})))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("dataset_version"));
    let snapshot = provider.snapshot().await;
    assert_eq!(
        snapshot["redaction_profile"],
        "references_hashes_and_dataset_metadata_only"
    );
    assert_ne!(
        CommerceCatalogRuntimeEventKind::SnapshotRecorded,
        CommerceCatalogRuntimeEventKind::ServiceCall
    );
}
