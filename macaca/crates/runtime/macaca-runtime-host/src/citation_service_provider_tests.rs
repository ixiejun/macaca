//! Canonical runtime and redaction tests for the citation provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_CITATIONS_COMMANDS,
};

use super::citation_service_provider::CitationSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn citation_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(CitationSystemServiceProvider::mock());
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("citations-start"))
        .await
        .unwrap();
    for command in KNOWLEDGE_CITATIONS_COMMANDS {
        let trace_id = format!("citations-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("citations-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"credential":"secret-marker", "quote":"raw-marker", "source_document":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events
            .events()
            .unwrap()
            .iter()
            .any(|event| event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"));
    }
    let observable = format!("{:?}", events.events().unwrap());
    for marker in ["secret-marker", "raw-marker", "private-marker"] {
        assert!(!observable.contains(marker));
    }
}

#[tokio::test]
async fn citation_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = CitationSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("citations.resolve_identifier", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = CitationSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("citations.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("citations.create_citation", "one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
