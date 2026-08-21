//! Runtime-host provider for the foundation session-state pack.
//!
//! The provider is a deterministic reference-only Strategy. It tracks revision and checkpoint
//! handles, never raw state values, and returns bounded metadata through the canonical service
//! runtime. Embedded durable and remote stores can replace this adapter at the composition root.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, SessionStateProviderCapability,
    SessionStateProviderSnapshot, SessionStateRedactionSummary, TraceSchemaRef,
    FOUNDATION_SESSION_STATE_COMMANDS, FOUNDATION_SESSION_STATE_PACK_ID,
    FOUNDATION_SESSION_STATE_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Sanitized lifecycle observation for session-state replay and audit consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: SessionStateRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded event taxonomy; values and secret material are intentionally absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStateRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    ServiceCall,
    CheckpointCreated,
    RestorePlanned,
    CompactionRequested,
    SessionCleared,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Reference-only mock or unavailable session-state provider.
pub struct FoundationSessionStateSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<SessionStateRuntimeEvent>,
    revisions: RwLock<BTreeMap<String, String>>,
    checkpoints: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl FoundationSessionStateSystemServiceProvider {
    /// Create the deterministic mock Strategy used by conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Create a fail-closed Null Object when session persistence is absent.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: foundation_session_state_service_descriptor(),
            events,
            revisions: RwLock::new(BTreeMap::new()),
            checkpoints: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Report only bounded feature facts and limits.
    pub fn capability(&self) -> SessionStateProviderCapability {
        SessionStateProviderCapability {
            provider_class: if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
            supported_commands: FOUNDATION_SESSION_STATE_COMMANDS
                .iter()
                .map(|command| (*command).into())
                .collect::<BTreeSet<_>>(),
            supports_checkpoints: self.unavailable_reason.is_none(),
            supports_restore: self.unavailable_reason.is_none(),
            supports_compaction: self.unavailable_reason.is_none(),
            supports_redacted_export: self.unavailable_reason.is_none(),
            max_state_bytes: 1_048_576,
            max_checkpoint_bytes: 4_194_304,
            availability: if self.unavailable_reason.is_some() {
                DomainPackProviderCapabilityState::Unavailable
            } else {
                DomainPackProviderCapabilityState::Preview
            },
        }
    }

    /// Subscribe to replay-safe lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionStateRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture a bounded Memento of revision/checkpoint identity only.
    pub async fn snapshot(&self) -> SessionStateProviderSnapshot {
        let revisions = self.revisions.read().await;
        let checkpoints = self.checkpoints.read().await;
        let _ = self.events.send(event(
            "session_state.snapshot",
            "snapshot:session-state-provider",
            SessionStateRuntimeEventKind::SnapshotRecorded,
        ));
        SessionStateProviderSnapshot {
            descriptor_hash: "foundation-session-state:descriptor".into(),
            provider_class: self.capability().provider_class,
            revision_hashes: revisions
                .iter()
                .take(100)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            checkpoint_hashes: checkpoints
                .iter()
                .take(100)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            redaction_summary: SessionStateRedactionSummary {
                redacted_value_count: revisions.len().min(u32::MAX as usize) as u32,
                redacted_secret_reference_count: 0,
            },
        }
    }

    /// Release reference state during runtime shutdown.
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.revisions.write().await.clear();
        self.checkpoints.write().await.clear();
        info!(
            service_id = FOUNDATION_SESSION_STATE_SERVICE_ID,
            "session state provider shutdown completed"
        );
        Ok(())
    }
}

#[async_trait]
impl SystemService for FoundationSessionStateSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "session_state.declaration",
            "declaration:session-state-provider",
            SessionStateRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "session state provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                SessionStateRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, reason_code = %reason, "session state provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !FOUNDATION_SESSION_STATE_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                SessionStateRuntimeEventKind::ProviderCallFailed,
            ));
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let revision_ref = format!("revision:reference:{}", trace.trace_id);
        self.revisions
            .write()
            .await
            .insert(trace.trace_id.clone(), revision_ref.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "session state provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "revision_ref": revision_ref,
                "checkpoint_ref": format!("checkpoint:reference:{}", trace.trace_id),
                "recovery_state": "reference_only",
                "provider_class": "mock",
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let health = match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        };
        let _ = self.events.send(event(
            "session_state.health",
            "health:session-state-provider",
            SessionStateRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor solely from provider-neutral protocol constants.
pub fn foundation_session_state_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FOUNDATION_SESSION_STATE_SERVICE_ID),
        ServiceType::new("foundation.session_state"),
        TraceSchemaRef::new("foundation.session_state.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), FOUNDATION_SESSION_STATE_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        FOUNDATION_SESSION_STATE_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [SessionStateRuntimeEventKind] {
    use SessionStateRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        ResourceReserved,
        ServiceCall,
        ProviderCallSucceeded,
    ]
}

fn event_kind(command: &str) -> SessionStateRuntimeEventKind {
    use SessionStateRuntimeEventKind::*;
    match command {
        "session_state.create_checkpoint" => CheckpointCreated,
        "session_state.restore_checkpoint" => RestorePlanned,
        "session_state.compact_history" => CompactionRequested,
        "session_state.clear_session" => SessionCleared,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: SessionStateRuntimeEventKind,
) -> SessionStateRuntimeEvent {
    SessionStateRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
