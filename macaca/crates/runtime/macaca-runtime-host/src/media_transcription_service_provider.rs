//! Runtime-host service Strategy for provider-neutral media transcription.
//!
//! This deterministic implementation proves the canonical service path without
//! reading audio, retaining transcript text, selecting a speech provider, or
//! exposing transport/provider payloads. Concrete adapters may replace it only
//! through the runtime-host composition root.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::media_transcription::{
    TranscriptionProviderCapability, MEDIA_TRANSCRIPTION_COMMANDS, MEDIA_TRANSCRIPTION_PACK_ID,
    MEDIA_TRANSCRIPTION_SERVICE_ID,
};
use macaca_proto::{
    admit_transcription_operation, domain_pack_command_trace, domain_pack_service_result,
    DomainPackProviderCapabilityState, KernelServiceId, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    TranscriptionPreflightFacts, TranscriptionPreflightFailure,
};
use tracing::{info, warn};

use crate::media_transcription_service_state::TranscriptionSideEffectLedger;

/// Bounded event used for audit/replay evidence; source and transcript payloads are absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionRuntimeEvent {
    pub command: String,
    /// Stable provider-neutral audit name; source audio and transcript payloads are omitted.
    pub event_name: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Mock or unavailable transcription provider selected at the host composition root.
pub struct MediaTranscriptionSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: tokio::sync::broadcast::Sender<TranscriptionRuntimeEvent>,
    side_effects: Arc<TranscriptionSideEffectLedger>,
    admission_facts: TranscriptionPreflightFacts,
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
            side_effects: Arc::new(TranscriptionSideEffectLedger::default()),
            admission_facts: TranscriptionPreflightFacts::permissive(),
        }
    }

    /// Inject host-issued admission evidence at the composition root.
    /// Callers cannot influence this decision through command metadata or payloads.
    pub fn with_admission_facts(mut self, admission_facts: TranscriptionPreflightFacts) -> Self {
        self.admission_facts = admission_facts;
        self
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

    /// Return a replay Memento with only descriptor-owned and aggregate facts.
    ///
    /// This supports restart diagnostics without preserving source handles,
    /// audio chunks, transcript content, artifact contents, or provider data.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let snapshot = BTreeMap::from([
            ("provider_class".into(), self.capability().provider_class),
            (
                "capability_state".into(),
                format!("{:?}", self.capability().state),
            ),
            (
                "command_count".into(),
                MEDIA_TRANSCRIPTION_COMMANDS.len().to_string(),
            ),
            (
                "idempotency_completion_count".into(),
                self.side_effects.completion_count().await.to_string(),
            ),
            (
                "snapshot_schema".into(),
                "media.transcription.replay.v1".into(),
            ),
        ]);
        self.emit(
            "transcription.snapshot",
            "snapshot:provider",
            "snapshot_recorded",
        );
        info!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, completion_count = %snapshot["idempotency_completion_count"], "media transcription provider snapshot recorded");
        snapshot
    }

    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(TranscriptionRuntimeEvent {
            command: command.into(),
            event_name: transcription_audit_event(command).into(),
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
        self.emit(
            "transcription.declaration",
            "declaration:transcription-provider",
            "declared",
        );
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
            self.emit(
                "transcription.command_failed",
                &trace.trace_id,
                "unsupported",
            );
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit("transcription.unavailable", &trace.trace_id, "unavailable");
            warn!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, reason_code = %reason, "media transcription provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if let Err(rejection) = admit_transcription_operation(operation, self.admission_facts) {
            self.emit(
                "transcription.policy_decision",
                &trace.trace_id,
                "preflight_rejected",
            );
            warn!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, rejection = ?rejection, "media transcription command rejected before provider dispatch");
            return Err(preflight_error(rejection));
        }
        if let Err(error) = TranscriptionSideEffectLedger::validate_source_version(&command) {
            self.emit(
                "transcription.command_failed",
                &trace.trace_id,
                "precondition_rejected",
            );
            warn!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "media transcription command rejected due to stale source version");
            return Err(error);
        }
        let artifact_ref = self
            .side_effects
            .outcome_ref(
                &command,
                format!("artifact:transcription:{}", trace.trace_id),
            )
            .await;
        self.emit(operation, &trace.trace_id, "completed");
        for audit in [
            "transcription.admission",
            "transcription.provider_inspection",
            "transcription.policy",
            "transcription.entitlement",
            "transcription.resource",
            "transcription.approval",
        ] {
            self.emit(audit, &trace.trace_id, "validated");
        }
        info!(service_id = MEDIA_TRANSCRIPTION_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "media transcription command completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"metadata_only","operation":operation,"artifact_ref":artifact_ref}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.side_effects.clear().await;
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        self.emit(
            "transcription.health",
            "health:transcription-provider",
            "reported",
        );
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

/// Map lifecycle markers and commands to stable sanitized audit vocabulary.
fn transcription_audit_event(command: &str) -> &'static str {
    match command {
        "transcription.declaration" => "transcription.pack_declared",
        "transcription.snapshot" => "transcription.snapshot_recorded",
        "transcription.health" => "transcription.health",
        "transcription.admission" => "transcription.admission",
        "transcription.provider_inspection" => "transcription.provider_inspection",
        "transcription.policy" => "transcription.policy",
        "transcription.entitlement" => "transcription.entitlement",
        "transcription.resource" => "transcription.resource",
        "transcription.approval" => "transcription.approval",
        "transcription.unavailable" => "transcription.unavailable",
        "transcription.command_failed" => "transcription.failure",
        _ => transcription_success_event(command),
    }
}

/// Map every typed transcription command to a stable, sanitized audit event.
fn transcription_success_event(operation: &str) -> &'static str {
    match operation {
        "transcription.inspect_provider" => "transcription.provider_inspected",
        "transcription.import_source_request" => "transcription.source_import_requested",
        "transcription.open_source" => "transcription.source_opened",
        "transcription.inspect_media" => "transcription.media_inspected",
        "transcription.plan_batch" => "transcription.batch_planned",
        "transcription.batch_request" => "transcription.batch_requested",
        "transcription.plan_stream" => "transcription.stream_planned",
        "transcription.start_stream" => "transcription.stream_started",
        "transcription.append_stream_chunk" => "transcription.stream_chunk_appended",
        "transcription.finish_stream" => "transcription.stream_finished",
        "transcription.cancel_stream" => "transcription.stream_cancelled",
        "transcription.plan_diarization" => "transcription.diarization_planned",
        "transcription.diarization_request" => "transcription.diarization_requested",
        "transcription.align_timestamps" => "transcription.timestamps_aligned",
        "transcription.normalize_transcript" => "transcription.transcript_normalized",
        "transcription.plan_redaction" => "transcription.redaction_planned",
        "transcription.redaction_request" => "transcription.redaction_requested",
        "transcription.plan_subtitle_export" => "transcription.subtitle_export_planned",
        "transcription.subtitle_export_request" => "transcription.subtitle_export_requested",
        "transcription.plan_translation_handoff" => "transcription.translation_handoff_planned",
        "transcription.translation_handoff_request" => {
            "transcription.translation_handoff_requested"
        }
        "transcription.inspect_job" => "transcription.job_inspected",
        "transcription.get_artifact_handle" => "transcription.artifact_handle_created",
        _ => "transcription.command_completed",
    }
}

fn preflight_error(rejection: TranscriptionPreflightFailure) -> ServiceError {
    match rejection {
        TranscriptionPreflightFailure::Denied
        | TranscriptionPreflightFailure::ApprovalRequired
        | TranscriptionPreflightFailure::PolicyDenied
        | TranscriptionPreflightFailure::EntitlementDenied
        | TranscriptionPreflightFailure::RedactionDenied
        | TranscriptionPreflightFailure::TranslationDenied
        | TranscriptionPreflightFailure::ExportDenied
        | TranscriptionPreflightFailure::ArtifactDenied => {
            ServiceError::DisabledByPolicy(format!("transcription_{rejection:?}").to_lowercase())
        }
        TranscriptionPreflightFailure::Unavailable => {
            ServiceError::ServiceUnavailable("transcription_provider_unavailable".into())
        }
        TranscriptionPreflightFailure::QuotaExceeded => {
            ServiceError::AdapterFailure("transcription_quota_exceeded".into())
        }
        TranscriptionPreflightFailure::SchemaMismatch
        | TranscriptionPreflightFailure::FormatUnsupported
        | TranscriptionPreflightFailure::LanguageUnsupported
        | TranscriptionPreflightFailure::ModelUnsupported
        | TranscriptionPreflightFailure::DiarizationUnsupported
        | TranscriptionPreflightFailure::TimestampUnsupported => {
            ServiceError::UnsupportedCommand(format!("transcription_{rejection:?}").to_lowercase())
        }
        TranscriptionPreflightFailure::Timeout | TranscriptionPreflightFailure::Cancellation => {
            ServiceError::AdapterFailure(format!("transcription_{rejection:?}").to_lowercase())
        }
    }
}
