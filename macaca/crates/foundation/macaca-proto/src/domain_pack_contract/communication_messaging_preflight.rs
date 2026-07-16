use serde::{Deserialize, Serialize};

use super::communication_messaging::{
    communication_messaging_pack_definition, MessagingSendMessageCommand,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Evaluated messaging policy facts that deliberately exclude raw message data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingAdmissionEvidence {
    pub sender_verified: bool,
    pub participant_channel_allowed: bool,
    pub recipient_consent_granted: bool,
    pub external_recipient_approved: bool,
    pub message_within_limit: bool,
    pub format_supported: bool,
    pub attachments_within_limit: bool,
    pub rate_limit_available: bool,
    pub event_signature_valid: bool,
    pub idempotency_available: bool,
    pub provider_capability_available: bool,
}

/// A provider-neutral Specification for messaging side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
    max_attachments: usize,
    max_attachment_bytes: u64,
}

impl MessagingDispatchPreflight {
    /// Build the gate from descriptor-owned scope and command metadata.
    pub fn new(
        max_attachments: usize,
        max_attachment_bytes: u64,
        approval_required_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &communication_messaging_pack_definition(),
                approval_required_commands,
            ),
            max_attachments,
            max_attachment_bytes,
        }
    }

    /// Validate a send request before a messaging provider can create side effects.
    pub fn evaluate_send(
        &self,
        command: &MessagingSendMessageCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &MessagingAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_admission_preconditions(self.max_attachments, self.max_attachment_bytes) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "messaging_send_invalid",
            ));
        }
        for (allowed, status, reason) in evidence_checks(evidence) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Execute a provider closure only after all communication safety checks pass.
    pub fn dispatch_send<T>(
        &self,
        command: &MessagingSendMessageCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &MessagingAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate_send(command, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn evidence_checks(
    evidence: &MessagingAdmissionEvidence,
) -> [(bool, DomainPackPreflightStatus, &'static str); 11] {
    [
        (
            evidence.sender_verified,
            DomainPackPreflightStatus::Denied,
            "messaging_sender_unverified",
        ),
        (
            evidence.participant_channel_allowed,
            DomainPackPreflightStatus::Denied,
            "messaging_participant_channel_denied",
        ),
        (
            evidence.recipient_consent_granted,
            DomainPackPreflightStatus::Denied,
            "messaging_consent_required",
        ),
        (
            evidence.external_recipient_approved,
            DomainPackPreflightStatus::Denied,
            "messaging_external_approval_required",
        ),
        (
            evidence.message_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "messaging_message_limit_exceeded",
        ),
        (
            evidence.format_supported,
            DomainPackPreflightStatus::Unsupported,
            "messaging_unsupported_format",
        ),
        (
            evidence.attachments_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "messaging_attachment_too_large",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "messaging_rate_limited",
        ),
        (
            evidence.event_signature_valid,
            DomainPackPreflightStatus::Denied,
            "messaging_event_signature_invalid",
        ),
        (
            evidence.idempotency_available,
            DomainPackPreflightStatus::Conflict,
            "messaging_idempotency_conflict",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "messaging_provider_rejected",
        ),
    ]
}

fn reject(status: DomainPackPreflightStatus, reason_code: &str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
