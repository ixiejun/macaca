//! Canonical runtime and redaction tests for the search provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_SEARCH_COMMANDS,
};

use super::search_service_provider::SearchSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn search_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(SearchSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider.clone(),
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("search-start"))
        .await
        .unwrap();
    let mut provider_events = provider.subscribe();
    for command in KNOWLEDGE_SEARCH_COMMANDS {
        let trace_id = format!("search-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("search-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"credential":"secret-marker", "query":"raw-marker", "document":"private-marker"}),
                    TraceContext::new(trace_id.clone()),
                ),
            )
            .await
            .unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events.events().unwrap().iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"
        }));
        let mut replayable = false;
        while let Ok(event) = provider_events.try_recv() {
            replayable |= event.trace_id == trace_id
                && event.command == *command
                && event.replay_ref == format!("replay:{trace_id}");
        }
        assert!(replayable, "search command must emit a replay reference");
    }
    let observable = format!("{:?}", events.events().unwrap());
    for marker in ["secret-marker", "raw-marker", "private-marker"] {
        assert!(!observable.contains(marker));
    }
}

#[tokio::test]
async fn search_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = SearchSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("search.search", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = SearchSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("search.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("search.search", "one"))
        .await
        .unwrap();
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot["active_reference_count"], "1");
    let observable = format!("{snapshot:?}");
    for marker in [
        "secret-marker",
        "raw-marker",
        "private-marker",
        "raw-query-token",
    ] {
        assert!(!observable.contains(marker));
    }
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn search_provider_projects_bounded_cursor_and_refresh_handles() {
    let provider = SearchSystemServiceProvider::mock();
    for (command, state, field) in [
        ("search.search", "paged", "next_cursor_ref"),
        ("search.refresh_index", "async", "async_handle_ref"),
    ] {
        let reply = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"result_state":state}),
                TraceContext::new(format!("search-{state}")),
            ))
            .await
            .unwrap();
        assert_eq!(reply.status, state);
        assert!(!reply.output[field].is_null());
    }
}

#[test]
fn search_mock_capability_reports_only_provider_neutral_bounded_features() {
    let capability = SearchSystemServiceProvider::mock().capability();
    for feature in [
        "query_ast",
        "filters",
        "facets",
        "sort",
        "suggest",
        "autocomplete",
        "explain",
        "refresh",
    ] {
        assert!(capability.query_features.contains(feature));
    }
    assert_eq!(capability.max_page_size, 100);
    assert_eq!(capability.max_explain_depth, 3);
    assert!(capability.supports_refresh);
    assert!(capability.supports_health);
    assert!(!capability.rate_limit_bucket.contains("credential"));
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
