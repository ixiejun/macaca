//! Provider-neutral runtime adapter for the communication-inbox pack.
//!
//! The deterministic mock retains only opaque source and item references for
//! conformance. Raw bodies, attachments, credentials, webhook secrets, and
//! connector payloads are deliberately excluded from state, logs, events, and
//! results so concrete inbox adapters remain independently replaceable.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    InboxProviderCapability, InboxProviderSnapshot, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    COMMUNICATION_INBOX_COMMANDS, COMMUNICATION_INBOX_PACK_ID, COMMUNICATION_INBOX_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe inbox observation emitted after descriptor-owned dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: InboxRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded inbox audit taxonomy with no body, attachment, or credential fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    SourceChanged,
    SourceHealthReported,
    SyncRecorded,
    CheckpointRecorded,
    EventIngested,
    ItemQueried,
    ItemMutated,
    ClaimChanged,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or fail-closed Null Object inbox service provider.
pub struct InboxSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<InboxRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl InboxSystemServiceProvider {
    /// Build the provider-neutral mock Strategy used by composition and replay tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Build the explicit unavailable Strategy for optional connector absence.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: inbox_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Report generic source and command support without connector-specific routing.
    pub fn capability(&self) -> InboxProviderCapability {
        InboxProviderCapability {
            provider_class: "mock".into(),
            supported_commands: COMMUNICATION_INBOX_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect(),
            source_kinds: BTreeSet::from(["reference".into()]),
            supports_query: true,
            supports_mutation: true,
            supports_claims: true,
            page_limit: 100,
            availability: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to redacted inbox lifecycle facts.
    pub fn subscribe(&self) -> broadcast::Receiver<InboxRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture bounded Memento counts and cursor hashes without message content.
    pub async fn snapshot(&self) -> InboxProviderSnapshot {
        let count = self.references.read().await.len();
        let _ = self.events.send(event(
            "inbox.snapshot",
            "snapshot:inbox-provider",
            InboxRuntimeEventKind::SnapshotRecorded,
        ));
        InboxProviderSnapshot {
            descriptor_hash: "inbox:descriptor".into(),
            provider_class: "mock".into(),
            source_count: count.min(u32::MAX as usize) as u32,
            item_count: count.min(u64::MAX as usize) as u64,
            cursor_hashes: BTreeMap::from([(
                "sync_cursor".into(),
                "bounded:provider-owned".into(),
            )]),
        }
    }
}

#[async_trait]
impl SystemService for InboxSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "inbox.declaration",
            "declaration:inbox-provider",
            InboxRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "inbox provider started");
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
                InboxRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "inbox provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMUNICATION_INBOX_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("inbox:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "inbox provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "inbox_handle_ref":reference, "provider_class":"mock", "cursor_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "inbox provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "inbox provider cleanup completed");
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
            "inbox.health",
            "health:inbox-provider",
            InboxRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor exclusively from the proto-owned inbox contract.
pub fn inbox_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMUNICATION_INBOX_SERVICE_ID),
        ServiceType::new("communication.inbox"),
        TraceSchemaRef::new("inbox.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMUNICATION_INBOX_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMUNICATION_INBOX_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [InboxRuntimeEventKind] {
    use InboxRuntimeEventKind::*;
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
fn event_kind(command: &str) -> InboxRuntimeEventKind {
    use InboxRuntimeEventKind::*;
    match command {
        "inbox.register_source" | "inbox.update_source" | "inbox.revoke_source" => SourceChanged,
        "inbox.sync_sources" | "inbox.resume_sync" => SyncRecorded,
        "inbox.ingest_event" => EventIngested,
        "inbox.list_items"
        | "inbox.search_items"
        | "inbox.get_item"
        | "inbox.fetch_body"
        | "inbox.fetch_attachment"
        | "inbox.list_threads"
        | "inbox.summarize_item" => ItemQueried,
        "inbox.label_item" | "inbox.move_item" | "inbox.archive_item" | "inbox.mark_read" => {
            ItemMutated
        }
        "inbox.claim_item" | "inbox.release_item" => ClaimChanged,
        _ => ServiceCall,
    }
}
fn event(command: &str, trace_id: &str, kind: InboxRuntimeEventKind) -> InboxRuntimeEvent {
    InboxRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
