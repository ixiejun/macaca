use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::location_timezone::LOCATION_TIMEZONE_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::location_timezone_service_provider::{
    LocationTimezoneRuntimeEventKind, LocationTimezoneSystemServiceProvider,
};

#[tokio::test]
async fn timezone_provider_dispatches_commands_with_dataset_evidence() {
    let provider = LocationTimezoneSystemServiceProvider::mock();
    for command in LOCATION_TIMEZONE_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({
                    "exact_coordinates": "coordinate-marker",
                    "database_path": "path-marker",
                    "provider_payload": "payload-marker",
                }),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.status, "ok");
        assert!(result.output["dataset_version"].as_str().is_some());
        assert!(!result.output.to_string().contains("marker"));
    }
}

#[tokio::test]
async fn timezone_provider_requires_explicit_local_resolution_strategy() {
    let provider = LocationTimezoneSystemServiceProvider::mock();
    for payload in [
        serde_json::json!({"resolver_strategy_missing": true}),
        serde_json::json!({"precise_coordinate_denied": true}),
        serde_json::json!({"stale_database": true}),
        serde_json::json!({"quota_exceeded": true}),
    ] {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new("timezone.resolve_local_time"),
                    payload,
                    TraceContext::new("timezone-denied"),
                ))
                .await,
            Err(ServiceError::DisabledByPolicy(_))
        ));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn timezone_provider_strategy_gap_and_unavailable_are_explicit() {
    let provider =
        LocationTimezoneSystemServiceProvider::mock_with_commands(["timezone.get_offset"]);
    assert!(provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("timezone.get_offset"),
            serde_json::json!({}),
            TraceContext::new("timezone-offset"),
        ))
        .await
        .is_ok());
    assert!(matches!(
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("timezone.list_transitions"),
                serde_json::json!({}),
                TraceContext::new("timezone-gap"),
            ))
            .await,
        Err(ServiceError::UnsupportedCommand(code)) if code == "timezone_command_unsupported"
    ));
    let unavailable = LocationTimezoneSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
}

#[tokio::test]
async fn timezone_provider_replay_events_include_dataset_version() {
    let provider = LocationTimezoneSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("timezone.inspect_database"),
            serde_json::json!({}),
            TraceContext::new("timezone-replay"),
        ))
        .await
        .unwrap();
    let mut saw_success = false;
    while let Ok(event) = events.try_recv() {
        saw_success |= event.kind == LocationTimezoneRuntimeEventKind::CommandSucceeded;
        assert_eq!(event.dataset_version, "synthetic-2026a");
        assert!(event.replay_ref.starts_with("replay:timezone:"));
    }
    assert!(saw_success);
}
