//! Canonical runtime and redaction tests for the retrieval provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_RETRIEVAL_COMMANDS,
};

use super::retrieval_service_provider::RetrievalSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn retrieval_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(RetrievalSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("retrieval-start"))
        .await
        .unwrap();
    for command in KNOWLEDGE_RETRIEVAL_COMMANDS {
        let trace_id = format!("retrieval-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("retrieval-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"credential":"secret-marker", "vector":"raw-marker", "corpus":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
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
async fn retrieval_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = RetrievalSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("retrieval.retrieve", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = RetrievalSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("retrieval.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("retrieval.retrieve", "one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn retrieval_provider_emits_bounded_lifecycle_and_failure_events() {
    let provider = RetrievalSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider.health().await.unwrap();
    provider.snapshot().await;
    assert_eq!(
        events.try_recv().unwrap().kind,
        super::retrieval_service_provider::RetrievalRuntimeEventKind::PackDeclared
    );
    assert_eq!(
        events.try_recv().unwrap().kind,
        super::retrieval_service_provider::RetrievalRuntimeEventKind::HealthReported
    );
    assert_eq!(
        events.try_recv().unwrap().kind,
        super::retrieval_service_provider::RetrievalRuntimeEventKind::SnapshotRecorded
    );
    assert!(provider
        .call(command("retrieval.unsupported", "failure"))
        .await
        .is_err());
    assert_eq!(
        events.try_recv().unwrap().kind,
        super::retrieval_service_provider::RetrievalRuntimeEventKind::Failure
    );
}

#[test]
fn retrieval_capability_reports_provider_neutral_limits_and_features() {
    let capability = RetrievalSystemServiceProvider::mock().capability();
    assert!(capability.vector_features.contains("multivector"));
    assert!(capability.vector_features.contains("named_vector_spaces"));
    assert!(capability.namespace_features.contains("namespace"));
    assert!(capability.query_features.contains("range_search"));
    assert_eq!(capability.max_filters, 32);
    assert_eq!(capability.consistency_mode, "bounded_eventual");
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
