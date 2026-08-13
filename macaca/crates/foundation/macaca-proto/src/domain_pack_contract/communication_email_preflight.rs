use serde::{Deserialize, Serialize};

use super::communication_email::{communication_email_pack_definition, EmailSendCommand};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Evaluated mail-delivery admission facts represented without message content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAdmissionEvidence {
    pub sender_verified: bool,
    pub recipient_valid: bool,
    pub recipient_consent_granted: bool,
    pub external_recipient_approved: bool,
    pub message_within_limit: bool,
    pub attachments_within_limit: bool,
    pub rate_limit_available: bool,
    pub webhook_signature_valid: bool,
    pub idempotency_available: bool,
    pub provider_capability_available: bool,
}

/// Declarative email-pack configuration owned by the application manifest layer.
///
/// Values are references and bounded identifiers only. Provider credentials,
/// mailbox contents, and inbound event payloads stay behind service adapters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailPackDeclaration {
    pub required: bool,
    pub sender_identity_refs: Vec<String>,
    pub mailbox_access_refs: Vec<String>,
    pub event_ingestion_endpoint_refs: Vec<String>,
}

/// Specification validating email declaration shape before provider admission.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmailPackDeclarationSpec;

impl EmailPackDeclarationSpec {
    /// Reject undeclared required capabilities and unbounded identity references.
    pub fn validate(&self, declaration: &EmailPackDeclaration) -> Result<(), String> {
        if declaration.required && declaration.sender_identity_refs.is_empty() {
            return Err("required email pack needs a sender identity reference".into());
        }
        for reference in declaration
            .sender_identity_refs
            .iter()
            .chain(declaration.mailbox_access_refs.iter())
            .chain(declaration.event_ingestion_endpoint_refs.iter())
        {
            if !is_safe_declaration_reference(reference) {
                return Err("email declaration contains an invalid reference".into());
            }
        }
        Ok(())
    }
}

fn is_safe_declaration_reference(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.' | '/')
        })
}

/// Provider-neutral mail dispatch Specification.
///
/// The host supplies policy outcomes and this contract protects the provider
/// closure. It neither resolves an address nor accesses SMTP, OAuth, mailbox,
/// attachment, webhook, or provider-native data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl EmailDispatchPreflight {
    /// Construct the gate from the email descriptor's command and scope lists.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &communication_email_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Validate a send request before a provider can create delivery side effects.
    pub fn evaluate_send(
        &self,
        command: &EmailSendCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &EmailAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if !command.has_send_preconditions() {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "email_send_invalid",
            ));
        }
        for (allowed, status, reason) in evidence_checks(evidence) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke a provider only after descriptor, policy, and delivery admission pass.
    pub fn dispatch_send<T>(
        &self,
        command: &EmailSendCommand,
        preflight: &DomainPackCommandPreflight,
        evidence: &EmailAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate_send(command, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn evidence_checks(
    evidence: &EmailAdmissionEvidence,
) -> [(bool, DomainPackPreflightStatus, &'static str); 10] {
    [
        (
            evidence.sender_verified,
            DomainPackPreflightStatus::Denied,
            "email_sender_unverified",
        ),
        (
            evidence.recipient_valid,
            DomainPackPreflightStatus::Denied,
            "email_invalid_recipient",
        ),
        (
            evidence.recipient_consent_granted,
            DomainPackPreflightStatus::Denied,
            "email_consent_required",
        ),
        (
            evidence.external_recipient_approved,
            DomainPackPreflightStatus::Denied,
            "email_external_approval_required",
        ),
        (
            evidence.message_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "email_message_limit_exceeded",
        ),
        (
            evidence.attachments_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "email_attachment_too_large",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "email_rate_limited",
        ),
        (
            evidence.webhook_signature_valid,
            DomainPackPreflightStatus::Denied,
            "email_webhook_signature_invalid",
        ),
        (
            evidence.idempotency_available,
            DomainPackPreflightStatus::Conflict,
            "email_idempotency_conflict",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "email_provider_rejected",
        ),
    ]
}

fn reject(status: DomainPackPreflightStatus, reason_code: &str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
