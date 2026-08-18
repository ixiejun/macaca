//! Approval Specification for secret-reference side effects.

use serde::{Deserialize, Serialize};

/// Sanitized facts used before any provider operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretApprovalFacts {
    pub policy_requires_approval: bool,
    pub approval_granted: bool,
    pub provider_resolution: bool,
    pub export_audit: bool,
    pub revoke_or_rotate: bool,
}

/// Fail-closed admission outcomes for secret-reference side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretApprovalFailure {
    ApprovalRequired,
}

/// Evaluate approval before provider selection or secret injection.
pub fn approve_secret_operation(
    command: &str,
    facts: SecretApprovalFacts,
) -> Result<(), SecretApprovalFailure> {
    let sensitive = matches!(
        command,
        "secrets.import_reference"
            | "secrets.bind_purpose"
            | "secrets.resolve_for_provider"
            | "secrets.rotate_reference"
            | "secrets.revoke_lease"
            | "secrets.audit_access"
    ) || facts.provider_resolution
        || facts.export_audit
        || facts.revoke_or_rotate;
    if (sensitive || facts.policy_requires_approval) && !facts.approval_granted {
        Err(SecretApprovalFailure::ApprovalRequired)
    } else {
        Ok(())
    }
}
