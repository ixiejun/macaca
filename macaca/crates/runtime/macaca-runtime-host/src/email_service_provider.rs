//! Provider-neutral runtime adapter for the communication-email pack.
//!
//! This mock Strategy is limited to conformance behavior. It retains opaque
//! delivery references only, so addresses, bodies, attachments, OAuth tokens,
//! SMTP credentials, webhook secrets, and provider payloads stay outside the
//! generic runtime and remain owned by replaceable provider adapters.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    EmailProviderCapability, EmailProviderSnapshot, EmailRateLimitStatus, KernelServiceId,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef, COMMUNICATION_EMAIL_COMMANDS, COMMUNICATION_EMAIL_PACK_ID,
    COMMUNICATION_EMAIL_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe email observation emitted after canonical service dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: EmailRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded email event taxonomy that excludes message or provider payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    Composed,
    DraftChanged,
    SendRequested,
    SendAccepted,
    SendFailed,
    MailboxSynced,
    AttachmentFetched,
    DeliveryEventIngested,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Provider adapter Bridge for email transports and remote APIs.
///
/// Concrete implementations live in optional provider crates. The runtime only
/// exchanges canonical service commands and redacted reference results, keeping
/// provider credentials and native payloads outside the OS contract.
#[async_trait]
pub trait EmailProviderAdapter: Send + Sync {
    /// Return the descriptor-only capability report for this adapter Strategy.
    fn capability(&self) -> EmailProviderCapability;

    /// Dispatch a canonical command and return an opaque provider-owned reference.
    async fn dispatch(&self, command: &ServiceCommand) -> ServiceResult<String>;

    /// Record an externally received event using only a verified reference.
    async fn ingest_event(&self, event_ref: &str, idempotency_key: &str) -> ServiceResult<()>;
}

/// Deterministic mock or fail-closed Null Object email provider.
pub struct EmailSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<EmailRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl EmailSystemServiceProvider {
    /// Build the provider-neutral mock used for runtime conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Build an explicit unavailable provider for optional adapter absence.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: email_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Return descriptor-derived capability facts without SMTP or mailbox specifics.
    pub fn capability(&self) -> EmailProviderCapability {
        EmailProviderCapability {
            provider_class: "mock".into(),
            supported_commands: COMMUNICATION_EMAIL_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect(),
            supports_drafts: true,
            supports_scheduled_send: true,
            supports_mailbox_sync: true,
            supports_event_ingest: true,
            supports_attachment_handles: true,
            supports_sync_cursors: true,
            supports_health: true,
            rate_limit_bucket: "runtime_host_default".into(),
            max_attachment_bytes: 65_536,
            max_recipients: 100,
            availability: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to sanitized email lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<EmailRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture a bounded Memento with sender counts and rate metadata only.
    pub async fn snapshot(&self) -> EmailProviderSnapshot {
        let count = self.references.read().await.len().min(u32::MAX as usize) as u32;
        let _ = self.events.send(event(
            "email.snapshot",
            "snapshot:email-provider",
            EmailRuntimeEventKind::SnapshotRecorded,
        ));
        EmailProviderSnapshot {
            descriptor_hash: "email:descriptor".into(),
            provider_class: "mock".into(),
            sender_identity_count: count,
            rate_limits: BTreeMap::from([(
                "provider_calls".into(),
                EmailRateLimitStatus {
                    bucket: "provider_calls".into(),
                    remaining: 100,
                    reset_epoch_ms: None,
                },
            )]),
        }
    }
}

#[async_trait]
impl SystemService for EmailSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "email.declaration",
            "declaration:email-provider",
            EmailRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "email provider started");
        Ok(())
    }
    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            if command.name.as_str() == "email.send" {
                let _ = self.events.send(event(
                    &command.name.to_string(),
                    &trace.trace_id,
                    EmailRuntimeEventKind::SendFailed,
                ));
            }
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                EmailRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "email provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMUNICATION_EMAIL_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                EmailRuntimeEventKind::ProviderCallFailed,
            ));
            warn!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "email provider rejected unsupported command");
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("email:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "email provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "email_handle_ref":reference, "provider_class":"mock", "delivery_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "email provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "email provider cleanup completed");
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
            "email.health",
            "health:email-provider",
            EmailRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a service descriptor only from the proto-owned email contract.
pub fn email_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMUNICATION_EMAIL_SERVICE_ID),
        ServiceType::new("communication.email"),
        TraceSchemaRef::new("email.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMUNICATION_EMAIL_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMUNICATION_EMAIL_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [EmailRuntimeEventKind] {
    use EmailRuntimeEventKind::*;
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
fn command_event_kinds(command: &str) -> &'static [EmailRuntimeEventKind] {
    use EmailRuntimeEventKind::*;
    match command {
        "email.compose" | "email.validate_recipients" => &[Composed],
        "email.save_draft" | "email.update_draft" => &[DraftChanged],
        "email.send" => &[SendRequested, SendAccepted],
        "email.schedule_send" | "email.cancel_scheduled_send" => &[SendRequested],
        "email.sync_mailbox"
        | "email.list_threads"
        | "email.fetch_message"
        | "email.apply_labels"
        | "email.mark_read" => &[MailboxSynced],
        "email.fetch_attachment" => &[AttachmentFetched],
        "email.delivery_status" | "email.ingest_event" => &[DeliveryEventIngested],
        _ => &[ServiceCall],
    }
}
fn event(command: &str, trace_id: &str, kind: EmailRuntimeEventKind) -> EmailRuntimeEvent {
    EmailRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
