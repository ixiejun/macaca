//! Canonical runtime and redaction tests for the inbox provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, COMMUNICATION_INBOX_COMMANDS,
};

use super::inbox_service_provider::InboxSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn inbox_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(InboxSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("inbox-start"))
        .await
        .unwrap();
    for command in COMMUNICATION_INBOX_COMMANDS {
        let trace_id = format!("inbox-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("inbox-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"credential":"secret-marker", "body":"raw-marker", "attachment":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
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
async fn inbox_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = InboxSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("inbox.list_items", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = InboxSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("inbox.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("inbox.sync_sources", "sync-one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await.item_count, 1);
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await.item_count, 0);
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_INBOX_COMMANDS.len()
    );
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
