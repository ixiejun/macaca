use super::commerce_entitlement::{
    CommerceEntitlementState, EntitlementArtifactHandle, EntitlementDimension,
    EntitlementEventReference, EntitlementFreshness, EntitlementGrant, EntitlementProofExportPlan,
    EntitlementResource, EntitlementSeatAssignment, EntitlementSourceEvidence, EntitlementSubject,
    EntitlementUsageBalance, EntitlementUsageRecord,
};

impl EntitlementFreshness {
    /// Return whether source or event evidence should produce a stale-data result.
    pub fn has_stale_data(&self) -> bool {
        matches!(self.freshness_class.as_str(), "stale" | "expired")
    }
}

impl EntitlementSubject {
    /// Validate subject identity remains an isolated reference, not application business logic.
    pub fn is_isolated_reference(&self) -> bool {
        bounded_entitlement_token(&self.subject_ref, 160)
            && matches!(
                self.subject_kind.as_str(),
                "account"
                    | "profile"
                    | "organization"
                    | "tenant"
                    | "device"
                    | "installation"
                    | "agent"
                    | "service_account"
                    | "external_customer"
            )
            && bounded_entitlement_token(&self.redaction_class, 96)
    }
}

impl EntitlementResource {
    /// Validate resource identity is generic and not an app-specific feature gate.
    pub fn is_isolated_reference(&self) -> bool {
        bounded_entitlement_token(&self.resource_ref, 160)
            && matches!(
                self.resource_kind.as_str(),
                "product"
                    | "sku"
                    | "feature"
                    | "plan"
                    | "offering"
                    | "license"
                    | "subscription"
                    | "seat_pool"
                    | "usage_credit"
                    | "content_item"
                    | "capability"
                    | "external_resource"
            )
            && self
                .external_resource_ref
                .as_deref()
                .is_none_or(|reference| bounded_entitlement_token(reference, 256))
    }
}

impl EntitlementDimension {
    /// Validate usage dimensions against generic entitlement units.
    pub fn is_supported(&self) -> bool {
        bounded_entitlement_token(&self.dimension_ref, 160)
            && matches!(
                self.dimension_kind.as_str(),
                "seats"
                    | "requests"
                    | "tokens"
                    | "storage"
                    | "time"
                    | "ownership"
                    | "subscription_period"
                    | "credit"
                    | "custom"
            )
            && bounded_entitlement_token(&self.unit, 64)
    }
}

impl EntitlementSourceEvidence {
    /// Validate source evidence uses authority references, not raw purchase tokens or payloads.
    pub fn is_visible_authority(&self) -> bool {
        bounded_entitlement_token(&self.source_ref, 160)
            && matches!(
                self.source_kind.as_str(),
                "order"
                    | "payment"
                    | "receipt"
                    | "invoice"
                    | "subscription"
                    | "app_store_transaction"
                    | "purchase_token"
                    | "license"
                    | "manual_grant"
                    | "support_override"
                    | "provider_event"
                    | "migration"
            )
            && bounded_entitlement_token(&self.authority_ref, 160)
            && bounded_entitlement_token(&self.redaction_class, 96)
    }
}

impl CommerceEntitlementState {
    /// Validate entitlement state names without hardcoding application feature behavior.
    pub fn is_supported(&self) -> bool {
        matches!(
            self.state.as_str(),
            "active"
                | "trial"
                | "grace"
                | "pending_payment"
                | "pending_acknowledgement"
                | "paused"
                | "suspended"
                | "expired"
                | "revoked"
                | "refunded"
                | "transferred"
                | "consumed"
                | "unknown"
        )
    }
}

impl EntitlementGrant {
    /// Validate grant/revoke/suspend/transfer planning before provider dispatch.
    pub fn has_grant_preconditions(&self) -> bool {
        bounded_entitlement_token(&self.grant_ref, 160)
            && self.subject.is_isolated_reference()
            && self.resource.is_isolated_reference()
            && !self.dimensions.is_empty()
            && self
                .dimensions
                .iter()
                .all(EntitlementDimension::is_supported)
            && self.state.is_supported()
            && self.quantity > 0
            && self
                .valid_from_epoch_ms
                .zip(self.valid_until_epoch_ms)
                .is_none_or(|(start, end)| start <= end)
            && self
                .source_evidence
                .iter()
                .all(EntitlementSourceEvidence::is_visible_authority)
            && !self.freshness.has_stale_data()
            && self
                .transfer_history_refs
                .iter()
                .all(|reference| bounded_entitlement_token(reference, 160))
    }
}

impl EntitlementSeatAssignment {
    /// Validate seat assignment quantity and audit evidence before mutation.
    pub fn is_within_limit(&self, max_quantity: i64) -> bool {
        bounded_entitlement_token(&self.assignment_ref, 160)
            && bounded_entitlement_token(&self.seat_pool_ref, 160)
            && bounded_entitlement_token(&self.assignee_ref, 160)
            && self.quantity > 0
            && self.quantity <= max_quantity
            && matches!(
                self.assignment_state.as_str(),
                "pending" | "assigned" | "released"
            )
    }
}

impl EntitlementUsageRecord {
    /// Validate usage metering with idempotency and bounded dimensions.
    pub fn has_recording_preconditions(&self) -> bool {
        bounded_entitlement_token(&self.usage_ref, 160)
            && self.dimension.is_supported()
            && self.quantity > 0
            && bounded_entitlement_token(&self.idempotency_key_hash, 256)
            && bounded_entitlement_token(&self.source_evidence_ref, 160)
            && self
                .freshness
                .as_ref()
                .is_none_or(|freshness| !freshness.has_stale_data())
    }
}

impl EntitlementUsageBalance {
    /// Validate usage balance does not exceed a declared limit.
    pub fn is_within_limit(&self) -> bool {
        bounded_entitlement_token(&self.dimension_ref, 160)
            && self.balance >= 0
            && self.limit.is_none_or(|limit| self.balance <= limit)
    }
}

impl EntitlementEventReference {
    /// Validate event references are fresh, replayable, and webhook-body free.
    pub fn is_fresh_reference(&self) -> bool {
        bounded_entitlement_token(&self.event_ref, 160)
            && bounded_entitlement_token(&self.provider_class, 96)
            && bounded_entitlement_token(&self.event_type, 96)
            && self.event_timestamp_epoch_ms > 0
            && bounded_entitlement_token(&self.delivery_id_hash, 256)
            && !self.webhook_freshness.has_stale_data()
            && bounded_entitlement_token(&self.replay_pointer, 256)
            && bounded_entitlement_token(&self.bounded_result_code, 96)
    }
}

impl EntitlementProofExportPlan {
    /// Validate proof export planning uses bounded proof handles and redaction metadata.
    pub fn is_bounded_plan(&self) -> bool {
        bounded_entitlement_token(&self.export_ref, 160)
            && matches!(self.proof_type.as_str(), "json" | "license" | "ownership")
            && bounded_entitlement_token(&self.scope_ref, 160)
            && bounded_entitlement_token(&self.retention_class, 96)
            && bounded_entitlement_token(&self.redaction_profile, 160)
            && bounded_entitlement_token(&self.replay_pointer, 256)
    }
}

impl EntitlementArtifactHandle {
    /// Validate proof artifact handles without exposing signed payloads or provider bodies.
    pub fn is_bounded_artifact(&self) -> bool {
        bounded_entitlement_token(&self.artifact_id, 160)
            && matches!(self.proof_type.as_str(), "json" | "license" | "ownership")
            && bounded_entitlement_token(&self.checksum, 256)
            && self.expires_at_epoch_ms > 0
            && bounded_entitlement_token(&self.retention_class, 96)
            && bounded_entitlement_token(&self.redaction_profile, 160)
            && bounded_entitlement_token(&self.access_policy_ref, 160)
            && bounded_entitlement_token(&self.replay_pointer, 256)
    }
}

fn bounded_entitlement_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.chars().any(char::is_control)
        && !value.contains("://")
}
