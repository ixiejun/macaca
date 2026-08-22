use super::identity_auth_handoff::{
    AssertionReference, AuthHandoffArtifactHandle, AuthHandoffAuditExportPlan,
    AuthHandoffFreshness, AuthHandoffPlan, AuthProtocolProfile, CallbackVerificationResult,
    HandoffCorrelation, RedirectCallbackDescriptor, SessionBindingEvidence, SubjectEvidence,
    TokenReference,
};
use super::identity_validation::{bounded_identity_reference, opaque_identity_artifact};

impl AuthProtocolProfile {
    /// Limit protocol metadata to bounded declarative capability labels.
    pub fn is_supported_shape(&self) -> bool {
        bounded_identity_reference(&self.profile_ref, 160)
            && matches!(
                self.protocol_kind.as_str(),
                "oauth2_authorization_code"
                    | "oidc_authorization_code"
                    | "saml_web_sso"
                    | "webauthn_assertion"
                    | "passkey"
                    | "device_code"
                    | "magic_link"
                    | "custom"
            )
            && self.support_metadata.len() <= 32
            && self.support_metadata.iter().all(|(key, value)| {
                bounded_identity_reference(key, 96) && bounded_identity_reference(value, 160)
            })
    }
}

impl RedirectCallbackDescriptor {
    /// Allow only registered callback handles, never raw redirect or callback URLs.
    pub fn is_allowlisted_reference(&self) -> bool {
        bounded_identity_reference(&self.redirect_ref, 256)
            && bounded_identity_reference(&self.callback_ref, 256)
            && bounded_identity_reference(&self.allowlist_policy_ref, 160)
            && matches!(
                self.binding_kind.as_str(),
                "redirect" | "device" | "passkey"
            )
            && matches!(self.method.as_str(), "GET" | "POST")
    }
}

impl HandoffCorrelation {
    /// Require correlation evidence to be hash/reference-only for replay protection.
    pub fn is_safe_correlation(&self) -> bool {
        [
            self.state_hash.as_deref(),
            self.nonce_hash.as_deref(),
            self.pkce_challenge_hash.as_deref(),
            self.relay_state_hash.as_deref(),
            self.webauthn_challenge_hash.as_deref(),
            self.device_code_ref.as_deref(),
            self.csrf_binding_ref.as_deref(),
        ]
        .into_iter()
        .flatten()
        .all(|value| bounded_identity_reference(value, 256))
            && bounded_identity_reference(&self.replay_pointer, 256)
    }

    /// Require the protocol-specific replay binding that makes a callback
    /// correlatable without retaining raw state, nonce, or challenge material.
    pub fn is_complete_for(&self, protocol: &AuthProtocolProfile) -> bool {
        self.is_safe_correlation()
            && match protocol.protocol_kind.as_str() {
                "oauth2_authorization_code" => {
                    self.state_hash.is_some() && self.pkce_challenge_hash.is_some()
                }
                "oidc_authorization_code" => self.state_hash.is_some() && self.nonce_hash.is_some(),
                "saml_web_sso" => self.relay_state_hash.is_some(),
                "webauthn_assertion" | "passkey" => self.webauthn_challenge_hash.is_some(),
                "device_code" => self.device_code_ref.is_some() && self.csrf_binding_ref.is_some(),
                "magic_link" | "custom" => {
                    self.state_hash.is_some() || self.csrf_binding_ref.is_some()
                }
                _ => false,
            }
    }
}

impl AuthHandoffFreshness {
    /// Preserve an explicit freshness state so callback and evidence consumers
    /// can produce stale-data outcomes rather than over-trusting cached claims.
    pub fn is_valid_evidence(&self, now_epoch_ms: u64) -> bool {
        self.source_timestamp_epoch_ms > 0
            && self
                .expires_at_epoch_ms
                .is_none_or(|expiry| expiry > now_epoch_ms)
            && matches!(self.freshness_class.as_str(), "fresh" | "stale" | "unknown")
    }
}

impl AuthHandoffPlan {
    /// Validate protocol, callback, correlation, scopes, and expiry before provider dispatch.
    pub fn has_safe_preconditions(&self, now_epoch_ms: u64, max_scopes: usize) -> bool {
        bounded_identity_reference(&self.plan_ref, 160)
            && self.protocol.is_supported_shape()
            && self.redirect.is_allowlisted_reference()
            && !self.scopes.is_empty()
            && self.scopes.len() <= max_scopes
            && self
                .scopes
                .iter()
                .all(|scope| bounded_identity_reference(scope, 160))
            && self.correlation.is_complete_for(&self.protocol)
            && self.expires_at_epoch_ms > now_epoch_ms
            && bounded_identity_reference(&self.redaction_class, 96)
    }
}

impl CallbackVerificationResult {
    /// Accept a successful callback only when every verification result is a
    /// bounded reference and a subject was established.  Cryptographic checking
    /// remains provider-owned; this verifies the provider-neutral evidence shape.
    pub fn has_complete_verified_evidence(&self, now_epoch_ms: u64) -> bool {
        bounded_identity_reference(&self.verification_ref, 160)
            && self.verified
            && self
                .issuer_ref
                .as_deref()
                .is_some_and(|value| bounded_identity_reference(value, 256))
            && self
                .audience_ref
                .as_deref()
                .is_some_and(|value| bounded_identity_reference(value, 256))
            && self
                .signature_ref
                .as_deref()
                .is_some_and(opaque_identity_artifact)
            && self
                .subject_ref
                .as_deref()
                .is_some_and(|value| bounded_identity_reference(value, 160))
            && self
                .claim_refs
                .iter()
                .all(|value| bounded_identity_reference(value, 160))
            && self.failure_reason.is_none()
            && self.freshness.is_valid_evidence(now_epoch_ms)
            && bounded_identity_reference(&self.replay_pointer, 256)
    }
}

impl TokenReference {
    /// Keep token exchange results as storage-bound references, never raw token material.
    pub fn is_safe_reference(&self, now_epoch_ms: u64) -> bool {
        opaque_identity_artifact(&self.token_ref)
            && bounded_identity_reference(&self.token_class, 96)
            && self
                .expires_at_epoch_ms
                .is_none_or(|expiry| expiry > now_epoch_ms)
            && self
                .scope_hints
                .iter()
                .all(|scope| bounded_identity_reference(scope, 160))
            && bounded_identity_reference(&self.storage_boundary_ref, 160)
            && bounded_identity_reference(&self.redaction_class, 96)
            && bounded_identity_reference(&self.access_policy_ref, 160)
    }
}

impl AssertionReference {
    /// Keep assertion handling on the same opaque storage boundary as tokens.
    pub fn is_safe_reference(&self, now_epoch_ms: u64) -> bool {
        opaque_identity_artifact(&self.assertion_ref)
            && bounded_identity_reference(&self.assertion_class, 96)
            && self
                .expires_at_epoch_ms
                .is_none_or(|expiry| expiry > now_epoch_ms)
            && self
                .claim_hints
                .iter()
                .all(|hint| bounded_identity_reference(hint, 160))
            && bounded_identity_reference(&self.storage_boundary_ref, 160)
            && bounded_identity_reference(&self.redaction_class, 96)
            && bounded_identity_reference(&self.access_policy_ref, 160)
    }
}

impl SubjectEvidence {
    /// Validate account and tenant hints as references, without owning account,
    /// profile, organization, or tenant authorization semantics.
    pub fn is_safe_reference(&self, now_epoch_ms: u64) -> bool {
        bounded_identity_reference(&self.subject_evidence_ref, 160)
            && bounded_identity_reference(&self.provider_subject_ref, 160)
            && self
                .account_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
            && self
                .profile_claim_refs
                .iter()
                .all(|value| bounded_identity_reference(value, 160))
            && bounded_identity_reference(&self.assurance_level, 96)
            && bounded_identity_reference(&self.authentication_context, 160)
            && self
                .organization_hint_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
            && self
                .tenant_hint_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
            && self.freshness.is_valid_evidence(now_epoch_ms)
            && bounded_identity_reference(&self.redaction_class, 96)
    }
}

impl SessionBindingEvidence {
    /// Validate session binding evidence without exposing session cookies or assertion payloads.
    pub fn is_safe_reference(&self, now_epoch_ms: u64) -> bool {
        bounded_identity_reference(&self.binding_ref, 160)
            && bounded_identity_reference(&self.session_ref, 160)
            && bounded_identity_reference(&self.subject_evidence_ref, 160)
            && self
                .expires_at_epoch_ms
                .is_none_or(|expiry| expiry > now_epoch_ms)
            && matches!(
                self.binding_state.as_str(),
                "planned" | "bound" | "revoked" | "expired"
            )
            && bounded_identity_reference(&self.replay_pointer, 256)
    }

    /// Binding a session is approval-sensitive.  This checks only that durable
    /// approval evidence exists; policy authorization is evaluated by runtime.
    pub fn has_approval_evidence(&self) -> bool {
        self.approval_ref
            .as_deref()
            .is_some_and(|approval| bounded_identity_reference(approval, 160))
    }
}

impl AuthHandoffAuditExportPlan {
    /// Constrain retained audit output to a small declared artifact shape.
    pub fn is_bounded_export(&self) -> bool {
        bounded_identity_reference(&self.export_ref, 160)
            && bounded_identity_reference(&self.scope_ref, 160)
            && matches!(self.format.as_str(), "json" | "csv" | "artifact")
            && bounded_identity_reference(&self.redaction_profile, 96)
    }
}

impl AuthHandoffArtifactHandle {
    /// Represent asynchronous audit output as an expiring handle, never bytes.
    pub fn is_safe_async_result(&self, now_epoch_ms: u64) -> bool {
        opaque_identity_artifact(&self.artifact_id)
            && bounded_identity_reference(&self.content_class, 96)
            && bounded_identity_reference(&self.checksum, 256)
            && self.expires_at_epoch_ms > now_epoch_ms
            && bounded_identity_reference(&self.retention_class, 96)
            && bounded_identity_reference(&self.access_policy_ref, 160)
    }
}

use super::identity_auth_handoff::{AUTH_HANDOFF_PERMISSION_SCOPES, IDENTITY_AUTH_HANDOFF_PACK_ID};
use super::model::AppServiceContractConfig;

/// Reject auth-handoff scopes outside the descriptor-owned vocabulary.
pub fn validate_identity_auth_handoff_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let Some(scopes) = declaration
        .pack_permission_scopes
        .get(IDENTITY_AUTH_HANDOFF_PACK_ID)
    else {
        return Ok(());
    };
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack| pack == IDENTITY_AUTH_HANDOFF_PACK_ID);
    if !declared {
        return Err("auth handoff permissions require the auth handoff pack");
    }
    if scopes
        .iter()
        .any(|scope| !AUTH_HANDOFF_PERMISSION_SCOPES.contains(&scope.as_str()))
    {
        return Err("auth handoff permission scope is not declared by the pack");
    }
    Ok(())
}

#[cfg(test)]
mod permission_tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn auth_handoff_permissions_are_descriptor_owned() {
        let valid = AppServiceContractConfig {
            optional_packs: vec![IDENTITY_AUTH_HANDOFF_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                IDENTITY_AUTH_HANDOFF_PACK_ID.into(),
                BTreeSet::from(["identity.auth.handoff.start".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_identity_auth_handoff_permission_declarations(&valid).is_ok());
        let invalid = AppServiceContractConfig {
            optional_packs: vec![IDENTITY_AUTH_HANDOFF_PACK_ID.into()],
            pack_permission_scopes: BTreeMap::from([(
                IDENTITY_AUTH_HANDOFF_PACK_ID.into(),
                BTreeSet::from(["identity.auth.native".into()]),
            )]),
            ..Default::default()
        };
        assert!(validate_identity_auth_handoff_permission_declarations(&invalid).is_err());
    }
}
