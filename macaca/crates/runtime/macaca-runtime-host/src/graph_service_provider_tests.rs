//! Canonical runtime, replay, and redaction tests for the graph adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, KNOWLEDGE_GRAPH_COMMANDS,
};

use super::graph_service_provider::GraphSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn graph_commands_are_traceable_replayable_and_redacted_through_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(GraphSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("graph-start"))
        .await
        .unwrap();
    let mut provider_events = provider.subscribe();
    for command in KNOWLEDGE_GRAPH_COMMANDS {
        let trace_id = format!("graph-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("graph-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"credential":"secret-marker", "query":"raw-marker", "graph_value":"private-marker"}),
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
        let event = provider_events.try_recv().unwrap();
        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.command, *command);
        assert_eq!(event.replay_ref, format!("replay:{trace_id}"));
    }
    assert!(!format!("{:?}", events.events().unwrap()).contains("marker"));
}

#[tokio::test]
async fn graph_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = GraphSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("graph.query", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = GraphSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("graph.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider.call(command("graph.query", "one")).await.unwrap();
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
