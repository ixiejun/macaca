//! Provider-neutral preflight Specification for media-audio operations.
//!
//! The facts are host-issued, bounded, and payload-free so no raw audio, prompt,
//! voice biometric, or provider-native data crosses the protocol boundary.

use serde::{Deserialize, Serialize};

/// Sanitized admission evidence gathered before an audio adapter is dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPreflightFacts {
    pub permission_granted: bool,
    pub provider_available: bool,
    pub scope_granted: bool,
    pub policy_granted: bool,
    pub entitlement_granted: bool,
    pub schema_valid: bool,
    pub format_supported: bool,
    pub codec_supported: bool,
    pub metadata_allowed: bool,
    pub voice_allowed: bool,
    pub prompt_allowed: bool,
    pub synthesis_allowed: bool,
    pub export_allowed: bool,
    pub write_allowed: bool,
    pub artifact_allowed: bool,
    pub within_timeout: bool,
    pub cancellation_requested: bool,
    pub approval_granted: bool,
    pub approval_required: bool,
    pub requested_units: u64,
    pub reserved_units: u64,
}

/// Fail-closed reasons returned before a concrete audio adapter can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioPreflightFailure {
    Denied,
    Unavailable,
    QuotaExceeded,
    ApprovalRequired,
    PolicyDenied,
    EntitlementDenied,
    SchemaMismatch,
    FormatUnsupported,
    CodecUnsupported,
    MetadataDenied,
    VoiceDenied,
    PromptDenied,
    SynthesisDenied,
    ExportDenied,
    WriteDenied,
    ArtifactDenied,
    Timeout,
    Cancellation,
}

impl AudioPreflightFacts {
    /// Build permissive test or preview evidence without inspecting command payloads.
    ///
    /// Production composition should obtain each fact from dedicated policy,
    /// entitlement, resource, approval, and capability services before dispatch.
    pub const fn permissive() -> Self {
        Self {
            permission_granted: true,
            provider_available: true,
            scope_granted: true,
            policy_granted: true,
            entitlement_granted: true,
            schema_valid: true,
            format_supported: true,
            codec_supported: true,
            metadata_allowed: true,
            voice_allowed: true,
            prompt_allowed: true,
            synthesis_allowed: true,
            export_allowed: true,
            write_allowed: true,
            artifact_allowed: true,
            within_timeout: true,
            cancellation_requested: false,
            approval_granted: true,
            approval_required: false,
            requested_units: 1,
            reserved_units: 1,
        }
    }
}

/// Evaluate permission, scope, entitlement/resource, and approval evidence.
pub fn admit_audio_operation(
    _command: &str,
    facts: AudioPreflightFacts,
) -> Result<(), AudioPreflightFailure> {
    if !facts.permission_granted || !facts.scope_granted {
        return Err(AudioPreflightFailure::Denied);
    }
    if !facts.provider_available {
        return Err(AudioPreflightFailure::Unavailable);
    }
    if !facts.policy_granted {
        return Err(AudioPreflightFailure::PolicyDenied);
    }
    if !facts.entitlement_granted {
        return Err(AudioPreflightFailure::EntitlementDenied);
    }
    if !facts.schema_valid {
        return Err(AudioPreflightFailure::SchemaMismatch);
    }
    if !facts.format_supported {
        return Err(AudioPreflightFailure::FormatUnsupported);
    }
    if !facts.codec_supported {
        return Err(AudioPreflightFailure::CodecUnsupported);
    }
    if !facts.metadata_allowed {
        return Err(AudioPreflightFailure::MetadataDenied);
    }
    if !facts.voice_allowed {
        return Err(AudioPreflightFailure::VoiceDenied);
    }
    if !facts.prompt_allowed {
        return Err(AudioPreflightFailure::PromptDenied);
    }
    if !facts.synthesis_allowed {
        return Err(AudioPreflightFailure::SynthesisDenied);
    }
    if !facts.export_allowed {
        return Err(AudioPreflightFailure::ExportDenied);
    }
    if !facts.write_allowed {
        return Err(AudioPreflightFailure::WriteDenied);
    }
    if !facts.artifact_allowed {
        return Err(AudioPreflightFailure::ArtifactDenied);
    }
    if facts.cancellation_requested {
        return Err(AudioPreflightFailure::Cancellation);
    }
    if !facts.within_timeout {
        return Err(AudioPreflightFailure::Timeout);
    }
    if facts.reserved_units < facts.requested_units {
        return Err(AudioPreflightFailure::QuotaExceeded);
    }
    if facts.approval_required && !facts.approval_granted {
        return Err(AudioPreflightFailure::ApprovalRequired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_preflight_rejects_before_adapter_dispatch() {
        let facts = AudioPreflightFacts::permissive();
        assert_eq!(
            admit_audio_operation(
                "audio.open_audio",
                AudioPreflightFacts {
                    permission_granted: false,
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::Denied)
        );
        assert_eq!(
            admit_audio_operation(
                "audio.open_audio",
                AudioPreflightFacts {
                    codec_supported: false,
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::CodecUnsupported)
        );
        assert_eq!(
            admit_audio_operation(
                "audio.open_audio",
                AudioPreflightFacts {
                    provider_available: false,
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::Unavailable)
        );
        assert_eq!(
            admit_audio_operation(
                "audio.open_audio",
                AudioPreflightFacts {
                    reserved_units: 0,
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::QuotaExceeded)
        );
        assert_eq!(
            admit_audio_operation(
                "audio.export_request",
                AudioPreflightFacts {
                    approval_required: true,
                    approval_granted: false,
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::ApprovalRequired)
        );
    }
}
