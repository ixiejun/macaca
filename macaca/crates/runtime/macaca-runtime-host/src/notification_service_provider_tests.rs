use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    NotificationDeliveryStatus, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError,
    ServiceHealth, TraceContext, COMMUNICATION_NOTIFICATION_COMMANDS,
};

use super::notification_service_provider::{
    NotificationRuntimeEventKind, NotificationSystemServiceProvider,
};
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn notification_provider_is_deterministic_for_every_descriptor_command() {
    let provider = NotificationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in COMMUNICATION_NOTIFICATION_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_token": "must-not-leak"}),
                TraceContext::new(format!("trace-{command}")),
            ))
            .await
            .unwrap();
        let event = receive_kind(&mut events, event_kind_for_command(command)).await;
        assert_eq!(event.command, *command);
        assert!(!result.output.to_string().contains("must-not-leak"));
        assert!(!format!("{:?}", event).contains("must-not-leak"));
    }
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_NOTIFICATION_COMMANDS.len()
    );
}

#[tokio::test]
async fn notification_provider_reports_unavailable_and_unsupported_without_events() {
    let provider = NotificationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let error = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("notification.unknown"),
            serde_json::json!({}),
            TraceContext::new("trace-unknown"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceError::UnsupportedCommand(_)));
    assert!(events.try_recv().is_err());
    let unavailable = NotificationSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    let error = unavailable
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("notification.publish"),
            serde_json::json!({}),
            TraceContext::new("trace-unavailable"),
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
}

#[tokio::test]
async fn notification_provider_snapshot_and_handles_stay_bounded_and_cleanup_releases_state() {
    let provider = NotificationSystemServiceProvider::mock();
    for trace_id in ["trace-one", "trace-two"] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("notification.publish"),
                serde_json::json!({"unbounded": "x".repeat(20_000)}),
                TraceContext::new(trace_id),
            ))
            .await
            .unwrap();
        assert_eq!(
            result.output["delivery_handle_ref"],
            format!("delivery:{trace_id}")
        );
        assert_eq!(result.output["retry_metadata"], "bounded:provider-owned");
        assert!(result.output.to_string().len() < 512);
    }
    assert_eq!(provider.snapshot().await.active_delivery_count, 2);
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await.active_delivery_count, 0);
}

#[tokio::test]
async fn notification_provider_reuses_idempotent_delivery_handles_and_bounds_pages() {
    let provider = NotificationSystemServiceProvider::mock();
    for trace_id in ["trace-first", "trace-duplicate", "trace-third"] {
        let mut command = ServiceCommand::with_trace(
            ServiceCommandName::new("notification.publish"),
            serde_json::json!({}),
            TraceContext::new(trace_id),
        );
        command.metadata.insert(
            "idempotency_key".into(),
            if trace_id == "trace-third" {
                "other"
            } else {
                "same"
            }
            .into(),
        );
        provider.call(command).await.unwrap();
    }
    assert_eq!(provider.snapshot().await.active_delivery_count, 2);
    assert_eq!(provider.delivery_page(1).await.len(), 1);
    assert_eq!(provider.delivery_page(10_000).await.len(), 2);
}

#[tokio::test]
async fn notification_provider_redacts_sensitive_payloads_and_replays_after_restart() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    }));
    let provider = Arc::new(NotificationSystemServiceProvider::mock());
    let mut observer = provider.subscribe();
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-notification-start"))
        .await
        .unwrap();
    let markers = [
        "token-marker",
        "endpoint-marker",
        "key-marker",
        "credential-marker",
        "provider-marker",
        "private-marker",
        "unbounded-marker",
    ];
    dispatch_notification(&runtime, &service_id, "notification.publish", "trace-notification-publish", serde_json::json!({"token": markers[0], "endpoint": markers[1], "key": markers[2], "credential": markers[3], "provider_payload": markers[4], "private": markers[5], "content": markers[6].repeat(1000)})).await;
    let first = receive_kind(
        &mut observer,
        NotificationRuntimeEventKind::DeliveryStatusChanged,
    )
    .await;
    assert_eq!(first.command, "notification.publish");
    assert_eq!(
        first.kind,
        NotificationRuntimeEventKind::DeliveryStatusChanged
    );
    runtime
        .stop(&service_id, TraceContext::new("trace-notification-stop"))
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-notification-restart"))
        .await
        .unwrap();
    dispatch_notification(
        &runtime,
        &service_id,
        "notification.register_subscription",
        "trace-notification-subscription",
        serde_json::json!({}),
    )
    .await;
    let second = receive_kind(
        &mut observer,
        NotificationRuntimeEventKind::SubscriptionChanged,
    )
    .await;
    assert_eq!(second.command, "notification.register_subscription");
    assert_eq!(
        second.kind,
        NotificationRuntimeEventKind::SubscriptionChanged
    );
    let observable = format!("{:?}{:?}", events.events().unwrap(), [first, second]);
    for marker in markers {
        assert!(!observable.contains(marker));
    }
    for trace_id in [
        "trace-notification-publish",
        "trace-notification-subscription",
    ] {
        assert!(events
            .events()
            .unwrap()
            .iter()
            .any(|event| event.trace_id.as_deref() == Some(trace_id)
                && event.operation == "service_runtime.call.completed"));
    }
}

#[tokio::test]
async fn notification_action_callback_is_trace_addressable() {
    let provider = NotificationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("notification.register_action"),
            serde_json::json!({}),
            TraceContext::new("trace-notification-action"),
        ))
        .await
        .unwrap();
    let event = receive_kind(&mut events, NotificationRuntimeEventKind::ActionReceived).await;
    assert_eq!(event.kind, NotificationRuntimeEventKind::ActionReceived);
    assert_eq!(event.trace_id, "trace-notification-action");
}

#[tokio::test]
async fn notification_provider_emits_complete_sanitized_audit_taxonomy() {
    let provider = NotificationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("notification.publish"),
            serde_json::json!({}),
            TraceContext::new("trace-notification-audit"),
        ))
        .await
        .unwrap();
    provider.health().await.unwrap();
    provider.snapshot().await;
    let unavailable = NotificationSystemServiceProvider::unavailable("not-installed");
    let mut unavailable_events = unavailable.subscribe();
    let _ = unavailable
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("notification.publish"),
            serde_json::json!({}),
            TraceContext::new("trace-notification-unavailable"),
        ))
        .await;
    assert_eq!(
        receive_kind(
            &mut unavailable_events,
            NotificationRuntimeEventKind::Unavailable
        )
        .await
        .kind,
        NotificationRuntimeEventKind::Unavailable
    );
    let mut emitted = Vec::new();
    for _ in 0..13 {
        emitted.push(events.recv().await.unwrap().kind);
    }
    for kind in [
        NotificationRuntimeEventKind::PackDeclared,
        NotificationRuntimeEventKind::AdmissionValidated,
        NotificationRuntimeEventKind::ConsentChecked,
        NotificationRuntimeEventKind::PolicyDecision,
        NotificationRuntimeEventKind::ResourceReserved,
        NotificationRuntimeEventKind::EntitlementChecked,
        NotificationRuntimeEventKind::ApprovalChecked,
        NotificationRuntimeEventKind::ServiceCall,
        NotificationRuntimeEventKind::ProviderCallStarted,
        NotificationRuntimeEventKind::ProviderCallSucceeded,
        NotificationRuntimeEventKind::DeliveryStatusChanged,
        NotificationRuntimeEventKind::HealthReported,
        NotificationRuntimeEventKind::SnapshotRecorded,
    ] {
        assert!(emitted.contains(&kind));
    }
}

async fn receive_kind(
    events: &mut tokio::sync::broadcast::Receiver<
        super::notification_service_provider::NotificationRuntimeEvent,
    >,
    expected: NotificationRuntimeEventKind,
) -> super::notification_service_provider::NotificationRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.kind == expected {
            return event;
        }
    }
}

fn event_kind_for_command(command: &str) -> NotificationRuntimeEventKind {
    match command {
        "notification.register_action" | "notification.unregister_action" => {
            NotificationRuntimeEventKind::ActionReceived
        }
        "notification.register_subscription" | "notification.revoke_subscription" => {
            NotificationRuntimeEventKind::SubscriptionChanged
        }
        "notification.publish"
        | "notification.schedule"
        | "notification.update"
        | "notification.cancel"
        | "notification.acknowledge"
        | "notification.dismiss"
        | "notification.inspect_delivery" => NotificationRuntimeEventKind::DeliveryStatusChanged,
        _ => NotificationRuntimeEventKind::ServiceCall,
    }
}

async fn dispatch_notification(
    runtime: &ServiceRuntime,
    service_id: &macaca_proto::KernelServiceId,
    command: &str,
    trace_id: &str,
    payload: serde_json::Value,
) {
    runtime
        .call(
            service_id,
            ServiceBusSource::new("notification-provider-test"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                payload,
                TraceContext::new(trace_id),
            ),
        )
        .await
        .unwrap();
}

#[test]
fn notification_delivery_status_remains_bounded() {
    let statuses = [
        NotificationDeliveryStatus::Accepted,
        NotificationDeliveryStatus::Scheduled,
        NotificationDeliveryStatus::Canceled,
        NotificationDeliveryStatus::Acknowledged,
        NotificationDeliveryStatus::Dismissed,
        NotificationDeliveryStatus::Delivered,
    ];
    assert!(statuses
        .iter()
        .all(|status| format!("{:?}", status).len() < 32));
}
