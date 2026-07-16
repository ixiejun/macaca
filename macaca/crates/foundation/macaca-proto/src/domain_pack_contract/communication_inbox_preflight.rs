use serde::{Deserialize, Serialize};

use super::communication_inbox::{communication_inbox_pack_definition, InboxFetchBodyCommand};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Source-bound inbox facts resolved by policy, identity, and resource services.
///
/// These flags intentionally contain only evaluated outcomes. Credentials,
/// webhook secrets, bodies, attachments, source payloads, and provider data
/// remain outside the contract and are never eligible for trace serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxAdmissionEvidence {
    pub source_owned_by_caller: bool,
    pub credential_secret_reference_valid: bool,
    pub webhook_secret_reference_valid: bool,
    pub provider_available: bool,
    pub command_capability_available: bool,
    pub rate_limit_available: bool,
    pub timeout_within_limit: bool,
    pub page_size_within_limit: bool,
    pub storage_budget_reserved: bool,
    pub body_redaction_allowed: bool,
    pub attachment_redaction_allowed: bool,
}

/// Provider-neutral inbox gate that composes descriptor and source admission checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
    max_body_bytes: u64,
}

impl InboxDispatchPreflight {
    /// Construct the specification from descriptor-owned command and scope allowlists.
    pub fn new(
        max_body_bytes: u64,
        approval_required_commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &communication_inbox_pack_definition(),
                approval_required_commands,
            ),
            max_body_bytes,
        }
    }

    /// Validate a body fetch before a source provider can retrieve any content.
    pub fn evaluate_body_fetch(
        &self,
        command: &InboxFetchBodyCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &InboxAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_bounded_fetch(self.max_body_bytes) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "inbox_body_fetch_invalid",
            ));
        }
        self.evaluate_evidence(evidence)
    }

    /// Dispatch a source operation only after every admission gate has accepted.
    pub fn dispatch_body_fetch<T>(
        &self,
        command: &InboxFetchBodyCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &InboxAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate_body_fetch(command, preflight, evidence)?;
        Ok(dispatch())
    }

    fn evaluate_evidence(
        &self,
        evidence: &InboxAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        let checks = [
            (
                evidence.source_owned_by_caller,
                DomainPackPreflightStatus::Denied,
                "inbox_source_ownership_denied",
            ),
            (
                evidence.credential_secret_reference_valid,
                DomainPackPreflightStatus::Denied,
                "inbox_credential_reference_invalid",
            ),
            (
                evidence.webhook_secret_reference_valid,
                DomainPackPreflightStatus::Denied,
                "inbox_webhook_reference_invalid",
            ),
            (
                evidence.provider_available,
                DomainPackPreflightStatus::Unavailable,
                "inbox_provider_unavailable",
            ),
            (
                evidence.command_capability_available,
                DomainPackPreflightStatus::Unsupported,
                "inbox_command_unsupported",
            ),
            (
                evidence.rate_limit_available,
                DomainPackPreflightStatus::QuotaExceeded,
                "inbox_rate_limited",
            ),
            (
                evidence.timeout_within_limit,
                DomainPackPreflightStatus::QuotaExceeded,
                "inbox_timeout_budget_exceeded",
            ),
            (
                evidence.page_size_within_limit,
                DomainPackPreflightStatus::QuotaExceeded,
                "inbox_page_limit_exceeded",
            ),
            (
                evidence.storage_budget_reserved,
                DomainPackPreflightStatus::QuotaExceeded,
                "inbox_storage_unreserved",
            ),
            (
                evidence.body_redaction_allowed,
                DomainPackPreflightStatus::Denied,
                "inbox_body_redaction_denied",
            ),
            (
                evidence.attachment_redaction_allowed,
                DomainPackPreflightStatus::Denied,
                "inbox_attachment_redaction_denied",
            ),
        ];
        for (allowed, status, reason_code) in checks {
            if !allowed {
                return Err(reject(status, reason_code));
            }
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
