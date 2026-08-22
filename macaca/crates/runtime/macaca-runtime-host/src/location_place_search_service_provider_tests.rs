use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::location_place_search::LOCATION_PLACE_SEARCH_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::location_place_search_service_provider::{
    LocationPlaceSearchSystemServiceProvider, PlaceSearchRuntimeEventKind,
};

#[tokio::test]
async fn place_search_provider_dispatches_commands_without_payload_echo() {
    let provider = LocationPlaceSearchSystemServiceProvider::mock();
    for command in LOCATION_PLACE_SEARCH_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({
                    "query_text": "private-marker",
                    "coordinates": "exact-marker",
                    "session_token": "token-marker",
                }),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.status, "ok");
        assert!(!result.output.to_string().contains("marker"));
    }
}

#[tokio::test]
async fn place_search_provider_enforces_admission_before_retaining_reference() {
    let provider = LocationPlaceSearchSystemServiceProvider::mock();
    for payload in [
        serde_json::json!({"policy_denied": true}),
        serde_json::json!({"field_mask_missing": true}),
        serde_json::json!({"precise_location_denied": true}),
        serde_json::json!({"quota_exceeded": true}),
    ] {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new("place_search.get_details"),
                    payload,
                    TraceContext::new("place-denied"),
                ))
                .await,
            Err(ServiceError::DisabledByPolicy(_))
        ));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn place_search_provider_strategy_gap_and_unavailable_are_explicit() {
    let provider =
        LocationPlaceSearchSystemServiceProvider::mock_with_commands(["place_search.search"]);
    assert!(provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("place_search.search"),
            serde_json::json!({}),
            TraceContext::new("place-search"),
        ))
        .await
        .is_ok());
    assert!(matches!(
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("place_search.get_details"),
                serde_json::json!({}),
                TraceContext::new("place-gap"),
            ))
            .await,
        Err(ServiceError::UnsupportedCommand(code)) if code == "place_search_command_unsupported"
    ));
    let unavailable = LocationPlaceSearchSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
}

#[tokio::test]
async fn place_search_provider_emits_attribution_and_purge_replay_events() {
    let provider = LocationPlaceSearchSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in [
        "place_search.inspect_attribution",
        "place_search.purge_session",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    let mut saw_attribution = false;
    let mut saw_purge = false;
    while let Ok(event) = events.try_recv() {
        saw_attribution |= event.kind == PlaceSearchRuntimeEventKind::AttributionRecorded;
        saw_purge |= event.kind == PlaceSearchRuntimeEventKind::SessionPurged;
        assert!(event.replay_ref.starts_with("replay:place-search:"));
    }
    assert!(saw_attribution && saw_purge);
}
