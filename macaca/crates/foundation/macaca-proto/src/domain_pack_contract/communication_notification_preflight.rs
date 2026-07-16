use serde::{Deserialize, Serialize};

use super::communication_notification::{
    communication_notification_pack_definition, NotificationPublishCommand,
    NotificationScheduleCommand,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Host-owned notification evidence evaluated before a provider is allowed to send.
///
/// Each value is a bounded decision outcome, never a raw consent record, target,
/// push token, endpoint, credential, notification payload, or provider response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAdmissionEvidence {
    pub consent_granted: bool,
    pub host_supported: bool,
    pub provider_healthy: bool,
    pub target_scope_granted: bool,
    pub content_sensitivity_allowed: bool,
    pub payload_within_limit: bool,
    pub channel_allowed: bool,
    pub rate_limit_available: bool,
    pub resource_budget_reserved: bool,
    pub entitlement_granted: bool,
    pub action_count: u32,
    pub max_action_count: u32,
    pub schedule_horizon_within_limit: bool,
}

/// Notification-specific Specification layered over the generic pack preflight.
///
/// The contract deliberately keeps policy and authorization facts outside the
/// descriptor and provider. Callers supply only evaluated booleans; this layer
/// combines them with DTO validation and invokes providers only after acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl NotificationDispatchPreflight {
    /// Create a preflight gate whose descriptor remains the source of allowed commands and scopes.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &communication_notification_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Validate an immediate publish request and all host-supplied admission facts.
    pub fn evaluate_publish(
        &self,
        command: &NotificationPublishCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &NotificationAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_admission_preconditions() {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_validation_failed",
            ));
        }
        self.evaluate_evidence(evidence)
    }

    /// Validate a scheduled publish request without relying on wall-clock time.
    pub fn evaluate_schedule(
        &self,
        command: &NotificationScheduleCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &NotificationAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_schedule_preconditions() || !evidence.schedule_horizon_within_limit {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_schedule_invalid",
            ));
        }
        self.evaluate_evidence(evidence)
    }

    /// Dispatch only when notification DTOs and every policy/host gate have accepted.
    pub fn dispatch_publish<T>(
        &self,
        command: &NotificationPublishCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &NotificationAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate_publish(command, preflight, evidence)?;
        Ok(dispatch())
    }

    fn evaluate_evidence(
        &self,
        evidence: &NotificationAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        if !evidence.consent_granted {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_consent_denied",
            ));
        }
        if !evidence.host_supported {
            return Err(reject(
                DomainPackPreflightStatus::Unavailable,
                "notification_host_unsupported",
            ));
        }
        if !evidence.provider_healthy {
            return Err(reject(
                DomainPackPreflightStatus::Unavailable,
                "notification_provider_unhealthy",
            ));
        }
        if !evidence.target_scope_granted {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_target_scope_denied",
            ));
        }
        if !evidence.content_sensitivity_allowed {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_content_sensitivity_denied",
            ));
        }
        if !evidence.payload_within_limit {
            return Err(reject(
                DomainPackPreflightStatus::QuotaExceeded,
                "notification_payload_limit_exceeded",
            ));
        }
        if !evidence.channel_allowed {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_channel_denied",
            ));
        }
        if !evidence.rate_limit_available {
            return Err(reject(
                DomainPackPreflightStatus::QuotaExceeded,
                "notification_rate_limited",
            ));
        }
        if !evidence.resource_budget_reserved {
            return Err(reject(
                DomainPackPreflightStatus::QuotaExceeded,
                "notification_resource_unreserved",
            ));
        }
        if !evidence.entitlement_granted {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_entitlement_denied",
            ));
        }
        if evidence.action_count > evidence.max_action_count {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "notification_action_limit_exceeded",
            ));
        }
        Ok(())
    }
}

fn reject(status: DomainPackPreflightStatus, reason_code: &str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
