//! Provider-neutral runtime adapter for the communication-calendar pack.
//!
//! The adapter implements a deterministic mock Strategy for conformance and
//! composition tests. It retains only opaque event and watch references, never
//! calendar descriptions, invitation payloads, credentials, exports, or vendor
//! responses. Concrete connectors remain replaceable service providers.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, CalendarProviderCapability,
    CalendarProviderSnapshot, DomainPackProviderCapabilityState, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    COMMUNICATION_CALENDAR_COMMANDS, COMMUNICATION_CALENDAR_PACK_ID,
    COMMUNICATION_CALENDAR_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe calendar observation emitted after canonical provider dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CalendarRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded calendar audit categories independent of connector implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    SourceListed,
    EventQueried,
    EventMutated,
    InviteAction,
    AvailabilityChecked,
    ReminderChanged,
    ConferenceChanged,
    SyncRecorded,
    WatchChanged,
    ConflictInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock and explicit unavailable calendar `SystemService` provider.
pub struct CalendarSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CalendarRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    watches: RwLock<BTreeSet<String>>,
    unavailable_reason: Option<String>,
}

impl CalendarSystemServiceProvider {
    /// Build the provider-neutral mock Strategy used by runtime conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build the fail-closed Null Object for optional provider absence.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: calendar_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            watches: RwLock::new(BTreeSet::new()),
            unavailable_reason,
        }
    }

    /// Report descriptor-derived generic capability facts without connector data.
    pub fn capability(&self) -> CalendarProviderCapability {
        CalendarProviderCapability {
            provider_class: "mock".into(),
            supported_commands: COMMUNICATION_CALENDAR_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect(),
            supports_event_crud: true,
            supports_recurrence: true,
            supports_availability: true,
            supports_sync_watch: true,
            supports_icalendar: true,
            max_recurrence_expansion: 128,
            availability: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to redacted calendar audit observations.
    pub fn subscribe(&self) -> broadcast::Receiver<CalendarRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture a bounded Memento without raw cursors, event data, or watch endpoints.
    pub async fn snapshot(&self) -> CalendarProviderSnapshot {
        let reference_count = self.references.read().await.len().min(u32::MAX as usize) as u32;
        let watch_count = self.watches.read().await.len().min(u32::MAX as usize) as u32;
        let _ = self.events.send(event(
            "calendar.snapshot",
            "snapshot:calendar-provider",
            CalendarRuntimeEventKind::SnapshotRecorded,
        ));
        CalendarProviderSnapshot {
            descriptor_hash: "calendar:descriptor".into(),
            provider_class: "mock".into(),
            source_count: reference_count,
            watch_count,
            cursor_hashes: BTreeMap::from([(
                "sync_cursor".into(),
                "bounded:provider-owned".into(),
            )]),
        }
    }
}

#[async_trait]
impl SystemService for CalendarSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "calendar.declaration",
            "declaration:calendar-provider",
            CalendarRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "calendar provider started");
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
                CalendarRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "calendar provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMUNICATION_CALENDAR_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("calendar:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        if command.name.as_str() == "calendar.register_watch" {
            self.watches.write().await.insert(reference.clone());
        }
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "calendar provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "calendar_handle_ref":reference, "provider_class":"mock", "cursor_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "calendar provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        self.watches.write().await.clear();
        info!(service_id = %self.descriptor.id, "calendar provider cleanup completed");
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
            "calendar.health",
            "health:calendar-provider",
            CalendarRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a service descriptor exclusively from the proto-owned calendar contract.
pub fn calendar_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMUNICATION_CALENDAR_SERVICE_ID),
        ServiceType::new("communication.calendar"),
        TraceSchemaRef::new("calendar.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMUNICATION_CALENDAR_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMUNICATION_CALENDAR_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [CalendarRuntimeEventKind] {
    use CalendarRuntimeEventKind::*;
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
fn event_kind(command: &str) -> CalendarRuntimeEventKind {
    use CalendarRuntimeEventKind::*;
    match command {
        "calendar.list_calendars" => SourceListed,
        "calendar.query_events" | "calendar.get_event" => EventQueried,
        "calendar.create_event" | "calendar.update_event" | "calendar.delete_event" => EventMutated,
        "calendar.respond_invite" => InviteAction,
        "calendar.check_availability" | "calendar.propose_times" => AvailabilityChecked,
        "calendar.set_reminder" => ReminderChanged,
        "calendar.manage_conference" => ConferenceChanged,
        "calendar.sync_events" | "calendar.import_icalendar" | "calendar.export_icalendar" => {
            SyncRecorded
        }
        "calendar.register_watch" => WatchChanged,
        "calendar.inspect_conflicts" => ConflictInspected,
        _ => ServiceCall,
    }
}
fn event(command: &str, trace_id: &str, kind: CalendarRuntimeEventKind) -> CalendarRuntimeEvent {
    CalendarRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
