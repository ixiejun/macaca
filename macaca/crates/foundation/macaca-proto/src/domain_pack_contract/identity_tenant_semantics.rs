//! Provider-neutral tenant governance semantics.
//!
//! State, Specification, Strategy, and Memento patterns make tenant safety
//! deterministic before a directory, cloud, quota, or policy provider call.
//! The module stores only bounded references, hashes, counters, and replay refs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Lifecycle for a generic administrative or isolation tenant boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantLifecycle {
    Planned,
    Active,
    Suspended,
    Archived,
    Restored,
    Deleted,
}

/// State of a policy attachment without implementing the policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantPolicyAttachmentState {
    Planned,
    Attached,
    Detached,
    Conflict,
}

/// State of a quota reservation retained as a bounded memento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantQuotaReservationState {
    Planned,
    Reserved,
    Released,
    Expired,
    Rejected,
}

/// Bounded pre-provider evidence for a tenant command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantMutationEvidenceV1 {
    pub command: String,
    pub tenant_scope_ref: String,
    pub idempotency_key_hash: String,
    pub expected_version_hash: String,
    pub current_version_hash: String,
    pub policy_allowed: bool,
    pub entitlement_available: bool,
    pub provider_supported: bool,
    pub host_capability_enabled: bool,
    pub approval_ref: Option<String>,
    pub sensitive_config_reference: bool,
    pub secret_reference_safe: bool,
    pub residency_change: bool,
    pub reserved_units: BTreeMap<String, u64>,
    pub required_units: BTreeMap<String, u64>,
    pub replay_ref: String,
}

/// Typed provider-dispatch decision with no provider data leakage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantMutationDecision {
    Allowed,
    ApprovalRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    QuotaExceeded,
    SecretReferenceDenied,
}

/// Replayable rejection or acceptance explanation carried by reference only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantMutationDiagnosticV1 {
    pub decision: TenantMutationDecision,
    pub reason_code: String,
    pub replay_ref: String,
}

/// State Specification for tenant lifecycle, policy, and quota transitions.
pub struct TenantLifecycleSpec;
impl TenantLifecycleSpec {
    /// Check tenant lifecycle transitions without cloud or directory semantics.
    pub fn tenant_allows(from: TenantLifecycle, to: TenantLifecycle) -> bool {
        matches!(
            (from, to),
            (TenantLifecycle::Planned, TenantLifecycle::Active)
                | (TenantLifecycle::Active, TenantLifecycle::Suspended)
                | (TenantLifecycle::Suspended, TenantLifecycle::Active)
                | (TenantLifecycle::Active, TenantLifecycle::Archived)
                | (TenantLifecycle::Archived, TenantLifecycle::Restored)
                | (TenantLifecycle::Restored, TenantLifecycle::Active)
                | (TenantLifecycle::Archived, TenantLifecycle::Deleted)
        )
    }
    /// Check policy attachment transitions without evaluating policy content.
    pub fn policy_attachment_allows(
        from: TenantPolicyAttachmentState,
        to: TenantPolicyAttachmentState,
    ) -> bool {
        matches!(
            (from, to),
            (
                TenantPolicyAttachmentState::Planned,
                TenantPolicyAttachmentState::Attached
            ) | (
                TenantPolicyAttachmentState::Attached,
                TenantPolicyAttachmentState::Detached
            )
        )
    }
    /// Check quota reservation transitions, including an explicit release memento.
    pub fn quota_allows(
        from: TenantQuotaReservationState,
        to: TenantQuotaReservationState,
    ) -> bool {
        matches!(
            (from, to),
            (
                TenantQuotaReservationState::Planned,
                TenantQuotaReservationState::Reserved
            ) | (
                TenantQuotaReservationState::Planned,
                TenantQuotaReservationState::Rejected
            ) | (
                TenantQuotaReservationState::Reserved,
                TenantQuotaReservationState::Released
            ) | (
                TenantQuotaReservationState::Reserved,
                TenantQuotaReservationState::Expired
            )
        )
    }
}

/// Specification evaluated before any provider callback may run.
pub struct TenantMutationSpec;
impl TenantMutationSpec {
    /// Apply deterministic admission, availability, version, secret, approval, and budget checks.
    pub fn evaluate(evidence: &TenantMutationEvidenceV1) -> TenantMutationDiagnosticV1 {
        let decision = if !bounded(&evidence.command)
            || !bounded(&evidence.tenant_scope_ref)
            || !bounded(&evidence.replay_ref)
        {
            TenantMutationDecision::Denied
        } else if !evidence.policy_allowed {
            TenantMutationDecision::Denied
        } else if !evidence.entitlement_available || !evidence.host_capability_enabled {
            TenantMutationDecision::Unavailable
        } else if !evidence.provider_supported {
            TenantMutationDecision::Unsupported
        } else if !covers(&evidence.reserved_units, &evidence.required_units) {
            TenantMutationDecision::QuotaExceeded
        } else if !evidence.expected_version_hash.is_empty()
            && evidence.expected_version_hash != evidence.current_version_hash
        {
            TenantMutationDecision::StaleVersion
        } else if evidence.sensitive_config_reference && !evidence.secret_reference_safe {
            TenantMutationDecision::SecretReferenceDenied
        } else if requires_approval(
            &evidence.command,
            evidence.residency_change,
            evidence.sensitive_config_reference,
        ) && !evidence.approval_ref.as_deref().is_some_and(bounded)
        {
            TenantMutationDecision::ApprovalRequired
        } else if is_side_effect(&evidence.command) && !bounded(&evidence.idempotency_key_hash) {
            TenantMutationDecision::Conflict
        } else {
            TenantMutationDecision::Allowed
        };
        TenantMutationDiagnosticV1 {
            decision,
            reason_code: reason(decision).into(),
            replay_ref: evidence.replay_ref.clone(),
        }
    }
    /// Call a provider closure only after the complete tenant Specification passes.
    pub fn dispatch_after_validation<T>(
        evidence: &TenantMutationEvidenceV1,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, TenantMutationDiagnosticV1> {
        let diagnostic = Self::evaluate(evidence);
        if diagnostic.decision == TenantMutationDecision::Allowed {
            Ok(dispatch())
        } else {
            Err(diagnostic)
        }
    }
}

/// Page only the already-authorized projection so omitted tenants affect no cursor or count.
pub fn filtered_tenant_page<T: Clone>(
    authorized: &[T],
    cursor: Option<usize>,
    page_size: usize,
) -> (Vec<T>, Option<String>) {
    let start = cursor.unwrap_or(0).min(authorized.len());
    let end = start
        .saturating_add(page_size.clamp(1, 100))
        .min(authorized.len());
    (
        authorized[start..end].to_vec(),
        (end < authorized.len()).then(|| format!("cursor:{end}")),
    )
}

fn requires_approval(command: &str, residency_change: bool, sensitive_config: bool) -> bool {
    matches!(
        command,
        "tenant.create"
            | "tenant.request_lifecycle_transition"
            | "tenant.request_policy_attachment"
            | "tenant.export_audit"
    ) || (command == "tenant.update_config_reference" && sensitive_config)
        || residency_change
}
fn is_side_effect(command: &str) -> bool {
    !command.starts_with("tenant.plan_")
        && !matches!(
            command,
            "tenant.get"
                | "tenant.search"
                | "tenant.inspect_provider"
                | "tenant.discover_schema"
                | "tenant.inspect_isolation_policy"
                | "tenant.inspect_quota"
                | "tenant.snapshot_usage"
                | "tenant.inspect_residency"
                | "tenant.inspect_config"
                | "tenant.inspect_relationships"
                | "tenant.get_artifact"
        )
}
fn covers(reserved: &BTreeMap<String, u64>, required: &BTreeMap<String, u64>) -> bool {
    required
        .iter()
        .all(|(key, value)| reserved.get(key).unwrap_or(&0) >= value)
}
fn bounded(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.contains('\n')
}
fn reason(decision: TenantMutationDecision) -> &'static str {
    match decision {
        TenantMutationDecision::Allowed => "allowed",
        TenantMutationDecision::ApprovalRequired => "approval_required",
        TenantMutationDecision::Denied => "policy_denied",
        TenantMutationDecision::Unavailable => "provider_unavailable",
        TenantMutationDecision::Unsupported => "command_unsupported",
        TenantMutationDecision::Conflict => "idempotency_required",
        TenantMutationDecision::StaleVersion => "stale_version",
        TenantMutationDecision::QuotaExceeded => "resource_reservation_insufficient",
        TenantMutationDecision::SecretReferenceDenied => "secret_reference_denied",
    }
}
