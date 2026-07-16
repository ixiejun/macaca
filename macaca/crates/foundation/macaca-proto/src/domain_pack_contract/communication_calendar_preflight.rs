use serde::{Deserialize, Serialize};

use super::communication_calendar::{
    communication_calendar_pack_definition, CalendarCreateEventCommand,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Evaluated calendar admission facts supplied by host policy and resource services.
///
/// The contract carries no raw event descriptions, credentials, external invite
/// addresses, conference secrets, iCalendar data, provider responses, or clocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarAdmissionEvidence {
    pub source_owned_by_caller: bool,
    pub credential_secret_reference_valid: bool,
    pub timezone_valid: bool,
    pub recurrence_within_limit: bool,
    pub idempotency_available: bool,
    pub conflict_policy_allows_write: bool,
    pub external_invite_approved: bool,
    pub availability_privacy_allowed: bool,
    pub import_export_within_limit: bool,
    pub provider_capability_available: bool,
    pub rate_limit_available: bool,
    pub timeout_within_limit: bool,
    pub resource_budget_reserved: bool,
}

/// Calendar-specific Specification layered over descriptor command admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
    max_attendees: usize,
    max_recurrence: u32,
}

impl CalendarDispatchPreflight {
    /// Build a gate from descriptor allowlists and host policy limits.
    pub fn new(
        max_attendees: usize,
        max_recurrence: u32,
        approval_required_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &communication_calendar_pack_definition(),
                approval_required_commands,
            ),
            max_attendees,
            max_recurrence,
        }
    }

    /// Validate a create-event request before a calendar provider can mutate state.
    pub fn evaluate_create(
        &self,
        command: &CalendarCreateEventCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &CalendarAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_admission_preconditions(self.max_attendees, self.max_recurrence) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "calendar_validation_failed",
            ));
        }
        for (allowed, status, reason) in evidence_checks(evidence) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke a provider closure only after calendar admission accepts the request.
    pub fn dispatch_create<T>(
        &self,
        command: &CalendarCreateEventCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &CalendarAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate_create(command, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn evidence_checks(
    evidence: &CalendarAdmissionEvidence,
) -> [(bool, DomainPackPreflightStatus, &'static str); 13] {
    [
        (
            evidence.source_owned_by_caller,
            DomainPackPreflightStatus::Denied,
            "calendar_source_ownership_denied",
        ),
        (
            evidence.credential_secret_reference_valid,
            DomainPackPreflightStatus::Denied,
            "calendar_credential_reference_invalid",
        ),
        (
            evidence.timezone_valid,
            DomainPackPreflightStatus::Denied,
            "calendar_timezone_invalid",
        ),
        (
            evidence.recurrence_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "calendar_recurrence_limit_exceeded",
        ),
        (
            evidence.idempotency_available,
            DomainPackPreflightStatus::Conflict,
            "calendar_idempotency_conflict",
        ),
        (
            evidence.conflict_policy_allows_write,
            DomainPackPreflightStatus::Conflict,
            "calendar_write_conflict",
        ),
        (
            evidence.external_invite_approved,
            DomainPackPreflightStatus::Denied,
            "calendar_external_invite_approval_required",
        ),
        (
            evidence.availability_privacy_allowed,
            DomainPackPreflightStatus::Denied,
            "calendar_availability_privacy_denied",
        ),
        (
            evidence.import_export_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "calendar_import_export_limit_exceeded",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "calendar_provider_capability_unsupported",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "calendar_rate_limited",
        ),
        (
            evidence.timeout_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "calendar_timeout_budget_exceeded",
        ),
        (
            evidence.resource_budget_reserved,
            DomainPackPreflightStatus::QuotaExceeded,
            "calendar_resource_unreserved",
        ),
    ]
}

fn reject(status: DomainPackPreflightStatus, reason_code: &str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
