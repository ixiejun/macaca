//! Provider-neutral preflight Specification for sensitive transcription calls.
//!
//! Facts are host-issued, bounded, and payload-free. Policy, entitlement,
//! resource, and approval services populate them before an adapter sees work.

use serde::{Deserialize, Serialize};

/// Sanitized transcription admission evidence from runtime-host decorators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionPreflightFacts {
    pub permission_granted: bool,
    pub provider_available: bool,
    pub source_consent_granted: bool,
    pub scope_granted: bool,
    pub policy_granted: bool,
    pub entitlement_granted: bool,
    pub schema_valid: bool,
    pub format_supported: bool,
    pub language_supported: bool,
    pub model_supported: bool,
    pub diarization_supported: bool,
    pub timestamp_supported: bool,
    pub redaction_allowed: bool,
    pub translation_allowed: bool,
    pub export_allowed: bool,
    pub artifact_allowed: bool,
    pub within_timeout: bool,
    pub cancellation_requested: bool,
    pub approval_granted: bool,
    pub approval_required: bool,
    pub requested_units: u64,
    pub reserved_units: u64,
}

/// Structured fail-closed outcomes emitted before provider dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionPreflightFailure {
    Denied,
    Unavailable,
    QuotaExceeded,
    ApprovalRequired,
    PolicyDenied,
    EntitlementDenied,
    SchemaMismatch,
    FormatUnsupported,
    LanguageUnsupported,
    ModelUnsupported,
    DiarizationUnsupported,
    TimestampUnsupported,
    RedactionDenied,
    TranslationDenied,
    ExportDenied,
    ArtifactDenied,
    Timeout,
    Cancellation,
}

impl TranscriptionPreflightFacts {
    /// Construct preview/mock evidence without examining request metadata or payloads.
    pub const fn permissive() -> Self {
        Self {
            permission_granted: true,
            provider_available: true,
            source_consent_granted: true,
            scope_granted: true,
            policy_granted: true,
            entitlement_granted: true,
            schema_valid: true,
            format_supported: true,
            language_supported: true,
            model_supported: true,
            diarization_supported: true,
            timestamp_supported: true,
            redaction_allowed: true,
            translation_allowed: true,
            export_allowed: true,
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

/// Evaluate host-issued admission evidence before a transcription adapter runs.
pub fn admit_transcription_operation(
    _command: &str,
    facts: TranscriptionPreflightFacts,
) -> Result<(), TranscriptionPreflightFailure> {
    if !facts.permission_granted || !facts.source_consent_granted || !facts.scope_granted {
        return Err(TranscriptionPreflightFailure::Denied);
    }
    if !facts.provider_available {
        return Err(TranscriptionPreflightFailure::Unavailable);
    }
    if !facts.policy_granted {
        return Err(TranscriptionPreflightFailure::PolicyDenied);
    }
    if !facts.entitlement_granted {
        return Err(TranscriptionPreflightFailure::EntitlementDenied);
    }
    if !facts.schema_valid {
        return Err(TranscriptionPreflightFailure::SchemaMismatch);
    }
    if !facts.format_supported {
        return Err(TranscriptionPreflightFailure::FormatUnsupported);
    }
    if !facts.language_supported {
        return Err(TranscriptionPreflightFailure::LanguageUnsupported);
    }
    if !facts.model_supported {
        return Err(TranscriptionPreflightFailure::ModelUnsupported);
    }
    if !facts.diarization_supported {
        return Err(TranscriptionPreflightFailure::DiarizationUnsupported);
    }
    if !facts.timestamp_supported {
        return Err(TranscriptionPreflightFailure::TimestampUnsupported);
    }
    if !facts.redaction_allowed {
        return Err(TranscriptionPreflightFailure::RedactionDenied);
    }
    if !facts.translation_allowed {
        return Err(TranscriptionPreflightFailure::TranslationDenied);
    }
    if !facts.export_allowed {
        return Err(TranscriptionPreflightFailure::ExportDenied);
    }
    if !facts.artifact_allowed {
        return Err(TranscriptionPreflightFailure::ArtifactDenied);
    }
    if facts.cancellation_requested {
        return Err(TranscriptionPreflightFailure::Cancellation);
    }
    if !facts.within_timeout {
        return Err(TranscriptionPreflightFailure::Timeout);
    }
    if facts.reserved_units < facts.requested_units {
        return Err(TranscriptionPreflightFailure::QuotaExceeded);
    }
    if facts.approval_required && !facts.approval_granted {
        return Err(TranscriptionPreflightFailure::ApprovalRequired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transcription_preflight_rejects_before_provider_dispatch() {
        let facts = TranscriptionPreflightFacts::permissive();
        assert_eq!(
            admit_transcription_operation(
                "transcription.batch_request",
                TranscriptionPreflightFacts {
                    permission_granted: false,
                    ..facts
                }
            ),
            Err(TranscriptionPreflightFailure::Denied)
        );
        assert_eq!(
            admit_transcription_operation(
                "transcription.batch_request",
                TranscriptionPreflightFacts {
                    language_supported: false,
                    ..facts
                }
            ),
            Err(TranscriptionPreflightFailure::LanguageUnsupported)
        );
        assert_eq!(
            admit_transcription_operation(
                "transcription.batch_request",
                TranscriptionPreflightFacts {
                    reserved_units: 0,
                    ..facts
                }
            ),
            Err(TranscriptionPreflightFailure::QuotaExceeded)
        );
        assert_eq!(
            admit_transcription_operation(
                "transcription.batch_request",
                TranscriptionPreflightFacts {
                    approval_required: true,
                    approval_granted: false,
                    ..facts
                }
            ),
            Err(TranscriptionPreflightFailure::ApprovalRequired)
        );
    }
}
