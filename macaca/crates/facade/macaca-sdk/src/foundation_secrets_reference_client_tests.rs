//! Tests proving secret-reference SDK helpers remain provider-neutral.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    SecretReference, SecretsAuditAccessCommand, SecretsBindPurposeCommand,
    SecretsInspectReferenceCommand, SecretsRenewLeaseCommand, SecretsResolveForProviderCommand,
    SecretsRevokeLeaseCommand, SecretsRotateReferenceCommand, TraceContext,
    FOUNDATION_SECRETS_REFERENCE_PACK_ID, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
};

use super::*;
use crate::domain_pack_client::SystemDomainPackClient;
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> crate::DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_secrets_reference_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_SECRETS_REFERENCE_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

fn reference() -> SecretReference {
    SecretReference {
        reference_id: "secret-ref".into(),
        provider_class: "mock".into(),
        version_hint: Some("current".into()),
    }
}

#[tokio::test]
async fn helpers_build_traced_calls_without_provider_access(
) -> Result<(), macaca_proto::MacacaError> {
    let reference = reference();
    let purpose = macaca_proto::SecretPurposeBinding {
        purpose: "database".into(),
        service_id: "service.database".into(),
        expires_at_epoch_millis: None,
    };
    let lease = macaca_proto::SecretLeaseReference {
        lease_id: "lease-ref".into(),
        reference_id: reference.reference_id.clone(),
        expires_at_epoch_millis: 9_999_999_999,
    };
    let helpers = vec![
        (
            "secrets.inspect_reference",
            secrets_reference_inspect_command(
                SecretsInspectReferenceCommand {
                    reference: reference.clone(),
                },
                TraceContext::new("trace-inspect"),
            )?,
        ),
        (
            "secrets.bind_purpose",
            secrets_reference_bind_purpose_command(
                SecretsBindPurposeCommand {
                    reference: reference.clone(),
                    purpose: purpose.clone(),
                },
                TraceContext::new("trace-bind"),
            )?,
        ),
        (
            "secrets.resolve_for_provider",
            secrets_reference_resolve_for_provider_command(
                SecretsResolveForProviderCommand {
                    reference: reference.clone(),
                    purpose: "database".into(),
                    service_id: "service.database".into(),
                },
                TraceContext::new("trace-resolve"),
            )?,
        ),
        (
            "secrets.renew_lease",
            secrets_reference_renew_lease_command(
                SecretsRenewLeaseCommand {
                    lease: lease.clone(),
                    ttl_seconds: 60,
                },
                TraceContext::new("trace-renew"),
            )?,
        ),
        (
            "secrets.revoke_lease",
            secrets_reference_revoke_lease_command(
                SecretsRevokeLeaseCommand {
                    lease: lease.clone(),
                    reason: "rotation".into(),
                },
                TraceContext::new("trace-revoke"),
            )?,
        ),
        (
            "secrets.rotate_reference",
            secrets_reference_rotate_command(
                SecretsRotateReferenceCommand {
                    reference: reference.clone(),
                    dry_run: true,
                },
                TraceContext::new("trace-rotate"),
            )?,
        ),
        (
            "secrets.audit_access",
            secrets_reference_audit_access_command(
                SecretsAuditAccessCommand {
                    reference,
                    since_event_id: None,
                },
                TraceContext::new("trace-audit"),
            )?,
        ),
    ];
    for (name, helper) in helpers {
        let command = helper.build(&resolved().await)?;
        assert_eq!(command.service_id, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID);
        assert_eq!(command.command_name, name);
        assert!(command.trace.is_some());
    }
    Ok(())
}

#[test]
fn unavailable_diagnostic_contains_metadata_only() {
    let diagnostic = secrets_reference_unavailable_diagnostic(Some(reference()));
    assert_eq!(
        diagnostic.code,
        macaca_proto::SecretsReferenceResultStatus::Unavailable
    );
    assert!(!diagnostic.message.contains("value"));
}
