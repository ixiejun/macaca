use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::identity_common::{
    define_identity_command_wrappers, identity_pack_definition, identity_stable_hash,
    IdentityPackCommandEnvelope, IdentityPackDescriptor, IdentityPackError, IdentityPackPage,
    IdentityProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const IDENTITY_AUTH_HANDOFF_PACK_ID: &str = "pack.identity.auth.handoff.v1";
pub const IDENTITY_AUTH_HANDOFF_SERVICE_ID: &str = "service.identity.auth_handoff";

/// Stable trace vocabulary for auth-handoff observability and replay indexes.
pub const IDENTITY_AUTH_HANDOFF_TRACE_EVENTS: &[&str] = &[
    "auth_handoff_pack_declared",
    "auth_handoff_pack_admission_validated",
    "auth_handoff_pack_policy_decision",
    "auth_handoff_pack_provider_inspected",
    "auth_handoff_pack_service_call_requested",
    "auth_handoff_pack_service_call_succeeded",
    "auth_handoff_pack_service_call_failed",
    "auth_handoff_pack_handoff_planned",
    "auth_handoff_pack_callback_verified",
    "auth_handoff_pack_token_reference_exchanged",
    "auth_handoff_pack_session_binding_planned",
    "auth_handoff_pack_unavailable",
    "auth_handoff_pack_snapshot_recorded",
];

pub const IDENTITY_AUTH_HANDOFF_COMMANDS: &[&str] = &[
    "auth_handoff.inspect_provider",
    "auth_handoff.describe_schema",
    "auth_handoff.plan_handoff",
    "auth_handoff.start_handoff",
    "auth_handoff.verify_callback",
    "auth_handoff.exchange_token_reference",
    "auth_handoff.inspect_subject_evidence",
    "auth_handoff.plan_session_binding",
    "auth_handoff.bind_session",
    "auth_handoff.cancel_handoff",
    "auth_handoff.expire_handoff",
    "auth_handoff.plan_audit_export",
    "auth_handoff.audit_export_request",
    "auth_handoff.get_artifact_handle",
];

pub(crate) const AUTH_HANDOFF_PERMISSION_SCOPES: &[&str] = &[
    "identity.auth.handoff.start",
    "identity.auth.handoff.callback",
    "identity.auth.handoff.token_reference",
    "identity.auth.handoff.subject",
    "identity.auth.handoff.session_bind",
    "identity.auth.handoff.audit_export",
];

const AUTH_PROTOCOL_METADATA: &[(&str, &str)] = &[
    ("oauth2_pkce", "true"),
    ("oidc", "true"),
    ("saml", "descriptor"),
    ("webauthn", "descriptor"),
];
const AUTH_CALLBACK_METADATA: &[(&str, &str)] = &[
    ("redirect_allowlist", "required"),
    ("state_nonce_correlation", "hash_only"),
    ("replay_protection", "required"),
];
const AUTH_TOKEN_METADATA: &[(&str, &str)] = &[
    ("token_reference", "handle_only"),
    ("raw_tokens", "false"),
    ("session_binding", "approval_required"),
];
const AUTH_MOCK_METADATA: &[(&str, &str)] = &[
    ("handoffs", "synthetic"),
    ("callbacks", "synthetic"),
    ("callable", "false"),
];
const AUTH_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("handoffs", "false"),
    ("callbacks", "false"),
    ("reason", "provider_not_installed"),
];

const AUTH_HANDOFF_PROVIDER_CLASSES: &[IdentityProviderClass<'_>] = &[
    IdentityProviderClass {
        provider_class: "auth-protocol",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUTH_PROTOCOL_METADATA,
    },
    IdentityProviderClass {
        provider_class: "callback-verifier",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUTH_CALLBACK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "token-reference",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUTH_TOKEN_METADATA,
    },
    IdentityProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUTH_MOCK_METADATA,
    },
    IdentityProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: AUTH_UNAVAILABLE_METADATA,
    },
];

/// Build the auth-handoff descriptor without binding IdP, browser, session, or credential code.
pub fn identity_auth_handoff_pack_definition() -> DomainPackDefinition {
    identity_pack_definition(IdentityPackDescriptor {
        pack_id: IDENTITY_AUTH_HANDOFF_PACK_ID,
        child_change_id: "openspec:add-pack-identity-auth-handoff",
        docs_slug: "auth-handoff",
        sdk_slug: "auth.handoff",
        service_id: IDENTITY_AUTH_HANDOFF_SERVICE_ID,
        commands: IDENTITY_AUTH_HANDOFF_COMMANDS,
        permission_scopes: AUTH_HANDOFF_PERMISSION_SCOPES,
        provider_classes: AUTH_HANDOFF_PROVIDER_CLASSES,
        health_probe: "auth_handoff.inspect_provider",
        unavailable_reason: "identity_auth_handoff_provider_not_installed",
        replay_schema: "identity.auth_handoff.replay.v1",
        data_classification: "sensitive_auth_handoff_reference_metadata",
        retention_policy: "handoff_correlation_callback_subject_session_binding_and_audit_metadata_by_reference",
        redaction_policy: "authorization_codes_tokens_assertions_pkce_verifiers_client_secrets_cookies_callback_payloads_provider_payloads_private_keys_and_signatures_redacted",
        timeout_ms: 120_000,
        budget_units: 5,
        examples: &[
            "Declare `pack.identity.auth.handoff.v1` as optional until an auth handoff provider is installed.",
            "Use handoff handles, token references, subject evidence, and session-binding evidence instead of raw callback or token payloads.",
        ],
        migration_notes: &[
            "Auth handoff commands become callable only after an approved auth handoff service provider registers matching schemas.",
            "Account lifecycle, profile writes, organization membership, tenant policy, session storage, secrets, device prompts, and application login UI remain separate boundaries.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffScope {
    pub tenant_scope: String,
    pub provider_scope: String,
    pub handoff_ref: Option<String>,
    pub callback_ref: Option<String>,
    pub subject_ref: Option<String>,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffProviderCapability {
    pub provider_class: String,
    pub protocol_profiles: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub expires_at_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffAttribution {
    pub source_ref: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthProtocolProfile {
    pub profile_ref: String,
    pub protocol_kind: String,
    pub support_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectCallbackDescriptor {
    pub redirect_ref: String,
    pub callback_ref: String,
    pub binding_kind: String,
    pub allowlist_policy_ref: String,
    pub method: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffPlan {
    pub plan_ref: String,
    pub protocol: AuthProtocolProfile,
    pub redirect: RedirectCallbackDescriptor,
    pub scopes: BTreeSet<String>,
    pub hints: BTreeMap<String, String>,
    pub correlation: HandoffCorrelation,
    pub expires_at_epoch_ms: u64,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffRecord {
    pub handoff_ref: String,
    pub plan_ref: String,
    pub protocol_kind: String,
    pub state: String,
    pub provider_class: String,
    pub freshness: AuthHandoffFreshness,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffCorrelation {
    pub state_hash: Option<String>,
    pub nonce_hash: Option<String>,
    pub pkce_challenge_hash: Option<String>,
    pub relay_state_hash: Option<String>,
    pub webauthn_challenge_hash: Option<String>,
    pub device_code_ref: Option<String>,
    pub csrf_binding_ref: Option<String>,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackVerificationResult {
    pub verification_ref: String,
    pub verified: bool,
    pub issuer_ref: Option<String>,
    pub audience_ref: Option<String>,
    pub signature_ref: Option<String>,
    pub subject_ref: Option<String>,
    pub claim_refs: BTreeSet<String>,
    pub failure_reason: Option<String>,
    pub freshness: AuthHandoffFreshness,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenReference {
    pub token_ref: String,
    pub token_class: String,
    pub expires_at_epoch_ms: Option<u64>,
    pub scope_hints: BTreeSet<String>,
    pub storage_boundary_ref: String,
    pub refreshable: bool,
    pub redaction_class: String,
    pub access_policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionReference {
    pub assertion_ref: String,
    pub assertion_class: String,
    pub expires_at_epoch_ms: Option<u64>,
    pub claim_hints: BTreeSet<String>,
    pub storage_boundary_ref: String,
    pub redaction_class: String,
    pub access_policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectEvidence {
    pub subject_evidence_ref: String,
    pub provider_subject_ref: String,
    pub account_ref: Option<String>,
    pub profile_claim_refs: BTreeSet<String>,
    pub assurance_level: String,
    pub authentication_context: String,
    pub organization_hint_ref: Option<String>,
    pub tenant_hint_ref: Option<String>,
    pub freshness: AuthHandoffFreshness,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBindingEvidence {
    pub binding_ref: String,
    pub session_ref: String,
    pub subject_evidence_ref: String,
    pub approval_ref: Option<String>,
    pub expires_at_epoch_ms: Option<u64>,
    pub binding_state: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffAuditReference {
    pub event_ref: String,
    pub event_type: String,
    pub bounded_reason_code: String,
    pub checksum: String,
    pub replay_pointer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffAuditExportPlan {
    pub export_ref: String,
    pub scope_ref: String,
    pub format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffArtifactHandle {
    pub artifact_id: String,
    pub content_class: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_class: String,
    pub access_policy_ref: String,
}

define_identity_command_wrappers!(
    AuthHandoffInspectProviderCommand,
    AuthHandoffDescribeSchemaCommand,
    AuthHandoffPlanHandoffCommand,
    AuthHandoffStartHandoffCommand,
    AuthHandoffVerifyCallbackCommand,
    AuthHandoffExchangeTokenReferenceCommand,
    AuthHandoffInspectSubjectEvidenceCommand,
    AuthHandoffPlanSessionBindingCommand,
    AuthHandoffBindSessionCommand,
    AuthHandoffCancelHandoffCommand,
    AuthHandoffExpireHandoffCommand,
    AuthHandoffPlanAuditExportCommand,
    AuthHandoffAuditExportRequestCommand,
    AuthHandoffGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthHandoffResultStatus {
    Success,
    Partial,
    Accepted,
    ActionRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    ReplayRejected,
    ApprovalRequired,
    Expired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffResultEnvelope<T> {
    pub status: AuthHandoffResultStatus,
    pub data: Option<T>,
    pub page: Option<IdentityPackPage<T>>,
    pub error: Option<IdentityPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHandoffDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub plan_hash: String,
    pub callback_hash: String,
    pub token_hash: String,
    pub binding_hash: String,
    pub artifact_hash: String,
}

pub fn identity_auth_handoff_descriptor_hashes() -> AuthHandoffDescriptorHashes {
    let freshness = AuthHandoffFreshness {
        source_timestamp_epoch_ms: 1,
        expires_at_epoch_ms: Some(10),
        freshness_class: "fresh".into(),
    };
    let protocol = AuthProtocolProfile {
        profile_ref: "profile".into(),
        protocol_kind: "oidc_authorization_code".into(),
        support_metadata: BTreeMap::from([("pkce".into(), "required".into())]),
    };
    let correlation = HandoffCorrelation {
        state_hash: Some("state".into()),
        nonce_hash: Some("nonce".into()),
        pkce_challenge_hash: Some("pkce".into()),
        replay_pointer: "replay".into(),
        ..Default::default()
    };
    AuthHandoffDescriptorHashes {
        command_schema_hash: auth_handoff_stable_hash(&IDENTITY_AUTH_HANDOFF_COMMANDS),
        result_schema_hash: auth_handoff_stable_hash(&AuthHandoffResultStatus::Success),
        descriptor_hash: auth_handoff_stable_hash(&identity_auth_handoff_pack_definition()),
        provider_capability_hash: auth_handoff_stable_hash(&AuthHandoffProviderCapability {
            provider_class: "mock".into(),
            protocol_profiles: BTreeSet::from(["oidc_authorization_code".into()]),
            feature_flags: BTreeSet::from(["token_reference".into(), "session_bind".into()]),
            limits: BTreeMap::from([("max_pending_handoffs".into(), 100)]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        plan_hash: auth_handoff_stable_hash(&AuthHandoffPlan {
            plan_ref: "plan".into(),
            protocol,
            redirect: RedirectCallbackDescriptor {
                redirect_ref: "redirect".into(),
                callback_ref: "callback".into(),
                binding_kind: "redirect".into(),
                allowlist_policy_ref: "allowlist".into(),
                method: "GET".into(),
            },
            correlation,
            expires_at_epoch_ms: 10,
            redaction_class: "correlation_hash_only".into(),
            ..Default::default()
        }),
        callback_hash: auth_handoff_stable_hash(&CallbackVerificationResult {
            verification_ref: "verification".into(),
            verified: true,
            subject_ref: Some("subject".into()),
            freshness,
            replay_pointer: "replay".into(),
            ..Default::default()
        }),
        token_hash: auth_handoff_stable_hash(&TokenReference {
            token_ref: "token-ref".into(),
            token_class: "access_reference".into(),
            storage_boundary_ref: "secrets-reference".into(),
            refreshable: false,
            redaction_class: "handle_only".into(),
            access_policy_ref: "policy".into(),
            ..Default::default()
        }),
        binding_hash: auth_handoff_stable_hash(&SessionBindingEvidence {
            binding_ref: "binding".into(),
            session_ref: "session".into(),
            subject_evidence_ref: "subject-evidence".into(),
            binding_state: "planned".into(),
            replay_pointer: "replay".into(),
            ..Default::default()
        }),
        artifact_hash: auth_handoff_stable_hash(&AuthHandoffArtifactHandle {
            artifact_id: "artifact".into(),
            content_class: "audit_export".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
            retention_class: "short".into(),
            access_policy_ref: "policy".into(),
        }),
    }
}

pub fn auth_handoff_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    identity_stable_hash(value)
}
