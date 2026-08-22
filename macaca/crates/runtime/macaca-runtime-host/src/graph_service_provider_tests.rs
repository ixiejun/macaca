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
        let event = receive_event(&mut provider_events, &trace_id, command).await;
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

#[tokio::test]
async fn graph_emits_stable_audit_taxonomy() {
    let provider = GraphSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider.health().await.unwrap();
    for operation in [
        "graph.upsert_node",
        "graph.query",
        "graph.traverse",
        "graph.find_path",
        "graph.import_subgraph",
        "graph.merge_entities",
        "graph.inspect_provenance",
    ] {
        provider.call(command(operation, operation)).await.unwrap();
    }
    let mut names = Vec::new();
    while let Ok(event) = events.try_recv() {
        names.push(event.event_name);
    }
    for expected in [
        "graph.pack_declared",
        "graph.health",
        "graph.admission_validated",
        "graph.policy_decision",
        "graph.entitlement_checked",
        "graph.resource_reserved",
        "graph.approval_checked",
        "graph.service_call",
        "graph.mutation",
        "graph.query",
        "graph.traversal",
        "graph.path",
        "graph.import_export",
        "graph.merge",
        "graph.provenance",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}

#[tokio::test]
async fn graph_admission_denies_policy_facts_before_reference_allocation() {
    let provider = GraphSystemServiceProvider::mock();
    for (trace, payload) in [
        ("source", serde_json::json!({"source_denied": true})),
        ("schema", serde_json::json!({"schema_incompatible": true})),
        ("sensitive", serde_json::json!({"query_sensitive": true})),
        (
            "delete",
            serde_json::json!({"delete_approval_required": true}),
        ),
        ("depth", serde_json::json!({"max_depth": 6})),
        ("rows", serde_json::json!({"max_rows": 10_001})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("graph.query"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}

async fn receive_event(
    events: &mut tokio::sync::broadcast::Receiver<super::graph_service_provider::GraphRuntimeEvent>,
    trace_id: &str,
    command: &str,
) -> super::graph_service_provider::GraphRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.trace_id == trace_id && event.command == command {
            return event;
        }
    }
}
