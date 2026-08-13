//! Canonical runtime and redaction tests for the email provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, COMMUNICATION_EMAIL_COMMANDS,
};

use super::email_service_provider::{EmailRuntimeEventKind, EmailSystemServiceProvider};
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn email_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(EmailSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("email-start"))
        .await
        .unwrap();
    for command in COMMUNICATION_EMAIL_COMMANDS {
        let trace_id = format!("email-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("email-conformance"), ServiceCommand::with_trace(ServiceCommandName::new(*command), serde_json::json!({"oauth_token":"secret-marker", "body":"raw-marker", "attachment":"private-marker"}), TraceContext::new(trace_id.clone()))).await.unwrap();
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
async fn email_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = EmailSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable.call(command("email.send", "unavailable")).await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = EmailSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    assert!(matches!(
        provider
            .call(command("email.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    assert!(matches!(
        events.recv().await.unwrap().kind,
        EmailRuntimeEventKind::ProviderCallFailed
    ));
    provider.call(command("email.send", "one")).await.unwrap();
    assert_eq!(provider.snapshot().await.sender_identity_count, 1);
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await.sender_identity_count, 0);
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_EMAIL_COMMANDS.len()
    );
    let capability = provider.capability();
    assert!(capability.supports_attachment_handles);
    assert!(capability.supports_sync_cursors);
    assert!(capability.supports_health);
    assert_eq!(capability.rate_limit_bucket, "runtime_host_default");
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
