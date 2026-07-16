//! Canonical runtime and redaction tests for the messaging provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, COMMUNICATION_MESSAGING_COMMANDS,
};

use super::messaging_service_provider::MessagingSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn messaging_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(MessagingSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("messaging-start"))
        .await
        .unwrap();
    for command in COMMUNICATION_MESSAGING_COMMANDS {
        let trace_id = format!("messaging-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("messaging-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"token":"secret-marker", "message_body":"raw-marker", "attachment":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
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
async fn messaging_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = MessagingSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("messaging.send_message", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = MessagingSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("messaging.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("messaging.send_message", "one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await.active_conversation_count, 1);
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await.active_conversation_count, 0);
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_MESSAGING_COMMANDS.len()
    );
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
