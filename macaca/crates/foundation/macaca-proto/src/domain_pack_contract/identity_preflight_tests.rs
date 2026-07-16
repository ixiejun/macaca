use super::identity_account::{
    AccountAuditExportPlan, AccountFreshness, AccountIdentifier, AccountLifecycleTransitionPlan,
    AccountLifecycleTransitionRequest, AccountRecord, AccountRecoveryReference, AccountScope,
    LinkedIdentityReference,
};
use super::identity_auth_handoff::{
    AuthHandoffAuditExportPlan, AuthHandoffFreshness, AuthHandoffPlan, AuthProtocolProfile,
    CallbackVerificationResult, HandoffCorrelation, RedirectCallbackDescriptor,
    SessionBindingEvidence, SubjectEvidence, TokenReference,
};
use super::identity_common::IdentityPackCommandEnvelope;
use super::identity_profile::{
    AvatarReference, ProfileExportPlan, ProfileField, ProfileFreshness, ProfileRecord,
    ProfileSchemaDescriptor,
};

#[test]
fn identity_preflight_rejects_raw_account_data_and_invalid_lifecycle_requests() {
    let identifier = AccountIdentifier {
        identifier_ref: "identifier-ref".into(),
        identifier_kind: "email".into(),
        normalized_value_hash: "hash".into(),
        verification_state: "verified".into(),
        redaction_class: "hash_only".into(),
    };
    assert!(identifier.is_safe_reference());
    assert!(!AccountIdentifier {
        normalized_value_hash: "user@example.test".into(),
        ..identifier
    }
    .is_safe_reference());

    let plan = AccountLifecycleTransitionPlan {
        plan_ref: "plan-ref".into(),
        from_state: "active".into(),
        to_state: "suspended".into(),
        requires_approval: true,
        validation_diagnostics: vec!["policy-pending".into()],
    };
    let request = AccountLifecycleTransitionRequest {
        plan_ref: "plan-ref".into(),
        idempotency_key_hash: "idem-hash".into(),
        approval_ref: Some("approval-ref".into()),
        version_token_hash: "version-hash".into(),
    };
    assert!(request.has_safe_preconditions(&plan));
    assert!(!AccountLifecycleTransitionRequest {
        approval_ref: None,
        ..request
    }
    .has_safe_preconditions(&plan));
}

#[test]
fn identity_preflight_requires_reference_only_handoff_correlation_and_tokens() {
    let plan = AuthHandoffPlan {
        plan_ref: "handoff-plan".into(),
        protocol: AuthProtocolProfile {
            profile_ref: "protocol-profile".into(),
            protocol_kind: "oidc_authorization_code".into(),
            support_metadata: Default::default(),
        },
        redirect: RedirectCallbackDescriptor {
            redirect_ref: "redirect-handle".into(),
            callback_ref: "callback-handle".into(),
            binding_kind: "redirect".into(),
            allowlist_policy_ref: "allowlist-policy".into(),
            method: "GET".into(),
        },
        scopes: ["identity.read".to_string()].into_iter().collect(),
        hints: Default::default(),
        correlation: HandoffCorrelation {
            state_hash: Some("state-hash".into()),
            nonce_hash: Some("nonce-hash".into()),
            replay_pointer: "replay-pointer".into(),
            ..Default::default()
        },
        expires_at_epoch_ms: 200,
        redaction_class: "hash_only".into(),
    };
    assert!(plan.has_safe_preconditions(100, 8));
    assert!(!RedirectCallbackDescriptor {
        callback_ref: "https://raw.example/callback".into(),
        ..plan.redirect.clone()
    }
    .is_allowlisted_reference());

    let token = TokenReference {
        token_ref: "artifact:token-reference".into(),
        token_class: "access_reference".into(),
        expires_at_epoch_ms: Some(200),
        scope_hints: Default::default(),
        storage_boundary_ref: "secret-store".into(),
        refreshable: false,
        redaction_class: "handle_only".into(),
        access_policy_ref: "policy-ref".into(),
    };
    assert!(token.is_safe_reference(100));
    assert!(!TokenReference {
        token_ref: "token=raw-secret".into(),
        ..token
    }
    .is_safe_reference(100));
}

#[test]
fn identity_preflight_enforces_account_isolation_uniqueness_and_recovery_redaction() {
    let scope = AccountScope {
        tenant_scope: "tenant-a".into(),
        provider_scope: "provider-scope".into(),
        permission_scope: "identity.account.update".into(),
        ..Default::default()
    };
    let identifier = AccountIdentifier {
        identifier_ref: "email-ref".into(),
        identifier_kind: "email".into(),
        normalized_value_hash: "email-hash".into(),
        verification_state: "verified".into(),
        redaction_class: "hash-only".into(),
    };
    let account = AccountRecord {
        account_ref: "account-ref".into(),
        stable_subject_ref: "subject-ref".into(),
        identifiers: vec![identifier.clone()],
        linked_identities: vec![LinkedIdentityReference {
            link_ref: "link-ref".into(),
            provider_class: "directory".into(),
            issuer_ref: "issuer-ref".into(),
            external_subject_ref: "external-subject".into(),
            assurance_level: "aal2".into(),
            link_state: "linked".into(),
            freshness: AccountFreshness {
                source_timestamp_epoch_ms: 10,
                freshness_class: "current".into(),
                ..Default::default()
            },
            replay_pointer: "replay-ref".into(),
        }],
        tenant_refs: vec!["tenant-a".into()],
        recovery_refs: vec![AccountRecoveryReference {
            recovery_ref: "recovery-ref".into(),
            recovery_kind: "reset_flow".into(),
            reset_flow_ref: Some("artifact:reset-flow".into()),
            redaction_profile: "reference-only".into(),
            ..Default::default()
        }],
        version_token_hash: "version-hash".into(),
        freshness: AccountFreshness {
            source_timestamp_epoch_ms: 10,
            freshness_class: "current".into(),
            ..Default::default()
        },
        redaction_class: "reference-only".into(),
        ..Default::default()
    };
    assert!(scope.is_isolated_scope());
    assert!(account.has_safe_projection(4, 4));
    assert!(account.is_isolated_to(&scope));
    assert!(!AccountRecord {
        identifiers: vec![identifier.clone(), identifier],
        ..account
    }
    .has_safe_projection(4, 4));
    assert!(AccountAuditExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "account-ref".into(),
        format: "artifact".into(),
        redaction_profile: "redacted".into(),
        retention_class: "short".into()
    }
    .is_bounded_export());
}

#[test]
fn identity_preflight_validates_callback_subject_and_session_evidence() {
    let freshness = AuthHandoffFreshness {
        source_timestamp_epoch_ms: 10,
        freshness_class: "fresh".into(),
        ..Default::default()
    };
    let callback = CallbackVerificationResult {
        verification_ref: "verification-ref".into(),
        verified: true,
        issuer_ref: Some("issuer-ref".into()),
        audience_ref: Some("audience-ref".into()),
        signature_ref: Some("artifact:signature".into()),
        subject_ref: Some("subject-ref".into()),
        freshness: freshness.clone(),
        replay_pointer: "replay-ref".into(),
        ..Default::default()
    };
    let subject = SubjectEvidence {
        subject_evidence_ref: "subject-evidence".into(),
        provider_subject_ref: "provider-subject".into(),
        assurance_level: "aal2".into(),
        authentication_context: "context-ref".into(),
        freshness,
        redaction_class: "reference-only".into(),
        ..Default::default()
    };
    let binding = SessionBindingEvidence {
        binding_ref: "binding-ref".into(),
        session_ref: "session-ref".into(),
        subject_evidence_ref: "subject-evidence".into(),
        approval_ref: Some("approval-ref".into()),
        binding_state: "bound".into(),
        replay_pointer: "replay-ref".into(),
        ..Default::default()
    };
    assert!(callback.has_complete_verified_evidence(1));
    assert!(subject.is_safe_reference(1));
    assert!(binding.is_safe_reference(1) && binding.has_approval_evidence());
    assert!(AuthHandoffAuditExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "handoff-ref".into(),
        format: "artifact".into(),
        redaction_profile: "redacted".into()
    }
    .is_bounded_export());
}

#[test]
fn identity_preflight_bounds_profile_fields_avatars_and_exports() {
    let field = ProfileField {
        field_key: "display_name".into(),
        value_type: "string".into(),
        value_ref: "artifact:field-value".into(),
        source_ref: "profile-source".into(),
        verification_state: "verified".into(),
        visibility: "private".into(),
        privacy_class: "private".into(),
        retention_class: "profile".into(),
        mutable: true,
        locale: None,
        redaction_class: "reference_only".into(),
    };
    assert!(field.is_safe_projection());
    assert!(!ProfileField {
        value_ref: "password=raw".into(),
        ..field
    }
    .is_safe_projection());

    let avatar = AvatarReference {
        avatar_ref: "avatar-ref".into(),
        hosted_url_ref: None,
        media_artifact_handle: Some("artifact:avatar".into()),
        checksum: "checksum".into(),
        dimensions: "128x128".into(),
        content_type: "image/png".into(),
        expires_at_epoch_ms: Some(200),
        retention_class: "short".into(),
        source_ref: "profile-source".into(),
        redaction_profile: "avatar-redacted".into(),
    };
    assert!(avatar.is_bounded_reference(100));
    let export = ProfileExportPlan {
        export_ref: "export-ref".into(),
        scope_ref: "profile-ref".into(),
        field_mask: ["display_name".to_string()].into_iter().collect(),
        format: "artifact".into(),
        redaction_profile: "export-redacted".into(),
    };
    assert!(export.is_bounded_export(4));

    let schema = ProfileSchemaDescriptor {
        schema_ref: "schema-ref".into(),
        field_definitions: [("display_name".into(), "string".into())]
            .into_iter()
            .collect(),
        default_field_mask: ["display_name".into()].into_iter().collect(),
        compatibility_hash: "schema-hash".into(),
        ..Default::default()
    };
    assert!(schema.is_valid_schema(4));
    let profile = ProfileRecord {
        profile_ref: "profile-ref".into(),
        account_ref: "account-ref".into(),
        subject_ref: "subject-ref".into(),
        fields: vec![ProfileField {
            field_key: "display_name".into(),
            value_type: "string".into(),
            value_ref: "artifact:value".into(),
            source_ref: "source-ref".into(),
            verification_state: "verified".into(),
            visibility: "private".into(),
            privacy_class: "private".into(),
            retention_class: "short".into(),
            mutable: true,
            redaction_class: "reference-only".into(),
            ..Default::default()
        }],
        privacy_map: [("display_name".into(), "private".into())]
            .into_iter()
            .collect(),
        version_token_hash: "version-hash".into(),
        freshness: ProfileFreshness {
            source_timestamp_epoch_ms: 10,
            freshness_class: "current".into(),
            ..Default::default()
        },
        redaction_class: "reference-only".into(),
        ..Default::default()
    };
    assert!(profile.has_safe_projection(4, 4));
    let command = IdentityPackCommandEnvelope {
        subject_ref: "subject-ref".into(),
        idempotency_key: Some("idempotency-key".into()),
        approval_ref: Some("approval-ref".into()),
        ..Default::default()
    };
    assert!(command.has_required_mutation_evidence());
}
