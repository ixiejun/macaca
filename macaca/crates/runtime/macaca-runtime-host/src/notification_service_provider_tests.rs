use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    NotificationDeliveryStatus, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError,
    ServiceHealth, TraceContext, COMMUNICATION_NOTIFICATION_COMMANDS,
};

use super::notification_service_provider::{
    transition_notification_state, NotificationLifecycleState, NotificationRuntimeEventKind,
    NotificationSystemServiceProvider,
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
        assert!(event.event_name.starts_with("notifications."));
        assert!(!result.output.to_string().contains("must-not-leak"));
        assert!(!format!("{:?}", event).contains("must-not-leak"));
    }
    assert_eq!(
        provider.capability().supported_commands.len(),
        COMMUNICATION_NOTIFICATION_COMMANDS.len()
    );
}

#[tokio::test]
async fn notification_commands_emit_stable_audit_event_names() {
    let provider = NotificationSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for (command, expected) in [
        ("notification.publish", "notifications.notification_posted"),
        (
            "notification.schedule",
            "notifications.notification_scheduled",
        ),
        ("notification.update", "notifications.notification_posted"),
        (
            "notification.cancel",
            "notifications.notification_cancelled",
        ),
        (
            "notification.register_action",
            "notifications.interaction_received",
        ),
        (
            "notification.unregister_action",
            "notifications.command_completed",
        ),
        (
            "notification.acknowledge",
            "notifications.interaction_received",
        ),
        ("notification.dismiss", "notifications.interaction_received"),
        (
            "notification.register_subscription",
            "notifications.interaction_received",
        ),
        (
            "notification.revoke_subscription",
            "notifications.interaction_received",
        ),
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
        let event = receive_kind(&mut events, event_kind_for_command(command)).await;
        assert_eq!(event.event_name, expected);
    }
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
async fn notification_admission_denies_policy_facts_before_delivery_allocation() {
    let provider = NotificationSystemServiceProvider::mock();
    for (trace, payload) in [
        ("auth", serde_json::json!({"authorization_denied": true})),
        ("content", serde_json::json!({"content_size_bytes": 65_537})),
        ("action", serde_json::json!({"action_count": 9})),
        (
            "background",
            serde_json::json!({"background_action_denied": true}),
        ),
        ("quiet", serde_json::json!({"quiet_hours": true})),
        ("schedule", serde_json::json!({"schedule_horizon_days": 31})),
    ] {
        let command = if trace == "schedule" {
            "notification.schedule"
        } else {
            "notification.publish"
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await.active_delivery_count, 0);
}

#[test]
fn notification_lifecycle_transitions_fail_closed() {
    assert_eq!(
        transition_notification_state(NotificationLifecycleState::Requested, "authorize"),
        Some(NotificationLifecycleState::Authorized)
    );
    assert_eq!(
        transition_notification_state(NotificationLifecycleState::Authorized, "schedule"),
        Some(NotificationLifecycleState::Scheduled)
    );
    assert_eq!(
        transition_notification_state(NotificationLifecycleState::Scheduled, "cancel"),
        Some(NotificationLifecycleState::Cancelled)
    );
    assert_eq!(
        transition_notification_state(NotificationLifecycleState::Delivered, "cancel"),
        None
    );
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
async fn notification_every_command_replays_after_runtime_restart() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    }));
    let provider = Arc::new(NotificationSystemServiceProvider::mock());
    let mut observer = provider.subscribe();
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
        .start(
            &service_id,
            TraceContext::new("trace-notification-replay-start"),
        )
        .await
        .unwrap();
    runtime
        .stop(
            &service_id,
            TraceContext::new("trace-notification-replay-stop"),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            TraceContext::new("trace-notification-replay-restart"),
        )
        .await
        .unwrap();

    for (index, command) in COMMUNICATION_NOTIFICATION_COMMANDS.iter().enumerate() {
        let trace_id = format!("trace-notification-replay-{index}");
        dispatch_notification(
            &runtime,
            &service_id,
            command,
            &trace_id,
            serde_json::json!({"body": "redacted"}),
        )
        .await;
        let event = receive_kind(&mut observer, event_kind_for_command(command)).await;
        assert_eq!(event.command, *command);
        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.replay_ref, format!("replay:{trace_id}"));
        assert!(event.event_name.starts_with("notifications."));
    }

    let runtime_events = events.events().unwrap();
    for (index, _) in COMMUNICATION_NOTIFICATION_COMMANDS.iter().enumerate() {
        let trace_id = format!("trace-notification-replay-{index}");
        assert!(runtime_events.iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"
        }));
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
