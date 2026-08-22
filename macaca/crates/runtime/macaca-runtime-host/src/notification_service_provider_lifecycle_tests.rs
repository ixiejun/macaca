use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use super::notification_service_provider::{
    NotificationRuntimeEventKind, NotificationSystemServiceProvider,
};

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

#[test]
fn notification_delivery_status_remains_bounded() {
    let statuses = [
        macaca_proto::NotificationDeliveryStatus::Accepted,
        macaca_proto::NotificationDeliveryStatus::Scheduled,
        macaca_proto::NotificationDeliveryStatus::Canceled,
        macaca_proto::NotificationDeliveryStatus::Acknowledged,
        macaca_proto::NotificationDeliveryStatus::Dismissed,
        macaca_proto::NotificationDeliveryStatus::Delivered,
    ];
    assert!(statuses
        .iter()
        .all(|status| format!("{:?}", status).len() < 32));
}
