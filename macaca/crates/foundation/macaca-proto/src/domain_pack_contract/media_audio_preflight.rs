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
        let facts = AudioPreflightFacts {
            permission_granted: true,
            provider_available: true,
            scope_granted: true,
            approval_granted: false,
            approval_required: false,
            requested_units: 1,
            reserved_units: 1,
        };
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
                    ..facts
                }
            ),
            Err(AudioPreflightFailure::ApprovalRequired)
        );
    }
}
