//! Provider-neutral approval semantics for the workflow approval pack.
//!
//! This module uses the State, Specification, Memento, and Idempotency
//! patterns. It owns only bounded references and hashes; a concrete approval
//! provider remains responsible for persistence, identity lookup, and transport.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Versioned lifecycle state for an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalLifecycleState {
    Requested,
    Pending,
    Claimed,
    Escalated,
    Decided,
    Expired,
    Cancelled,
    Consumed,
}

/// A bounded approval request and its redaction profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestV1 {
    pub schema_version: String,
    pub request_ref: String,
    pub subject_ref: String,
    pub policy_hash: String,
    pub requester_ref: String,
    pub state: ApprovalLifecycleState,
    pub deadline_epoch_ms: Option<u64>,
    pub redaction_profile: String,
}

/// Assignment history is retained as references so escalation remains auditable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAssignmentV1 {
    pub assignment_ref: String,
    pub request_ref: String,
    pub eligible_principal_refs: BTreeSet<String>,
    pub claimed_by_ref: Option<String>,
    pub escalated_from_ref: Option<String>,
}

/// Decision evidence contains no raw prompt, identity, or provider payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionV1 {
    pub decision_ref: String,
    pub request_ref: String,
    pub approver_ref: String,
    pub outcome: String,
    pub policy_hash: String,
    pub source_trace_ref: String,
    pub expires_at_epoch_ms: Option<u64>,
    pub consumed: bool,
}

/// Evidence bundle memento used to reconstruct a decision without raw content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEvidenceBundleV1 {
    pub evidence_ref: String,
    pub request_ref: String,
    pub evidence_hash: String,
    pub source_trace_ref: String,
    pub redaction_profile: String,
}

/// Gate checked by a protected side-effect service before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionGateV1 {
    pub gate_ref: String,
    pub request_ref: String,
    pub subject_ref: String,
    pub required_outcome: String,
    pub policy_hash: String,
    pub decision: Option<ApprovalDecisionV1>,
    pub valid_until_epoch_ms: Option<u64>,
    pub consumption_mode: ApprovalConsumptionMode,
}

/// Whether a decision can be consumed once or repeatedly until expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalConsumptionMode {
    SingleUse,
    Reusable,
}

/// Result of a duplicate request or decision idempotency check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalIdempotencyResult<T> {
    New(T),
    Replay(T),
    Conflict,
}

/// Compare bounded idempotency evidence without exposing the original payload.
pub fn check_idempotency(
    existing_key: Option<&str>,
    existing_payload_hash: Option<&str>,
    request_key: &str,
    request_payload_hash: &str,
) -> ApprovalIdempotencyResult<()> {
    match (existing_key, existing_payload_hash) {
        (Some(key), Some(hash)) if key == request_key && hash == request_payload_hash => {
            ApprovalIdempotencyResult::Replay(())
        }
        (Some(key), _) if key == request_key => ApprovalIdempotencyResult::Conflict,
        _ => ApprovalIdempotencyResult::New(()),
    }
}

/// Clock-free deadline Specification for deterministic expiry behavior.
pub struct ApprovalDeadlineSpec;

impl ApprovalDeadlineSpec {
    pub fn is_expired(deadline_epoch_ms: Option<u64>, now_epoch_ms: u64) -> bool {
        deadline_epoch_ms.is_some_and(|deadline| now_epoch_ms >= deadline)
    }
}

/// Sanitized pending item used after policy filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPendingProjection {
    pub approval_ref: String,
    pub state: ApprovalLifecycleState,
    pub replay_ref: String,
}

/// Paginate authorized projections using an opaque cursor only.
pub fn filtered_pending_page(
    items: Vec<ApprovalPendingProjection>,
    cursor: Option<usize>,
    page_size: usize,
) -> (Vec<ApprovalPendingProjection>, Option<String>) {
    let start = cursor.unwrap_or(0).min(items.len());
    let width = page_size.max(1);
    let end = start.saturating_add(width).min(items.len());
    let next = (end < items.len()).then_some(end);
    (
        items[start..end].to_vec(),
        next.map(|value| format!("cursor:{value}")),
    )
}

/// State Specification for legal lifecycle transitions.
pub struct ApprovalLifecycleSpec;

impl ApprovalLifecycleSpec {
    /// Return whether a transition is legal without consulting a provider.
    pub fn allows(from: ApprovalLifecycleState, to: ApprovalLifecycleState) -> bool {
        matches!(
            (from, to),
            (
                ApprovalLifecycleState::Requested,
                ApprovalLifecycleState::Pending
            ) | (
                ApprovalLifecycleState::Pending,
                ApprovalLifecycleState::Claimed
            ) | (
                ApprovalLifecycleState::Pending,
                ApprovalLifecycleState::Escalated
            ) | (
                ApprovalLifecycleState::Pending,
                ApprovalLifecycleState::Decided
            ) | (
                ApprovalLifecycleState::Claimed,
                ApprovalLifecycleState::Decided
            ) | (
                ApprovalLifecycleState::Claimed,
                ApprovalLifecycleState::Escalated
            ) | (
                ApprovalLifecycleState::Pending,
                ApprovalLifecycleState::Expired
            ) | (
                ApprovalLifecycleState::Claimed,
                ApprovalLifecycleState::Expired
            ) | (
                ApprovalLifecycleState::Pending,
                ApprovalLifecycleState::Cancelled
            ) | (
                ApprovalLifecycleState::Claimed,
                ApprovalLifecycleState::Cancelled
            ) | (
                ApprovalLifecycleState::Decided,
                ApprovalLifecycleState::Consumed
            )
        )
    }

    /// Return the first legal terminal transition; later races are rejected.
    pub fn terminal_race_winner(
        current: ApprovalLifecycleState,
        proposed: ApprovalLifecycleState,
    ) -> Option<ApprovalLifecycleState> {
        if matches!(
            current,
            ApprovalLifecycleState::Pending | ApprovalLifecycleState::Claimed
        ) && matches!(
            proposed,
            ApprovalLifecycleState::Decided
                | ApprovalLifecycleState::Expired
                | ApprovalLifecycleState::Cancelled
        ) {
            Some(proposed)
        } else {
            None
        }
    }
}

/// Eligibility facts evaluated again when a decision is submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalEligibilityEvidence {
    pub principal_ref: String,
    pub identity_active: bool,
    pub membership_active: bool,
    pub delegation_active: bool,
    pub tenant_matches: bool,
}

/// Eligibility Specification used at decision time.
pub struct ApprovalEligibilitySpec;

impl ApprovalEligibilitySpec {
    pub fn accepts(evidence: &ApprovalEligibilityEvidence) -> bool {
        !evidence.principal_ref.trim().is_empty()
            && evidence.identity_active
            && evidence.membership_active
            && evidence.delegation_active
            && evidence.tenant_matches
    }

    /// Re-check that the principal is still eligible for the current assignment.
    pub fn is_assigned(
        evidence: &ApprovalEligibilityEvidence,
        assignment: &ApprovalAssignmentV1,
    ) -> bool {
        Self::accepts(evidence)
            && !assignment.request_ref.is_empty()
            && assignment
                .eligible_principal_refs
                .contains(&evidence.principal_ref)
    }
}

impl ApprovalAssignmentV1 {
    /// Create an escalation assignment while retaining the previous audit ref.
    pub fn escalate(&self, assignment_ref: impl Into<String>, eligible: BTreeSet<String>) -> Self {
        Self {
            assignment_ref: assignment_ref.into(),
            request_ref: self.request_ref.clone(),
            eligible_principal_refs: eligible,
            claimed_by_ref: None,
            escalated_from_ref: Some(self.assignment_ref.clone()),
        }
    }
}

impl ApprovalDecisionGateV1 {
    /// Check subject, policy, outcome, expiry, and consumption without wall-clock access.
    pub fn evaluate(&self, subject_ref: &str, policy_hash: &str, now_epoch_ms: u64) -> bool {
        let Some(decision) = &self.decision else {
            return false;
        };
        decision.request_ref == self.request_ref
            && subject_ref == self.subject_ref
            && policy_hash == self.policy_hash
            && decision.policy_hash == self.policy_hash
            && decision.outcome == self.required_outcome
            && self
                .valid_until_epoch_ms
                .is_none_or(|deadline| now_epoch_ms <= deadline)
            && decision
                .expires_at_epoch_ms
                .is_none_or(|deadline| now_epoch_ms <= deadline)
            && !decision.consumed
    }

    /// Consume a valid gate; single-use gates become invalid after this call.
    pub fn consume(&mut self, subject_ref: &str, policy_hash: &str, now_epoch_ms: u64) -> bool {
        if !self.evaluate(subject_ref, policy_hash, now_epoch_ms) {
            return false;
        }
        if let Some(decision) = &mut self.decision {
            if matches!(self.consumption_mode, ApprovalConsumptionMode::SingleUse) {
                decision.consumed = true;
            }
        }
        true
    }
}
