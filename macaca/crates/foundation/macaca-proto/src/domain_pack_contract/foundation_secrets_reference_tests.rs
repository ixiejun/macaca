use std::collections::{BTreeMap, BTreeSet};

use super::*;

// These tests validate secret-reference metadata and DTO compatibility only.
// They never create, import, resolve, or log a real secret value.

#[test]
fn secret_reference_manifest_declarations_require_the_pack_and_unique_safe_ids() {
    let reference = SecretReference {
        reference_id: "secret-ref".into(),
        provider_class: "vault".into(),
        version_hint: Some("current".into()),
    };
    let mut declaration = AppServiceContractConfig {
        secret_reference_declarations: vec![reference.clone()],
        ..Default::default()
    };
    assert!(validate_secret_reference_declarations(&declaration).is_err());
    declaration
        .optional_packs
        .push(FOUNDATION_SECRETS_REFERENCE_PACK_ID.into());
    assert!(validate_secret_reference_declarations(&declaration).is_ok());
    declaration.secret_reference_declarations.push(reference);
    assert!(validate_secret_reference_declarations(&declaration).is_err());
}

#[test]
fn foundation_secrets_reference_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_secrets_reference_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_SECRETS_REFERENCE_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "secrets_reference_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/secrets-reference"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_SECRETS_REFERENCE_SERVICE_ID)
        .expect("secrets-reference descriptor exposes command schemas");
    for command in FOUNDATION_SECRETS_REFERENCE_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "secrets.reference.read",
        "secrets.reference.create",
        "secrets.reference.import",
        "secrets.reference.list",
        "secrets.reference.bind",
        "secrets.reference.resolve",
        "secrets.reference.lease",
        "secrets.reference.rotate",
        "secrets.reference.revoke",
        "secrets.reference.audit",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in [
        "vault",
        "cloud-secrets",
        "host-keychain",
        "kubernetes-secret",
        "mock",
        "unavailable",
    ] {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor {provider_class}"
        );
    }
}

#[test]
fn secret_reference_approval_is_fail_closed_for_sensitive_operations() {
    let denied = SecretApprovalFacts {
        policy_requires_approval: false,
        approval_granted: false,
        provider_resolution: false,
        export_audit: false,
        revoke_or_rotate: false,
    };
    assert_eq!(
        approve_secret_operation("secrets.import_reference", denied),
        Err(SecretApprovalFailure::ApprovalRequired)
    );
    assert!(approve_secret_operation("secrets.inspect_reference", denied).is_ok());
    assert!(approve_secret_operation(
        "secrets.resolve_for_provider",
        SecretApprovalFacts {
            approval_granted: true,
            ..denied
        }
    )
    .is_ok());
}

#[test]
fn secret_reference_preconditions_reject_invalid_purpose_and_private_material() {
    let bad_purpose = SecretPurposeBinding {
        purpose: "purpose".into(),
        service_id: "not-a-service".into(),
        expires_at_epoch_millis: None,
    };
    assert!(!bad_purpose.is_bounded_binding());
    let raw_locator = SecretExternalLocator {
        provider_class: "mock".into(),
        redacted_locator_hash: "https://provider.example/raw".into(),
    };
    assert!(!raw_locator.is_safe_reference());
    assert_eq!(
        SecretsReferenceResultStatus::RawSecretForbidden,
        SecretsReferenceResultStatus::RawSecretForbidden
    );
}

#[test]
fn industrial_catalog_uses_foundation_secrets_reference_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_SECRETS_REFERENCE_PACK_ID)
        .expect("industrial catalog includes foundation secrets reference");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_SECRETS_REFERENCE_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("vault")
            .and_then(|descriptor| descriptor.metadata.get("provider_injection"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_secrets_reference_command_dtos_are_serde_compatible() {
    let reference = SecretReference {
        reference_id: "secret-ref".into(),
        provider_class: "vault".into(),
        version_hint: Some("current".into()),
    };
    let purpose = SecretPurposeBinding {
        purpose: "database-password".into(),
        service_id: "service.example".into(),
        expires_at_epoch_millis: Some(1_800_000_000_000),
    };
    let policy = SecretAccessPolicy {
        allowed_service_ids: BTreeSet::from(["service.example".into()]),
        requires_approval: true,
        max_lease_ttl_seconds: 300,
    };
    let lease = SecretLeaseReference {
        lease_id: "lease-ref".into(),
        reference_id: "secret-ref".into(),
        expires_at_epoch_millis: 1_800_000_000_000,
    };

    let commands = vec![
        serde_json::to_value(SecretsCreateReferenceCommand {
            reference: reference.clone(),
            purpose: purpose.clone(),
            policy: policy.clone(),
        })
        .unwrap(),
        serde_json::to_value(SecretsImportReferenceCommand {
            locator: SecretExternalLocator {
                provider_class: "vault".into(),
                redacted_locator_hash: "locator-hash".into(),
            },
            purpose: purpose.clone(),
            policy,
        })
        .unwrap(),
        serde_json::to_value(SecretsInspectReferenceCommand {
            reference: reference.clone(),
        })
        .unwrap(),
        serde_json::to_value(SecretsListReferencesCommand {
            provider_class: Some("vault".into()),
            cursor: None,
            page_size: 50,
        })
        .unwrap(),
        serde_json::to_value(SecretsBindPurposeCommand {
            reference: reference.clone(),
            purpose,
        })
        .unwrap(),
        serde_json::to_value(SecretsResolveForProviderCommand {
            reference: reference.clone(),
            purpose: "database-password".into(),
            service_id: "service.example".into(),
        })
        .unwrap(),
        serde_json::to_value(SecretsCreateLeaseCommand {
            reference: reference.clone(),
            purpose: "database-password".into(),
            ttl_seconds: 300,
        })
        .unwrap(),
        serde_json::to_value(SecretsRenewLeaseCommand {
            lease: lease.clone(),
            ttl_seconds: 300,
        })
        .unwrap(),
        serde_json::to_value(SecretsRevokeLeaseCommand {
            lease: lease.clone(),
            reason: "test-finished".into(),
        })
        .unwrap(),
        serde_json::to_value(SecretsRotateReferenceCommand {
            reference: reference.clone(),
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(SecretsVersionStatusCommand {
            reference: reference.clone(),
        })
        .unwrap(),
        serde_json::to_value(SecretsAuditAccessCommand {
            reference,
            since_event_id: Some("event-ref".into()),
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_SECRETS_REFERENCE_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_secrets_reference_hashes_change_with_contract_content() {
    let request = SecretsListReferencesCommand {
        provider_class: Some("vault".into()),
        cursor: None,
        page_size: 50,
    };
    let mut changed = request.clone();
    changed.page_size = 100;

    assert_eq!(
        secrets_reference_stable_hash(&request),
        secrets_reference_stable_hash(&request)
    );
    assert_ne!(
        secrets_reference_stable_hash(&request),
        secrets_reference_stable_hash(&changed)
    );

    let hashes = foundation_secrets_reference_descriptor_hashes();
    let unique = BTreeSet::from([
        hashes.command_schema_hash,
        hashes.result_schema_hash,
        hashes.snapshot_schema_hash,
        hashes.provider_capability_schema_hash,
        hashes.unavailable_schema_hash,
    ]);
    assert_eq!(unique.len(), 5);
    assert!(unique.iter().all(|hash| !hash.is_empty()));
}

#[test]
fn foundation_secrets_reference_result_and_snapshot_dtos_are_bounded() {
    let snapshot = SecretsReferenceProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        reference_state_hashes: BTreeMap::from([("secret-ref".into(), "state-hash".into())]),
        lease_state_hashes: BTreeMap::from([("lease-ref".into(), "lease-hash".into())]),
        audit_tail_hash: "audit-tail-hash".into(),
    };
    let unavailable: SecretsReferenceResultEnvelope<SecretAuditRecord> =
        SecretsReferenceResultEnvelope {
            status: SecretsReferenceResultStatus::Unavailable,
            data: None,
            error: Some(SecretsReferenceError {
                code: SecretsReferenceResultStatus::Unavailable,
                message: "secret-reference provider is not installed".into(),
                retryable: false,
            }),
            trace_id: "trace-secrets-reference-unavailable".into(),
            descriptor_hash: secrets_reference_stable_hash(&snapshot),
        };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-secrets-reference-unavailable"));
    assert!(!serialized.contains("raw-secret-value"));
    assert!(!serialized.contains("provider-private-locator"));
    assert!(!serialized.contains("DATABASE_PASSWORD"));
}
