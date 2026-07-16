use macaca_proto::domain_pack_contract::{
    identity_account::{IDENTITY_ACCOUNT_PACK_ID, IDENTITY_ACCOUNT_SERVICE_ID},
    identity_auth_handoff::{IDENTITY_AUTH_HANDOFF_PACK_ID, IDENTITY_AUTH_HANDOFF_SERVICE_ID},
    identity_organization::{IDENTITY_ORGANIZATION_PACK_ID, IDENTITY_ORGANIZATION_SERVICE_ID},
    identity_profile::{IDENTITY_PROFILE_PACK_ID, IDENTITY_PROFILE_SERVICE_ID},
    identity_tenant::{IDENTITY_TENANT_PACK_ID, IDENTITY_TENANT_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Identity SDK tests validate catalog discovery only. The SDK must not create
// identity providers, auth clients, browser callbacks, session stores, policy
// engines, cloud quota clients, or credential stores; it only reports
// descriptor metadata and unavailable diagnostics from the provider-neutral
// catalog.

#[tokio::test]
async fn catalog_client_discovers_identity_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            IDENTITY_ACCOUNT_PACK_ID,
            IDENTITY_ACCOUNT_SERVICE_ID,
            "account.plan_lifecycle_transition",
            "identity_account_provider_not_installed",
            "account-lifecycle",
        ),
        (
            IDENTITY_PROFILE_PACK_ID,
            IDENTITY_PROFILE_SERVICE_ID,
            "profile.inspect_privacy_fields",
            "identity_profile_provider_not_installed",
            "profile-privacy",
        ),
        (
            IDENTITY_AUTH_HANDOFF_PACK_ID,
            IDENTITY_AUTH_HANDOFF_SERVICE_ID,
            "auth_handoff.verify_callback",
            "identity_auth_handoff_provider_not_installed",
            "callback-verifier",
        ),
        (
            IDENTITY_ORGANIZATION_PACK_ID,
            IDENTITY_ORGANIZATION_SERVICE_ID,
            "organization.plan_membership_change",
            "identity_organization_provider_not_installed",
            "organization-membership",
        ),
        (
            IDENTITY_TENANT_PACK_ID,
            IDENTITY_TENANT_SERVICE_ID,
            "tenant.inspect_quota",
            "identity_tenant_provider_not_installed",
            "tenant-quota",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid identity id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("identity descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/identity"));
    }
}

#[tokio::test]
async fn catalog_client_reports_identity_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                IDENTITY_ACCOUNT_PACK_ID.into(),
                IDENTITY_PROFILE_PACK_ID.into(),
                IDENTITY_AUTH_HANDOFF_PACK_ID.into(),
                IDENTITY_ORGANIZATION_PACK_ID.into(),
                IDENTITY_TENANT_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            IDENTITY_ACCOUNT_PACK_ID,
            "identity_account_provider_not_installed",
        ),
        (
            IDENTITY_PROFILE_PACK_ID,
            "identity_profile_provider_not_installed",
        ),
        (
            IDENTITY_AUTH_HANDOFF_PACK_ID,
            "identity_auth_handoff_provider_not_installed",
        ),
        (
            IDENTITY_ORGANIZATION_PACK_ID,
            "identity_organization_provider_not_installed",
        ),
        (
            IDENTITY_TENANT_PACK_ID,
            "identity_tenant_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
