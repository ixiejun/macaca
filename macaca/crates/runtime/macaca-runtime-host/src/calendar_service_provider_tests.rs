//! Canonical runtime and redaction tests for the calendar provider adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext, COMMUNICATION_CALENDAR_COMMANDS,
};

use super::calendar_service_provider::CalendarSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn calendar_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(CalendarSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("calendar-start"))
        .await
        .unwrap();
    for command in COMMUNICATION_CALENDAR_COMMANDS {
        let trace_id = format!("calendar-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("calendar-conformance"), ServiceCommand::with_trace(
            ServiceCommandName::new(*command), serde_json::json!({"credential":"secret-marker", "export":"raw-marker", "invite":"private-marker"}), TraceContext::new(trace_id.clone()),
        )).await.unwrap();
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
async fn calendar_provider_fails_closed_and_cleans_bounded_snapshot_state() {
    let unavailable = CalendarSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("calendar.list_calendars", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = CalendarSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("calendar.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("calendar.register_watch", "watch-one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await.watch_count, 1);
    provider.cleanup().await.unwrap();
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot.source_count, 0);
    assert_eq!(snapshot.watch_count, 0);
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_CALENDAR_COMMANDS.len()
    );
}

#[test]
fn calendar_mock_capability_reports_only_bounded_generic_facts() {
    let capability = CalendarSystemServiceProvider::mock().capability();
    assert!(capability.supports_event_crud);
    assert!(capability.supports_recurrence);
    assert!(capability.supports_attendees);
    assert!(capability.supports_reminders_and_conference);
    assert_eq!(capability.max_recurrence_expansion, 128);
    assert_eq!(capability.page_limit, 100);
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
