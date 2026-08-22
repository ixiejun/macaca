//! Runtime-host Strategy for provider-neutral media-audio commands.
//!
//! This deterministic provider proves canonical dispatch without reading PCM,
//! prompts, voice data, audio files, or provider payloads. Production adapters
//! are selected only by the runtime-host composition root.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::media_audio::{
    AudioProviderCapability, MEDIA_AUDIO_COMMANDS, MEDIA_AUDIO_PACK_ID, MEDIA_AUDIO_SERVICE_ID,
};
use macaca_proto::{
    admit_audio_operation, domain_pack_command_trace, domain_pack_service_result,
    AudioPreflightFacts, AudioPreflightFailure, DomainPackProviderCapabilityState, KernelServiceId,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceType, TraceSchemaRef,
};
use tracing::{info, warn};

use crate::media_audio_service_state::AudioSideEffectLedger;

/// Bounded audit/replay event that deliberately excludes all media payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRuntimeEvent {
    pub command: String,
    /// Stable provider-neutral audit name; PCM, prompts, voices, and payloads are omitted.
    pub event_name: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Mock and unavailable audio-provider Strategy selected by runtime-host.
pub struct MediaAudioSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: tokio::sync::broadcast::Sender<AudioRuntimeEvent>,
    side_effects: Arc<AudioSideEffectLedger>,
    admission_facts: AudioPreflightFacts,
}

impl MediaAudioSystemServiceProvider {
    /// Build a deterministic metadata-only provider for contract and ABI tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a fail-closed Null Object when no audio module is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            unavailable_reason,
            events,
            side_effects: Arc::new(AudioSideEffectLedger::default()),
            admission_facts: AudioPreflightFacts::permissive(),
        }
    }

    /// Replace preview evidence with host-issued admission facts.
    ///
    /// This is a composition-root seam for policy, entitlement, resource,
    /// approval, and capability decorators. It intentionally cannot inspect or
    /// derive authorization from caller-provided command metadata or payloads.
    pub fn with_admission_facts(mut self, admission_facts: AudioPreflightFacts) -> Self {
        self.admission_facts = admission_facts;
        self
    }

    /// Report descriptor-owned capability facts, never engine or voice internals.
    pub fn capability(&self) -> AudioProviderCapability {
        AudioProviderCapability {
            provider_class: if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
            codecs: BTreeSet::from(["pcm".into()]),
            containers: BTreeSet::from(["wav".into()]),
            features: if self.unavailable_reason.is_some() {
                BTreeSet::new()
            } else {
                BTreeSet::from(["metadata_only".into(), "planning".into()])
            },
            max_duration_ms: 300_000,
            state: if self.unavailable_reason.is_some() {
                DomainPackProviderCapabilityState::Unavailable
            } else {
                DomainPackProviderCapabilityState::Preview
            },
        }
    }

    /// Subscribe to bounded events for audit and replay conformance checks.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AudioRuntimeEvent> {
        self.events.subscribe()
    }

    /// Produce a bounded Memento for restart diagnostics.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let snapshot = BTreeMap::from([
            ("provider_class".into(), self.capability().provider_class),
            (
                "capability_state".into(),
                format!("{:?}", self.capability().state),
            ),
            (
                "command_count".into(),
                MEDIA_AUDIO_COMMANDS.len().to_string(),
            ),
            (
                "idempotency_completion_count".into(),
                self.side_effects.completion_count().await.to_string(),
            ),
            ("snapshot_schema".into(), "media.audio.replay.v1".into()),
        ]);
        self.emit("audio.snapshot", "snapshot:provider", "snapshot_recorded");
        snapshot
    }

    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(AudioRuntimeEvent {
            command: command.into(),
            event_name: match outcome {
                "unavailable" => "audio.unavailable",
                "unsupported" => "audio.failure",
                _ => audio_audit_event(command),
            }
            .into(),
            trace_id: trace_id.into(),
            replay_ref: format!("replay:audio:{trace_id}"),
            outcome,
        });
    }
}

#[async_trait]
impl SystemService for MediaAudioSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(MEDIA_AUDIO_SERVICE_ID),
            ServiceType::new("media.audio"),
            TraceSchemaRef::new("media.audio.replay.v1"),
        );
        descriptor
            .metadata
            .insert("pack_id".into(), MEDIA_AUDIO_PACK_ID.into());
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor.metadata.insert(
            "command_count".into(),
            MEDIA_AUDIO_COMMANDS.len().to_string(),
        );
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        self.emit(
            "audio.declaration",
            "declaration:audio-provider",
            "declared",
        );
        info!(
            service_id = MEDIA_AUDIO_SERVICE_ID,
            "media audio provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !MEDIA_AUDIO_COMMANDS.contains(&operation) {
            self.emit(operation, &trace.trace_id, "unsupported");
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit(operation, &trace.trace_id, "unavailable");
            warn!(service_id = MEDIA_AUDIO_SERVICE_ID, command = operation, trace_id = %trace.trace_id, reason_code = %reason, "media audio provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if let Err(rejection) = admit_audio_operation(operation, self.admission_facts) {
            self.emit(operation, &trace.trace_id, "preflight_rejected");
            warn!(service_id = MEDIA_AUDIO_SERVICE_ID, command = operation, trace_id = %trace.trace_id, rejection = ?rejection, "media audio command rejected before provider dispatch");
            return Err(preflight_error(rejection));
        }
        if let Some(reason) = audio_payload_denial(operation, &command.payload) {
            self.emit(operation, &trace.trace_id, "payload_rejected");
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if let Err(error) = AudioSideEffectLedger::validate_audio_version(&command) {
            self.emit(operation, &trace.trace_id, "precondition_rejected");
            return Err(error);
        }
        let artifact_ref = self
            .side_effects
            .outcome_ref(&command, format!("artifact:audio:{}", trace.trace_id))
            .await;
        self.emit(operation, &trace.trace_id, "completed");
        for audit in [
            "audio.admission",
            "audio.policy",
            "audio.entitlement",
            "audio.resource",
            "audio.approval",
        ] {
            self.emit(audit, &trace.trace_id, "validated");
        }
        info!(service_id = MEDIA_AUDIO_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "media audio command completed");
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
        self.emit("audio.health", "health:audio-provider", "reported");
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

/// Evaluate bounded policy facts before audio side-effect allocation.
fn audio_payload_denial(operation: &str, payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("scope_denied", "scope_denied")
        .or_else(|| blocked("format_unsupported", "format_unsupported"))
        .or_else(|| blocked("codec_unsupported", "codec_unsupported"))
        .or_else(|| blocked("metadata_denied", "metadata_denied"))
        .or_else(|| blocked("voice_denied", "voice_consent_denied"))
        .or_else(|| blocked("prompt_denied", "prompt_redaction_denied"))
        .or_else(|| blocked("synthesis_denied", "synthesis_denied"))
        .or_else(|| blocked("export_denied", "export_denied"))
        .or_else(|| blocked("approval_required", "approval_required"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
        .or_else(|| {
            (operation.contains("render")
                && payload
                    .get("duration_ms")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|duration| duration > 300_000))
            .then_some("duration_limit_exceeded")
        })
}

/// Map command and lifecycle markers to stable sanitized audit vocabulary.
fn audio_audit_event(command: &str) -> &'static str {
    match command {
        "audio.declaration" => "audio.pack_declared",
        "audio.snapshot" => "audio.snapshot_recorded",
        "audio.health" => "audio.health",
        "audio.unavailable" => "audio.unavailable",
        "audio.failure" => "audio.failure",
        "audio.admission" => "audio.admission",
        "audio.policy" => "audio.policy",
        "audio.entitlement" => "audio.entitlement",
        "audio.resource" => "audio.resource",
        "audio.approval" => "audio.approval",
        command if command.contains("inspect_provider") => "audio.provider_inspection",
        command if command.contains("import") || command.contains("open_audio") => {
            "audio.import_open"
        }
        command if command.contains("metadata") => "audio.metadata_inspection",
        command if command.contains("waveform") => "audio.waveform_inspection",
        command if command.contains("transcode") => "audio.transcode",
        command if command.contains("segment") => "audio.segment",
        command if command.contains("filter") => "audio.filter",
        command if command.contains("mix") => "audio.mix",
        command if command.contains("synthesis") => "audio.synthesis",
        command if command.contains("export") => "audio.export",
        command if command.contains("artifact") => "audio.artifact_handle",
        _ => "audio.command",
    }
}

fn preflight_error(rejection: AudioPreflightFailure) -> ServiceError {
    match rejection {
        AudioPreflightFailure::Denied
        | AudioPreflightFailure::ApprovalRequired
        | AudioPreflightFailure::PolicyDenied
        | AudioPreflightFailure::EntitlementDenied
        | AudioPreflightFailure::MetadataDenied
        | AudioPreflightFailure::VoiceDenied
        | AudioPreflightFailure::PromptDenied
        | AudioPreflightFailure::SynthesisDenied
        | AudioPreflightFailure::ExportDenied
        | AudioPreflightFailure::WriteDenied
        | AudioPreflightFailure::ArtifactDenied => {
            ServiceError::DisabledByPolicy(format!("audio_{rejection:?}").to_lowercase())
        }
        AudioPreflightFailure::Unavailable => {
            ServiceError::ServiceUnavailable("audio_provider_unavailable".into())
        }
        AudioPreflightFailure::QuotaExceeded => {
            ServiceError::AdapterFailure("audio_quota_exceeded".into())
        }
        AudioPreflightFailure::SchemaMismatch
        | AudioPreflightFailure::FormatUnsupported
        | AudioPreflightFailure::CodecUnsupported => {
            ServiceError::UnsupportedCommand(format!("audio_{rejection:?}").to_lowercase())
        }
        AudioPreflightFailure::Timeout | AudioPreflightFailure::Cancellation => {
            ServiceError::AdapterFailure(format!("audio_{rejection:?}").to_lowercase())
        }
    }
}
