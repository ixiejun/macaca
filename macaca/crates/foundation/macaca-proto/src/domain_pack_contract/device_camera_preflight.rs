//! Provider-neutral admission and State specifications for camera operations.
//!
//! Runtime-host supplies bounded evidence from authorization, foreground,
//! privacy, policy, approval, entitlement, and resource decorators. No camera
//! frame, media byte, stable identifier, or command payload participates.

use serde::{Deserialize, Serialize};

/// Bounded host-issued evidence required before a camera provider dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraPreflightFacts {
    pub permission_granted: bool,
    pub provider_available: bool,
    pub foreground_active: bool,
    pub privacy_indicator_available: bool,
    pub constraints_valid: bool,
    pub output_intent_allowed: bool,
    pub retention_allowed: bool,
    pub policy_granted: bool,
    pub entitlement_granted: bool,
    pub approval_granted: bool,
    pub approval_required: bool,
    pub requested_units: u64,
    pub reserved_units: u64,
    pub within_timeout: bool,
    pub cancellation_requested: bool,
}

/// Structured failure reason returned before a concrete camera adapter runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraPreflightFailure {
    Denied,
    Unavailable,
    ForegroundRequired,
    PrivacyIndicatorUnavailable,
    ConstraintUnsatisfied,
    PolicyDenied,
    EntitlementDenied,
    ApprovalRequired,
    QuotaExceeded,
    Timeout,
    Cancellation,
}

impl CameraPreflightFacts {
    /// Build preview/mock evidence; production hosts replace it at composition.
    pub const fn permissive() -> Self {
        Self {
            permission_granted: true,
            provider_available: true,
            foreground_active: true,
            privacy_indicator_available: true,
            constraints_valid: true,
            output_intent_allowed: true,
            retention_allowed: true,
            policy_granted: true,
            entitlement_granted: true,
            approval_granted: true,
            approval_required: false,
            requested_units: 1,
            reserved_units: 1,
            within_timeout: true,
            cancellation_requested: false,
        }
    }
}

/// Check all host-issued facts before a camera adapter observes a call.
pub fn admit_camera_operation(facts: CameraPreflightFacts) -> Result<(), CameraPreflightFailure> {
    if !facts.permission_granted {
        return Err(CameraPreflightFailure::Denied);
    }
    if !facts.provider_available {
        return Err(CameraPreflightFailure::Unavailable);
    }
    if !facts.foreground_active {
        return Err(CameraPreflightFailure::ForegroundRequired);
    }
    if !facts.privacy_indicator_available {
        return Err(CameraPreflightFailure::PrivacyIndicatorUnavailable);
    }
    if !facts.constraints_valid || !facts.output_intent_allowed || !facts.retention_allowed {
        return Err(CameraPreflightFailure::ConstraintUnsatisfied);
    }
    if !facts.policy_granted {
        return Err(CameraPreflightFailure::PolicyDenied);
    }
    if !facts.entitlement_granted {
        return Err(CameraPreflightFailure::EntitlementDenied);
    }
    if facts.cancellation_requested {
        return Err(CameraPreflightFailure::Cancellation);
    }
    if !facts.within_timeout {
        return Err(CameraPreflightFailure::Timeout);
    }
    if facts.reserved_units < facts.requested_units {
        return Err(CameraPreflightFailure::QuotaExceeded);
    }
    if facts.approval_required && !facts.approval_granted {
        return Err(CameraPreflightFailure::ApprovalRequired);
    }
    Ok(())
}

/// Explicit session lifecycle states used by all replaceable camera Strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraSessionState {
    Requested,
    Active,
    Paused,
    Stopping,
    Closed,
    Expired,
    Revoked,
    Failed,
    Unavailable,
}

/// Provider-neutral camera-session actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraSessionAction {
    Open,
    Pause,
    Resume,
    Stop,
    Close,
    Expire,
    Revoke,
    Fail,
    Unavailable,
}

/// Transition a camera session without carrying output or provider data.
pub fn transition_camera_session(
    state: CameraSessionState,
    action: CameraSessionAction,
) -> Option<CameraSessionState> {
    use CameraSessionAction::*;
    use CameraSessionState::*;
    match (state, action) {
        (Requested, Open) => Some(Active),
        (Active, Pause) => Some(Paused),
        (Paused, Resume) => Some(Active),
        (Active | Paused, Stop) => Some(Stopping),
        (Stopping, Close) | (Active | Paused, Close) => Some(Closed),
        (Requested | Active | Paused | Stopping, Expire) => Some(Expired),
        (Requested | Active | Paused | Stopping, Revoke) => Some(Revoked),
        (Requested | Active | Paused | Stopping, Fail) => Some(Failed),
        (Requested, CameraSessionAction::Unavailable) => Some(CameraSessionState::Unavailable),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_admission_and_session_transitions_fail_closed() {
        let facts = CameraPreflightFacts::permissive();
        assert_eq!(
            admit_camera_operation(CameraPreflightFacts {
                foreground_active: false,
                ..facts
            }),
            Err(CameraPreflightFailure::ForegroundRequired)
        );
        assert_eq!(
            transition_camera_session(CameraSessionState::Requested, CameraSessionAction::Open),
            Some(CameraSessionState::Active)
        );
        assert_eq!(
            transition_camera_session(CameraSessionState::Closed, CameraSessionAction::Resume),
            None
        );
    }
}
