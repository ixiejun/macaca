//! Runtime-host Strategy for provider-neutral media-audio commands.
//!
//! This deterministic provider proves canonical dispatch without reading PCM,
//! prompts, voice data, audio files, or provider payloads. Production adapters
//! are selected only by the runtime-host composition root.

use std::collections::{BTreeMap, BTreeSet};

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

/// Bounded audit/replay event that deliberately excludes all media payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Mock and unavailable audio-provider Strategy selected by runtime-host.
pub struct MediaAudioSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: tokio::sync::broadcast::Sender<AudioRuntimeEvent>,
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
        }
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
    pub fn snapshot(&self) -> BTreeMap<String, String> {
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
            ("snapshot_schema".into(), "media.audio.replay.v1".into()),
        ]);
        self.emit("audio.snapshot", "snapshot:provider", "snapshot_recorded");
        snapshot
    }

    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(AudioRuntimeEvent {
            command: command.into(),
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
        if let Err(rejection) = admit_audio_operation(operation, preflight_facts(&command)) {
            self.emit(operation, &trace.trace_id, "preflight_rejected");
            return Err(preflight_error(rejection));
        }
        self.emit(operation, &trace.trace_id, "completed");
        info!(service_id = MEDIA_AUDIO_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "media audio command completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"metadata_only","operation":operation,"artifact_ref":format!("artifact:audio:{}", trace.trace_id)}),
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

/// Read bounded host-issued evidence only; raw command payloads are never inspected.
fn preflight_facts(command: &ServiceCommand) -> AudioPreflightFacts {
    let enabled = |key: &str, default: bool| {
        command
            .metadata
            .get(key)
            .map_or(default, |value| value == "true")
    };
    AudioPreflightFacts {
        permission_granted: enabled("permission_granted", true),
        provider_available: enabled("provider_available", true),
        scope_granted: enabled("scope_granted", true),
        approval_granted: enabled("approval_granted", true),
        approval_required: enabled("approval_required", false),
        requested_units: command
            .metadata
            .get("requested_units")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1),
        reserved_units: command
            .metadata
            .get("reserved_units")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1),
    }
}

fn preflight_error(rejection: AudioPreflightFailure) -> ServiceError {
    match rejection {
        AudioPreflightFailure::Denied | AudioPreflightFailure::ApprovalRequired => {
            ServiceError::DisabledByPolicy(format!("audio_{rejection:?}").to_lowercase())
        }
        AudioPreflightFailure::Unavailable => {
            ServiceError::ServiceUnavailable("audio_provider_unavailable".into())
        }
        AudioPreflightFailure::QuotaExceeded => {
            ServiceError::AdapterFailure("audio_quota_exceeded".into())
        }
    }
}
