//! Provider-neutral admission and State specifications for host lifecycle calls.
//!
//! Runtime-host policy, presentation, resource, entitlement, and approval
//! decorators provide bounded facts before an adapter may observe a request.

use serde::{Deserialize, Serialize};

/// Payload-free host-issued admission evidence for lifecycle operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecyclePreflightFacts {
    pub permission_granted: bool,
    pub provider_available: bool,
    pub presentation_available: bool,
    pub background_allowed: bool,
    pub dependent_capabilities_allowed: bool,
    pub policy_granted: bool,
    pub entitlement_granted: bool,
    pub approval_granted: bool,
    pub approval_required: bool,
    pub throttled: bool,
    pub suspended: bool,
    pub requested_units: u64,
    pub reserved_units: u64,
    pub within_timeout: bool,
    pub cancellation_requested: bool,
}

/// Structured rejection codes returned before lifecycle adapter dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLifecyclePreflightFailure {
    Denied,
    Unavailable,
    PresentationRequired,
    BackgroundDenied,
    PolicyDenied,
    EntitlementRequired,
    ApprovalRequired,
    Throttled,
    Suspended,
    QuotaExceeded,
    Timeout,
    Cancellation,
}

impl HostLifecyclePreflightFacts {
    /// Construct explicit preview/mock evidence without inspecting command data.
    pub const fn permissive() -> Self {
        Self {
            permission_granted: true,
            provider_available: true,
            presentation_available: true,
            background_allowed: true,
            dependent_capabilities_allowed: true,
            policy_granted: true,
            entitlement_granted: true,
            approval_granted: true,
            approval_required: false,
            throttled: false,
            suspended: false,
            requested_units: 1,
            reserved_units: 1,
            within_timeout: true,
            cancellation_requested: false,
        }
    }
}

/// Evaluate runtime-host evidence before a lifecycle Strategy sees a command.
pub fn admit_host_lifecycle_operation(
    facts: HostLifecyclePreflightFacts,
) -> Result<(), HostLifecyclePreflightFailure> {
    if !facts.permission_granted {
        return Err(HostLifecyclePreflightFailure::Denied);
    }
    if !facts.provider_available {
        return Err(HostLifecyclePreflightFailure::Unavailable);
    }
    if !facts.presentation_available {
        return Err(HostLifecyclePreflightFailure::PresentationRequired);
    }
    if !facts.background_allowed || !facts.dependent_capabilities_allowed {
        return Err(HostLifecyclePreflightFailure::BackgroundDenied);
    }
    if !facts.policy_granted {
        return Err(HostLifecyclePreflightFailure::PolicyDenied);
    }
    if !facts.entitlement_granted {
        return Err(HostLifecyclePreflightFailure::EntitlementRequired);
    }
    if facts.throttled {
        return Err(HostLifecyclePreflightFailure::Throttled);
    }
    if facts.suspended {
        return Err(HostLifecyclePreflightFailure::Suspended);
    }
    if facts.cancellation_requested {
        return Err(HostLifecyclePreflightFailure::Cancellation);
    }
    if !facts.within_timeout {
        return Err(HostLifecyclePreflightFailure::Timeout);
    }
    if facts.reserved_units < facts.requested_units {
        return Err(HostLifecyclePreflightFailure::QuotaExceeded);
    }
    if facts.approval_required && !facts.approval_granted {
        return Err(HostLifecyclePreflightFailure::ApprovalRequired);
    }
    Ok(())
}

/// State shared by foreground sessions and background leases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLifecycleLeaseState {
    Requested,
    Active,
    Throttled,
    Suspended,
    Closing,
    Closed,
    Expired,
    Revoked,
    Failed,
    Unavailable,
}

/// Provider-neutral state transition action for sessions and leases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLifecycleLeaseAction {
    Activate,
    Throttle,
    Suspend,
    Resume,
    Close,
    Expire,
    Revoke,
    Fail,
    Unavailable,
}

/// Apply the State-pattern transition without retaining lifecycle payloads.
pub fn transition_host_lifecycle_lease(
    state: HostLifecycleLeaseState,
    action: HostLifecycleLeaseAction,
) -> Option<HostLifecycleLeaseState> {
    use HostLifecycleLeaseAction::*;
    use HostLifecycleLeaseState::*;
    match (state, action) {
        (Requested, Activate) => Some(Active),
        (Active, Throttle) => Some(Throttled),
        (Active | Throttled, Suspend) => Some(Suspended),
        (Throttled | Suspended, Resume) => Some(Active),
        (Requested | Active | Throttled | Suspended, Close) => Some(Closing),
        (Closing, Close) => Some(Closed),
        (Requested | Active | Throttled | Suspended | Closing, Expire) => Some(Expired),
        (Requested | Active | Throttled | Suspended | Closing, Revoke) => Some(Revoked),
        (Requested | Active | Throttled | Suspended | Closing, Fail) => Some(Failed),
        (Requested, HostLifecycleLeaseAction::Unavailable) => {
            Some(HostLifecycleLeaseState::Unavailable)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_lifecycle_admission_and_transitions_fail_closed() {
        let facts = HostLifecyclePreflightFacts::permissive();
        assert_eq!(
            admit_host_lifecycle_operation(HostLifecyclePreflightFacts {
                throttled: true,
                ..facts
            }),
            Err(HostLifecyclePreflightFailure::Throttled)
        );
        assert_eq!(
            transition_host_lifecycle_lease(
                HostLifecycleLeaseState::Requested,
                HostLifecycleLeaseAction::Activate
            ),
            Some(HostLifecycleLeaseState::Active)
        );
        assert_eq!(
            transition_host_lifecycle_lease(
                HostLifecycleLeaseState::Closed,
                HostLifecycleLeaseAction::Resume
            ),
            None
        );
    }
}
