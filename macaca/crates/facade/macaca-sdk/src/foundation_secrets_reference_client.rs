//! SDK Facade helpers for the foundation secrets-reference pack.
//!
//! These helpers construct only canonical traced service calls. Provider
//! clients, credentials, locators, and raw secret values remain outside SDK
//! memory and are owned by the runtime service composition.

use macaca_proto::{
    MacacaResult, SecretReference, SecretsAuditAccessCommand, SecretsBindPurposeCommand,
    SecretsInspectReferenceCommand, SecretsReferenceError, SecretsReferenceResultStatus,
    SecretsRenewLeaseCommand, SecretsResolveForProviderCommand, SecretsRevokeLeaseCommand,
    SecretsRotateReferenceCommand, TraceContext, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

/// Builder for one provider-neutral secrets-reference command.
#[derive(Debug, Clone, PartialEq)]
pub struct SecretsReferenceDomainPackCommandBuilder {
    command_name: &'static str,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl SecretsReferenceDomainPackCommandBuilder {
    /// Build a traced call after effective pack capability admission.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        DomainPackServiceCallBuilder::new(
            FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
            self.command_name,
            self.payload,
            self.trace,
        )?
        .build(resolved)
    }
}

pub fn secrets_reference_inspect_command(
    request: SecretsInspectReferenceCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.inspect_reference", &request, trace)
}
pub fn secrets_reference_bind_purpose_command(
    request: SecretsBindPurposeCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.bind_purpose", &request, trace)
}
pub fn secrets_reference_resolve_for_provider_command(
    request: SecretsResolveForProviderCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.resolve_for_provider", &request, trace)
}
pub fn secrets_reference_renew_lease_command(
    request: SecretsRenewLeaseCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.renew_lease", &request, trace)
}
pub fn secrets_reference_revoke_lease_command(
    request: SecretsRevokeLeaseCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.revoke_lease", &request, trace)
}
pub fn secrets_reference_rotate_command(
    request: SecretsRotateReferenceCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.rotate_reference", &request, trace)
}
pub fn secrets_reference_audit_access_command(
    request: SecretsAuditAccessCommand,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    command("secrets.audit_access", &request, trace)
}

/// Return a sanitized unavailable diagnostic for metadata-only callers.
pub fn secrets_reference_unavailable_diagnostic(
    reference: Option<SecretReference>,
) -> SecretsReferenceError {
    SecretsReferenceError {
        code: SecretsReferenceResultStatus::Unavailable,
        message: reference
            .map(|value| format!("provider unavailable for reference {}", value.reference_id))
            .unwrap_or_else(|| "secrets-reference provider is unavailable".into()),
        retryable: true,
    }
}

fn command<T: serde::Serialize>(
    command_name: &'static str,
    request: &T,
    trace: TraceContext,
) -> MacacaResult<SecretsReferenceDomainPackCommandBuilder> {
    Ok(SecretsReferenceDomainPackCommandBuilder {
        command_name,
        payload: serde_json::to_value(request)?,
        trace,
    })
}

#[cfg(test)]
#[path = "foundation_secrets_reference_client_tests.rs"]
mod tests;
