//! Provider-neutral runtime adapter for the communication-messaging pack.
//!
//! The mock Strategy proves canonical dispatch without retaining conversation
//! bodies, attachments, credentials, webhook payloads, or platform-specific
//! message formats. Connector adapters remain replaceable outside SDK and apps.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, MessagingConversationKind, MessagingProviderCapability,
    MessagingProviderSnapshot, MessagingRateLimitStatus, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    COMMUNICATION_MESSAGING_COMMANDS, COMMUNICATION_MESSAGING_PACK_ID,
    COMMUNICATION_MESSAGING_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe message lifecycle observation for audit and replay consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: MessagingRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded messaging taxonomy that never serializes provider message payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    ConversationChanged,
    ParticipantsInspected,
    MessageRequested,
    MessageChanged,
    ReactionChanged,
    ReadReceiptChanged,
    AttachmentReferenced,
    DeliveryInspected,
    TypingSent,
    EventIngested,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    ProviderCallFailed,
    SendRequested,
    SendAccepted,
    SendFailed,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Provider adapter Bridge for messaging transports and remote APIs.
///
/// Concrete adapters are installed outside the OS layer. They receive canonical
/// commands and exchange opaque references, so native payloads and credentials
/// cannot leak into SDK callers or application framework surfaces.
#[async_trait]
pub trait MessagingProviderAdapter: Send + Sync {
    /// Return a descriptor-only capability report for the adapter Strategy.
    fn capability(&self) -> MessagingProviderCapability;

    /// Dispatch a normalized command and return an opaque provider-owned reference.
    async fn dispatch(&self, command: &ServiceCommand) -> ServiceResult<String>;

    /// Ingest a verified provider event by reference with idempotency protection.
    async fn ingest_event(&self, event_ref: &str, idempotency_key: &str) -> ServiceResult<()>;
}

/// Deterministic mock or explicit unavailable messaging `SystemService` adapter.
pub struct MessagingSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<MessagingRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl MessagingSystemServiceProvider {
    /// Build the provider-neutral mock Strategy for tests and composition roots.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Build the fail-closed Null Object for absent optional connectors.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: messaging_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Return generic descriptor-derived capabilities with bounded delivery limits.
    pub fn capability(&self) -> MessagingProviderCapability {
        MessagingProviderCapability {
            provider_class: "mock".into(),
            supported_commands: COMMUNICATION_MESSAGING_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect(),
            supported_conversation_kinds: BTreeSet::from([MessagingConversationKind::Channel]),
            supports_reactions: true,
            supports_typing: true,
            supports_event_ingest: true,
            supports_attachment_handles: true,
            supports_cursors: true,
            supports_health: true,
            supported_formats: BTreeSet::from([
                "text".into(),
                "markdown".into(),
                "reference".into(),
            ]),
            rate_limit_bucket: "runtime_host_default".into(),
            max_attachment_bytes: 65_536,
            max_message_bytes: 65_536,
            availability: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to sanitized messaging lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<MessagingRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture a Memento with counts and rate metadata only, never message data.
    pub async fn snapshot(&self) -> MessagingProviderSnapshot {
        let count = self.references.read().await.len().min(u32::MAX as usize) as u32;
        let _ = self.events.send(event(
            "messaging.snapshot",
            "snapshot:messaging-provider",
            MessagingRuntimeEventKind::SnapshotRecorded,
        ));
        MessagingProviderSnapshot {
            descriptor_hash: "messaging:descriptor".into(),
            provider_class: "mock".into(),
            active_conversation_count: count,
            rate_limits: BTreeMap::from([(
                "provider_calls".into(),
                MessagingRateLimitStatus {
                    bucket: "provider_calls".into(),
                    remaining: 100,
                    reset_epoch_ms: None,
                },
            )]),
        }
    }
}

#[async_trait]
impl SystemService for MessagingSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "messaging.declaration",
            "declaration:messaging-provider",
            MessagingRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "messaging provider started");
        Ok(())
    }
    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            if matches!(
                command.name.as_str(),
                "messaging.send_message" | "messaging.reply_message"
            ) {
                let _ = self.events.send(event(
                    &command.name.to_string(),
                    &trace.trace_id,
                    MessagingRuntimeEventKind::SendFailed,
                ));
            }
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                MessagingRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "messaging provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMUNICATION_MESSAGING_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                MessagingRuntimeEventKind::ProviderCallFailed,
            ));
            warn!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "messaging provider rejected unsupported command");
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("messaging:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in common_event_kinds()
            .iter()
            .chain(command_event_kinds(command.name.as_str()).iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "messaging provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "message_handle_ref":reference, "provider_class":"mock", "delivery_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "messaging provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "messaging provider cleanup completed");
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
            "messaging.health",
            "health:messaging-provider",
            MessagingRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a service descriptor only from the proto-owned messaging contract.
pub fn messaging_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMUNICATION_MESSAGING_SERVICE_ID),
        ServiceType::new("communication.messaging"),
        TraceSchemaRef::new("messaging.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMUNICATION_MESSAGING_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMUNICATION_MESSAGING_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [MessagingRuntimeEventKind] {
    use MessagingRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        ResourceReserved,
        EntitlementChecked,
        ApprovalChecked,
        ServiceCall,
        ProviderCallStarted,
        ProviderCallSucceeded,
    ]
}
fn command_event_kinds(command: &str) -> &'static [MessagingRuntimeEventKind] {
    use MessagingRuntimeEventKind::*;
    match command {
        "messaging.find_conversation" | "messaging.create_conversation" => &[ConversationChanged],
        "messaging.inspect_participants" => &[ParticipantsInspected],
        "messaging.list_messages" | "messaging.fetch_message" => &[MessageRequested],
        "messaging.send_message" | "messaging.reply_message" => {
            &[SendRequested, MessageChanged, SendAccepted]
        }
        "messaging.edit_message" | "messaging.delete_message" => &[MessageChanged],
        "messaging.add_reaction" | "messaging.remove_reaction" => &[ReactionChanged],
        "messaging.mark_read" => &[ReadReceiptChanged],
        "messaging.attach_handle" => &[AttachmentReferenced],
        "messaging.delivery_status" => &[DeliveryInspected],
        "messaging.send_typing" => &[TypingSent],
        "messaging.ingest_event" => &[EventIngested],
        _ => &[ServiceCall],
    }
}
fn event(command: &str, trace_id: &str, kind: MessagingRuntimeEventKind) -> MessagingRuntimeEvent {
    MessagingRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
