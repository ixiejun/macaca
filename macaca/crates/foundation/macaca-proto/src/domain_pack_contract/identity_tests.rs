use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::identity_account::*;
use super::identity_auth_handoff::*;
use super::identity_common::*;
use super::identity_organization::*;
use super::identity_profile::*;
use super::identity_tenant::*;
use super::*;

// Identity tests validate provider-neutral contract shape only. They do not
// contact identity providers, directories, auth endpoints, browsers, session
// stores, policy engines, cloud control planes, or credential stores. Fixtures
// intentionally use synthetic handles and hashes instead of raw identity data.

#[test]
fn identity_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            identity_account_pack_definition(),
            IDENTITY_ACCOUNT_PACK_ID,
            IDENTITY_ACCOUNT_SERVICE_ID,
            IDENTITY_ACCOUNT_COMMANDS,
            "identity_account_provider_not_installed",
            "account-lifecycle",
            "account.plan_lifecycle_transition",
        ),
        (
            identity_profile_pack_definition(),
            IDENTITY_PROFILE_PACK_ID,
            IDENTITY_PROFILE_SERVICE_ID,
            IDENTITY_PROFILE_COMMANDS,
            "identity_profile_provider_not_installed",
            "profile-privacy",
            "profile.inspect_privacy_fields",
        ),
        (
            identity_auth_handoff_pack_definition(),
            IDENTITY_AUTH_HANDOFF_PACK_ID,
            IDENTITY_AUTH_HANDOFF_SERVICE_ID,
            IDENTITY_AUTH_HANDOFF_COMMANDS,
            "identity_auth_handoff_provider_not_installed",
            "callback-verifier",
            "auth_handoff.verify_callback",
        ),
        (
            identity_organization_pack_definition(),
            IDENTITY_ORGANIZATION_PACK_ID,
            IDENTITY_ORGANIZATION_SERVICE_ID,
            IDENTITY_ORGANIZATION_COMMANDS,
            "identity_organization_provider_not_installed",
            "organization-membership",
            "organization.plan_membership_change",
        ),
        (
            identity_tenant_pack_definition(),
            IDENTITY_TENANT_PACK_ID,
            IDENTITY_TENANT_SERVICE_ID,
            IDENTITY_TENANT_COMMANDS,
            "identity_tenant_provider_not_installed",
            "tenant-quota",
            "tenant.inspect_quota",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.identity.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/identity"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("identity descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_identity_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let account = find_pack(&definitions, IDENTITY_ACCOUNT_PACK_ID);
    let profile = find_pack(&definitions, IDENTITY_PROFILE_PACK_ID);
    let auth = find_pack(&definitions, IDENTITY_AUTH_HANDOFF_PACK_ID);
    let organization = find_pack(&definitions, IDENTITY_ORGANIZATION_PACK_ID);
    let tenant = find_pack(&definitions, IDENTITY_TENANT_PACK_ID);

    assert_eq!(
        account
            .metadata
            .provider_descriptors
            .get("linked-identity")
            .and_then(|descriptor| descriptor.metadata.get("raw_credentials"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        profile
            .metadata
            .provider_descriptors
            .get("profile-avatar")
            .and_then(|descriptor| descriptor.metadata.get("raw_bytes"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        auth.metadata
            .provider_descriptors
            .get("token-reference")
            .and_then(|descriptor| descriptor.metadata.get("raw_tokens"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        organization
            .metadata
            .provider_descriptors
            .get("organization-membership")
            .and_then(|descriptor| descriptor.metadata.get("account_profile_mutation"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        tenant
            .metadata
            .provider_descriptors
            .get("tenant-config")
            .and_then(|descriptor| descriptor.metadata.get("secret_values"))
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn identity_command_and_result_dtos_are_serde_compatible() {
    let envelope = IdentityPackCommandEnvelope {
        subject_ref: "identity:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(20),
        idempotency_key: Some("idem-identity".into()),
        approval_ref: Some("approval-identity".into()),
    };

    let values = [
        serde_json::to_value(AccountPlanLifecycleTransitionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(ProfilePlanAvatarUpdateCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(AuthHandoffVerifyCallbackCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(OrganizationPlanRoleBindingCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(TenantPlanQuotaReservationCommand { request: envelope }).unwrap(),
        serde_json::to_value(AccountResultEnvelope::<AccountRecord> {
            status: AccountResultStatus::LifecycleInvalid,
            data: None,
            page: None,
            error: Some(IdentityPackError {
                code: "lifecycle_invalid".into(),
                message: "synthetic invalid lifecycle transition".into(),
                retryable: false,
                trace_safe_detail: Some("plan_required".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(ProfileResultEnvelope::<ProfileRecord> {
            status: ProfileResultStatus::ArtifactRedacted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(AuthHandoffResultEnvelope::<AuthHandoffRecord> {
            status: AuthHandoffResultStatus::ReplayRejected,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(OrganizationResultEnvelope::<OrganizationRecord> {
            status: OrganizationResultStatus::ApprovalRequired,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(TenantResultEnvelope::<TenantRecord> {
            status: TenantResultStatus::SecretReferenceDenied,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn identity_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&identity_account_descriptor_hashes()),
        hash_values(&identity_profile_descriptor_hashes()),
        hash_values(&identity_auth_handoff_descriptor_hashes()),
        hash_values(&identity_organization_descriptor_hashes()),
        hash_values(&identity_tenant_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 8);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn organization_and_tenant_hashes_cover_schema_policy_and_redaction() {
    let organization_hashes = identity_organization_descriptor_hashes();
    assert_eq!(
        organization_hashes,
        identity_organization_descriptor_hashes()
    );
    assert!(!organization_hashes
        .schema_version_compatibility_hash
        .is_empty());
    assert!(!organization_hashes.command_availability_hash.is_empty());
    assert!(!organization_hashes.role_schema_hash.is_empty());
    assert!(!organization_hashes.policy_template_hash.is_empty());
    assert!(!organization_hashes.redaction_profile_hash.is_empty());

    let tenant_hashes = identity_tenant_descriptor_hashes();
    assert_eq!(tenant_hashes, identity_tenant_descriptor_hashes());
    assert!(!tenant_hashes.schema_version_compatibility_hash.is_empty());
    assert!(!tenant_hashes.command_availability_hash.is_empty());
    assert!(!tenant_hashes.quota_envelope_hash.is_empty());
    assert!(!tenant_hashes.config_reference_hash.is_empty());
    assert!(!tenant_hashes.policy_template_hash.is_empty());
    assert!(!tenant_hashes.redaction_profile_hash.is_empty());
}

#[test]
fn organization_and_tenant_descriptor_contract_rejects_invalid_metadata() {
    for definition in [
        identity_organization_pack_definition(),
        identity_tenant_pack_definition(),
    ] {
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());

        let mut missing_command_schema = definition.clone();
        missing_command_schema.metadata.availability = DomainPackAvailability::Available;
        missing_command_schema
            .metadata
            .service_command_schemas
            .clear();
        assert!(DomainPackDefinitionSpec
            .validate(&missing_command_schema)
            .is_err());

        let mut invalid_scope = definition.clone();
        invalid_scope
            .metadata
            .permission_scopes
            .insert("identity invalid scope".into());
        assert!(DomainPackDefinitionSpec.validate(&invalid_scope).is_err());

        let mut incompatible_metadata_version = definition.clone();
        incompatible_metadata_version.metadata.version = "v2".into();
        assert!(DomainPackDefinitionSpec
            .validate(&incompatible_metadata_version)
            .is_err());

        let mut incompatible_parent_version = definition.clone();
        incompatible_parent_version.metadata.parent_pack_id = Some("pack.identity.v2".into());
        assert!(DomainPackDefinitionSpec
            .validate(&incompatible_parent_version)
            .is_err());

        assert!(definition
            .metadata
            .data_governance
            .redaction_policy
            .contains("redacted"));
    }
}

#[test]
fn identity_validation_helpers_are_provider_neutral() {
    let account = AccountRecord {
        identifiers: vec![AccountIdentifier::default()],
        linked_identities: vec![LinkedIdentityReference::default()],
        ..Default::default()
    };
    assert!(account.is_bounded(2, 2));

    let profile = ProfileRecord {
        fields: vec![ProfileField::default()],
        preferences: vec![ProfilePreference::default()],
        ..Default::default()
    };
    assert!(profile.is_bounded(2, 2));

    let organization = OrganizationRecord {
        identifiers: vec![OrganizationIdentifier::default()],
        audit_refs: vec![OrganizationAuditReference::default()],
        ..Default::default()
    };
    assert!(organization.is_bounded(2, 2));

    let tenant = TenantRecord {
        identifiers: vec![TenantIdentifier::default()],
        relationship_refs: vec![TenantRelationshipReference::default()],
        ..Default::default()
    };
    assert!(tenant.is_bounded(2, 2));

    let token_ref = TokenReference {
        token_ref: "token-ref".into(),
        storage_boundary_ref: "secret-ref".into(),
        redaction_class: "handle_only".into(),
        ..Default::default()
    };
    assert_eq!(token_ref.token_ref, "token-ref");
}

#[test]
fn invalid_identity_descriptor_is_rejected() {
    let mut invalid = identity_account_pack_definition();
    invalid.pack_id = "pack.identity.account.v2".into();
    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized identity descriptor")
}

fn hash_values<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_value(value)
        .expect("descriptor hash DTO is serializable")
        .as_object()
        .expect("descriptor hash DTO serializes as an object")
        .values()
        .map(|value| {
            value
                .as_str()
                .expect("descriptor hash fields are strings")
                .to_string()
        })
        .collect()
}
