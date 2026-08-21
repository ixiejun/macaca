//! Runtime-host service Strategy for provider-neutral media transcription.
//!
//! This deterministic implementation proves the canonical service path without
//! reading audio, retaining transcript text, selecting a speech provider, or
//! exposing transport/provider payloads. Concrete adapters may replace it only
//! through the runtime-host composition root.

use std::collections::BTreeSet;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::media_transcription::{
    TranscriptionProviderCapability, MEDIA_TRANSCRIPTION_COMMANDS, MEDIA_TRANSCRIPTION_PACK_ID,
    MEDIA_TRANSCRIPTION_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tracing::{info, warn};

/// Bounded event used for audit/replay evidence; source and transcript payloads are absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Mock or unavailable transcription provider selected at the host composition root.
pub struct MediaTranscriptionSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: tokio::sync::broadcast::Sender<TranscriptionRuntimeEvent>,
}

impl MediaTranscriptionSystemServiceProvider {
    /// Build a deterministic provider for contract, ABI, and replay tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a fail-closed Null Object when no transcription module is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            unavailable_reason,
            events,
        }
    }

    /// Report only descriptor-owned capability facts and never provider internals.
    pub fn capability(&self) -> TranscriptionProviderCapability {
        TranscriptionProviderCapability {
            provider_class: if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
            languages: BTreeSet::from(["und".into()]),
            model_classes: BTreeSet::from(["provider-neutral".into()]),
            features: if self.unavailable_reason.is_some() {
                BTreeSet::new()
            } else {
                BTreeSet::from(["metadata_only".into()])
            },
            state: if self.unavailable_reason.is_some() {
                DomainPackProviderCapabilityState::Unavailable
            } else {
                DomainPackProviderCapabilityState::Preview
            },
        }
    }

    /// Subscribe to sanitized events that carry no audio, chunk, or transcript data.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TranscriptionRuntimeEvent> {
        self.events.subscribe()
    }

    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(TranscriptionRuntimeEvent {
            command: command.into(),
            trace_id: trace_id.into(),
            outcome,
            replay_ref: format!("replay:transcription:{trace_id}"),
        });
    }
}

#[async_trait]
impl SystemService for MediaTranscriptionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(MEDIA_TRANSCRIPTION_SERVICE_ID),
            ServiceType::new("media.transcription"),
            TraceSchemaRef::new("media.transcription.replay.v1"),
        );
        descriptor
            .metadata
            .insert("pack_id".into(), MEDIA_TRANSCRIPTION_PACK_ID.into());
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor.metadata.insert(
            "command_count".into(),
            MEDIA_TRANSCRIPTION_COMMANDS.len().to_string(),
        );
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = MEDIA_TRANSCRIPTION_SERVICE_ID,
            "media transcription provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !MEDIA_TRANSCRIPTION_COMMANDS.contains(&operation) {
            self.emit(operation, &trace.trace_id, "unsupported");
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit(operation, &trace.trace_id, "unavailable");
            warn!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, reason_code = %reason, "media transcription provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        self.emit(operation, &trace.trace_id, "completed");
        info!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "media transcription command completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"metadata_only","operation":operation,"artifact_ref":format!("artifact:transcription:{}", trace.trace_id)}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self
            .unavailable_reason
            .as_ref()
            .map_or(ServiceHealth::Healthy, |reason| {
                ServiceHealth::Unavailable {
                    reason: reason.clone(),
                }
            }))
    }
}
