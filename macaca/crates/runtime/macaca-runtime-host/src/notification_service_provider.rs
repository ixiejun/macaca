//! Optional runtime-host provider for communication notification conformance.
//!
//! The provider is a deterministic mock Strategy, not a platform notification
//! adapter. It proves descriptor-owned dispatch, health, lifecycle, bounded
//! capability reporting, and sanitized delivery events without interpreting
//! tokens, endpoints, credentials, message bodies, or application behavior.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, NotificationDeliveryChannel, NotificationDeliveryStatus,
    NotificationProviderCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceType, TraceSchemaRef, COMMUNICATION_NOTIFICATION_COMMANDS,
    COMMUNICATION_NOTIFICATION_PACK_ID, COMMUNICATION_NOTIFICATION_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Reference-only notification event published after a canonical service call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: NotificationRuntimeEventKind,
    pub delivery_status: NotificationDeliveryStatus,
    pub replay_ref: String,
}

/// Bounded notification observability categories safe for trace and replay indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    ConsentChecked,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    ActionReceived,
    DeliveryStatusChanged,
    SubscriptionChanged,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or unavailable notification provider behind `SystemService`.
pub struct NotificationSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<NotificationRuntimeEvent>,
    deliveries: RwLock<BTreeMap<String, NotificationDeliveryStatus>>,
    idempotency: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl NotificationSystemServiceProvider {
    /// Build the deterministic provider used by conformance and composition tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a Null Object provider for hosts without a notification implementation.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: notification_service_descriptor(),
            events,
            deliveries: RwLock::new(BTreeMap::new()),
            idempotency: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Return declarative capability facts without exposing platform-provider internals.
    pub fn capability(&self) -> NotificationProviderCapability {
        NotificationProviderCapability {
            provider_class: "mock".into(),
            supported_commands: COMMUNICATION_NOTIFICATION_COMMANDS
                .iter()
                .map(|command| (*command).into())
                .collect(),
            channels: BTreeSet::from([
                NotificationDeliveryChannel::Local,
                NotificationDeliveryChannel::Push,
                NotificationDeliveryChannel::InApp,
            ]),
            supports_schedule: true,
            supports_update_cancel: true,
            supports_actions: true,
            supports_subscriptions: true,
            max_payload_bytes: 65_536,
            max_actions: 8,
            availability: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to sanitized delivery lifecycle changes for audit and shell observers.
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationRuntimeEvent> {
        self.events.subscribe()
    }

    /// Return a bounded Memento containing counts and hashes, never message or target data.
    pub async fn snapshot(&self) -> macaca_proto::NotificationProviderSnapshot {
        let deliveries = self.deliveries.read().await;
        let snapshot = macaca_proto::NotificationProviderSnapshot {
            descriptor_hash: "notification:descriptor".into(),
            provider_class: "mock".into(),
            active_delivery_count: deliveries.len().try_into().unwrap_or(u32::MAX),
            subscription_count: 0,
            quota_hashes: BTreeMap::from([(
                "retry_metadata".into(),
                "bounded:provider-owned".into(),
            )]),
        };
        let _ = self.events.send(event(
            "notification.snapshot",
            "snapshot:provider",
            NotificationRuntimeEventKind::SnapshotRecorded,
            NotificationDeliveryStatus::Unknown,
        ));
        snapshot
    }

    /// Return a stable bounded page of opaque delivery references for provider diagnostics.
    pub async fn delivery_page(&self, page_size: usize) -> Vec<String> {
        const MAX_PAGE_SIZE: usize = 100;
        self.deliveries
            .read()
            .await
            .keys()
            .take(page_size.min(MAX_PAGE_SIZE))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl SystemService for NotificationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "notification.declaration",
            "declaration:provider",
            NotificationRuntimeEventKind::PackDeclared,
            NotificationDeliveryStatus::Unknown,
        ));
        info!(service_id = %self.descriptor.id, "notification provider started");
        Ok(())
    }

    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                NotificationRuntimeEventKind::Unavailable,
                NotificationDeliveryStatus::Unknown,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "notification provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMUNICATION_NOTIFICATION_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let status = status_for(command.name.as_str());
        let idempotency_key = command
            .metadata
            .get("idempotency_key")
            .cloned()
            .unwrap_or_else(|| trace.trace_id.clone());
        let delivery_ref = {
            let mut idempotency = self.idempotency.write().await;
            idempotency
                .entry(idempotency_key)
                .or_insert_with(|| format!("delivery:{}", trace.trace_id))
                .clone()
        };
        self.deliveries
            .write()
            .await
            .insert(delivery_ref.clone(), status);
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                *kind,
                status,
            ));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "notification provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "delivery_handle_ref": delivery_ref,
                "delivery_status": format!("{:?}", status),
                "retry_metadata": "bounded:provider-owned",
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "notification provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.deliveries.write().await.clear();
        self.idempotency.write().await.clear();
        info!(service_id = %self.descriptor.id, "notification provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let health = match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        };
        let _ = self.events.send(event(
            "notification.health",
            "health:provider",
            NotificationRuntimeEventKind::HealthReported,
            NotificationDeliveryStatus::Unknown,
        ));
        Ok(health)
    }
}

/// Descriptor derived from the proto pack contract, never from a platform provider.
pub fn notification_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMUNICATION_NOTIFICATION_SERVICE_ID),
        ServiceType::new("communication.notification"),
        TraceSchemaRef::new("notification.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMUNICATION_NOTIFICATION_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMUNICATION_NOTIFICATION_COMMANDS.len().to_string(),
    );
    descriptor
}

fn status_for(command: &str) -> NotificationDeliveryStatus {
    match command {
        "notification.schedule" => NotificationDeliveryStatus::Scheduled,
        "notification.cancel" | "notification.revoke_subscription" => {
            NotificationDeliveryStatus::Canceled
        }
        "notification.acknowledge" => NotificationDeliveryStatus::Acknowledged,
        "notification.dismiss" => NotificationDeliveryStatus::Dismissed,
        "notification.inspect_delivery" => NotificationDeliveryStatus::Delivered,
        _ => NotificationDeliveryStatus::Accepted,
    }
}

fn event_kind(command: &str) -> NotificationRuntimeEventKind {
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

fn common_event_kinds() -> &'static [NotificationRuntimeEventKind] {
    use NotificationRuntimeEventKind::*;
    &[
        AdmissionValidated,
        ConsentChecked,
        PolicyDecision,
        ResourceReserved,
        EntitlementChecked,
        ApprovalChecked,
        ServiceCall,
        ProviderCallStarted,
        ProviderCallSucceeded,
    ]
}

fn event(
    command: &str,
    trace_id: &str,
    kind: NotificationRuntimeEventKind,
    delivery_status: NotificationDeliveryStatus,
) -> NotificationRuntimeEvent {
    NotificationRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        delivery_status,
        replay_ref: format!("replay:{trace_id}"),
    }
}
