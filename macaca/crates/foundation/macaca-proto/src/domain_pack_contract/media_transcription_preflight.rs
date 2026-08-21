//! Provider-neutral preflight Specification for sensitive transcription calls.
//!
//! Every fact is bounded and payload-free. Runtime-host obtains these facts from
//! policy, entitlement, resource, and approval services before dispatching a
//! command to a transcription adapter.

use serde::{Deserialize, Serialize};

/// Sanitized inputs for transcription command admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionPreflightFacts {
    pub permission_granted: bool,
    pub provider_available: bool,
    pub source_consent_granted: bool,
    pub approval_granted: bool,
    pub sensitive_source: bool,
    pub requested_units: u64,
    pub reserved_units: u64,
}

/// Structured fail-closed reasons returned before media-provider dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionPreflightFailure {
    Denied,
    Unavailable,
    QuotaExceeded,
    ApprovalRequired,
}

/// Evaluate policy, consent, resource, and approval facts before an adapter sees a command.
pub fn admit_transcription_operation(
    command: &str,
    facts: TranscriptionPreflightFacts,
) -> Result<(), TranscriptionPreflightFailure> {
    if !facts.permission_granted || !facts.source_consent_granted {
        return Err(TranscriptionPreflightFailure::Denied);
    }
    if !facts.provider_available {
        return Err(TranscriptionPreflightFailure::Unavailable);
    }
    if facts.reserved_units < facts.requested_units {
        return Err(TranscriptionPreflightFailure::QuotaExceeded);
    }
    let approval_required = facts.sensitive_source
        || matches!(
            command,
            "transcription.subtitle_export_request" | "transcription.translation_handoff_request"
        );
    if approval_required && !facts.approval_granted {
        return Err(TranscriptionPreflightFailure::ApprovalRequired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> TranscriptionPreflightFacts {
        TranscriptionPreflightFacts {
            permission_granted: true,
            provider_available: true,
            source_consent_granted: true,
            approval_granted: false,
            sensitive_source: false,
            requested_units: 1,
            reserved_units: 1,
        }
    }

    #[test]
    fn transcription_preflight_rejects_before_provider_dispatch() {
        let mut denied = facts();
        denied.permission_granted = false;
        assert_eq!(
            admit_transcription_operation("transcription.batch_request", denied),
            Err(TranscriptionPreflightFailure::Denied)
        );
        let mut unavailable = facts();
        unavailable.provider_available = false;
        assert_eq!(
            admit_transcription_operation("transcription.batch_request", unavailable),
            Err(TranscriptionPreflightFailure::Unavailable)
        );
        let mut quota = facts();
        quota.reserved_units = 0;
        assert_eq!(
            admit_transcription_operation("transcription.batch_request", quota),
            Err(TranscriptionPreflightFailure::QuotaExceeded)
        );
        assert_eq!(
            admit_transcription_operation("transcription.subtitle_export_request", facts()),
            Err(TranscriptionPreflightFailure::ApprovalRequired)
        );
    }
}
