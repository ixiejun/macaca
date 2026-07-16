use super::identity_account::{
    AccountAttributePatch, AccountAuditExportPlan, AccountFreshness, AccountIdentifier,
    AccountLifecycleTransitionPlan, AccountLifecycleTransitionRequest, AccountRecord,
    AccountRecoveryReference, AccountScope, LinkedIdentityReference,
};
use super::identity_common::IdentityPackCommandEnvelope;
use super::identity_validation::{
    bounded_identity_hash, bounded_identity_page, bounded_identity_reference,
    opaque_identity_artifact,
};

impl IdentityPackCommandEnvelope {
    /// Validate generic command correlation without interpreting application-owned parameters.
    pub fn has_bounded_identity_request(&self, max_parameters: usize, max_page_size: u32) -> bool {
        bounded_identity_reference(&self.subject_ref, 160)
            && self.parameters.len() <= max_parameters
            && self.parameters.iter().all(|(key, value)| {
                bounded_identity_reference(key, 96) && bounded_identity_reference(value, 256)
            })
            && self
                .cursor
                .as_deref()
                .is_none_or(|cursor| bounded_identity_reference(cursor, 256))
            && bounded_identity_page(self.page_size, max_page_size)
            && self
                .idempotency_key
                .as_deref()
                .is_none_or(|key| bounded_identity_reference(key, 160))
            && self
                .approval_ref
                .as_deref()
                .is_none_or(|approval| bounded_identity_reference(approval, 160))
    }

    /// Require mutation evidence structurally while leaving approval authorization
    /// and idempotency persistence to the service runtime.
    pub fn has_required_mutation_evidence(&self) -> bool {
        self.idempotency_key
            .as_deref()
            .is_some_and(|key| bounded_identity_reference(key, 160))
            && self
                .approval_ref
                .as_deref()
                .is_some_and(|approval| bounded_identity_reference(approval, 160))
    }
}

impl AccountScope {
    /// Check that a request names exactly one tenant and one provider boundary.
    /// The runtime uses these handles for policy lookup; this DTO layer prevents
    /// an ambiguous scope from reaching a provider adapter.
    pub fn is_isolated_scope(&self) -> bool {
        bounded_identity_reference(&self.tenant_scope, 160)
            && bounded_identity_reference(&self.provider_scope, 160)
            && bounded_identity_reference(&self.permission_scope, 160)
            && self
                .account_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
            && self
                .subject_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
            && self
                .identity_provider_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 160))
    }
}

impl AccountIdentifier {
    /// Validate hash-only account identifiers, excluding raw addresses and credentials.
    pub fn is_safe_reference(&self) -> bool {
        bounded_identity_reference(&self.identifier_ref, 160)
            && bounded_identity_reference(&self.identifier_kind, 64)
            && bounded_identity_hash(&self.normalized_value_hash)
            && matches!(
                self.verification_state.as_str(),
                "unverified" | "verified" | "revoked"
            )
            && bounded_identity_reference(&self.redaction_class, 96)
    }
}

impl AccountFreshness {
    /// Accept explicit current or stale evidence only, so callers can surface a
    /// typed stale-data result instead of silently treating cached data as fresh.
    pub fn is_valid_evidence(&self) -> bool {
        self.source_timestamp_epoch_ms > 0
            && self
                .cache_timestamp_epoch_ms
                .is_none_or(|cached| cached >= self.source_timestamp_epoch_ms)
            && matches!(
                self.freshness_class.as_str(),
                "current" | "stale" | "unknown"
            )
    }

    pub fn is_stale(&self) -> bool {
        self.freshness_class == "stale"
    }
}

impl LinkedIdentityReference {
    /// Validate linked identities as provider-neutral handles and lifecycle evidence.
    pub fn is_safe_reference(&self) -> bool {
        bounded_identity_reference(&self.link_ref, 160)
            && bounded_identity_reference(&self.provider_class, 96)
            && bounded_identity_reference(&self.issuer_ref, 256)
            && bounded_identity_reference(&self.external_subject_ref, 256)
            && matches!(
                self.link_state.as_str(),
                "linked" | "pending" | "unlinked" | "conflict"
            )
            && bounded_identity_reference(&self.replay_pointer, 256)
    }
}

impl AccountRecoveryReference {
    /// Keep recovery configuration reference-only and require one bounded route.
    /// Raw reset tokens, recovery codes, credentials, and contact values must be
    /// stored by the dedicated secrets or provider boundary, never in this DTO.
    pub fn is_sensitive_reference_only(&self) -> bool {
        bounded_identity_reference(&self.recovery_ref, 160)
            && matches!(
                self.recovery_kind.as_str(),
                "email" | "phone" | "reset_flow" | "support_case"
            )
            && bounded_identity_reference(&self.redaction_profile, 96)
            && [
                self.contact_ref.as_deref(),
                self.reset_flow_ref.as_deref(),
                self.support_case_ref.as_deref(),
            ]
            .into_iter()
            .flatten()
            .all(opaque_identity_artifact)
            && (self.contact_ref.is_some()
                || self.reset_flow_ref.is_some()
                || self.support_case_ref.is_some())
    }
}

impl AccountAuditExportPlan {
    /// Bound audit exports to a declared scope, format, and redaction profile.
    pub fn is_bounded_export(&self) -> bool {
        bounded_identity_reference(&self.export_ref, 160)
            && bounded_identity_reference(&self.scope_ref, 160)
            && matches!(self.format.as_str(), "json" | "csv" | "artifact")
            && bounded_identity_reference(&self.redaction_profile, 96)
            && bounded_identity_reference(&self.retention_class, 96)
    }
}

impl AccountAttributePatch {
    /// Reject profile-preference payloads and raw attribute values at the account boundary.
    pub fn is_minimized(&self, max_attributes: usize) -> bool {
        opaque_identity_artifact(&self.patch_ref)
            && !self.profile_preference_boundary
            && self.attributes.len() <= max_attributes
            && self.attributes.iter().all(|(key, value)| {
                bounded_identity_reference(key, 96) && opaque_identity_artifact(value)
            })
            && self
                .custom_schema_refs
                .iter()
                .all(|schema| bounded_identity_reference(schema, 160))
    }
}

impl AccountLifecycleTransitionPlan {
    /// Validate explicit lifecycle transitions before runtime approval and state checks.
    pub fn is_valid_transition(&self) -> bool {
        bounded_identity_reference(&self.plan_ref, 160)
            && matches!(
                self.from_state.as_str(),
                "active" | "suspended" | "disabled" | "pending"
            )
            && matches!(
                self.to_state.as_str(),
                "active" | "suspended" | "disabled" | "deleted"
            )
            && self.from_state != self.to_state
            && self
                .validation_diagnostics
                .iter()
                .all(|value| bounded_identity_reference(value, 160))
    }
}

impl AccountLifecycleTransitionRequest {
    /// Require bounded idempotency, version, and approval evidence for a transition request.
    pub fn has_safe_preconditions(&self, plan: &AccountLifecycleTransitionPlan) -> bool {
        plan.is_valid_transition()
            && self.plan_ref == plan.plan_ref
            && bounded_identity_hash(&self.idempotency_key_hash)
            && bounded_identity_hash(&self.version_token_hash)
            && (!plan.requires_approval
                || self
                    .approval_ref
                    .as_deref()
                    .is_some_and(|approval| bounded_identity_reference(approval, 160)))
    }
}

impl AccountRecord {
    /// Validate a redacted account projection before trace, audit, or SDK use.
    pub fn has_safe_projection(&self, max_identifiers: usize, max_links: usize) -> bool {
        self.is_bounded(max_identifiers, max_links)
            && bounded_identity_reference(&self.account_ref, 160)
            && bounded_identity_reference(&self.stable_subject_ref, 160)
            && self
                .identifiers
                .iter()
                .all(AccountIdentifier::is_safe_reference)
            && self
                .linked_identities
                .iter()
                .all(LinkedIdentityReference::is_safe_reference)
            && self
                .identifiers
                .iter()
                .map(|identifier| {
                    (
                        &identifier.identifier_kind,
                        &identifier.normalized_value_hash,
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.identifiers.len()
            && self
                .linked_identities
                .iter()
                .map(|identity| {
                    (
                        &identity.provider_class,
                        &identity.issuer_ref,
                        &identity.external_subject_ref,
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.linked_identities.len()
            && self
                .recovery_refs
                .iter()
                .all(AccountRecoveryReference::is_sensitive_reference_only)
            && self.freshness.is_valid_evidence()
            && bounded_identity_hash(&self.version_token_hash)
            && bounded_identity_reference(&self.redaction_class, 96)
    }

    /// Ensure a returned record cannot span tenant boundaries.  Providers may
    /// retain their own tenancy model, but a service result is admissible only
    /// when every projected tenant handle equals the requested tenant scope.
    pub fn is_isolated_to(&self, scope: &AccountScope) -> bool {
        scope.is_isolated_scope()
            && !self.tenant_refs.is_empty()
            && self.tenant_refs.len() == 1
            && self
                .tenant_refs
                .iter()
                .all(|tenant| tenant == &scope.tenant_scope)
    }
}
