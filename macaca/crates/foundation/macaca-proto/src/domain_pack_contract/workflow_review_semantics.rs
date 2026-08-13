//! Provider-neutral industrial review semantics.
//!
//! State, Specification, Memento, and Strategy patterns model bounded review
//! coordination facts. Concrete services retain persistence, identity lookup,
//! transport, and audit event emission; this protocol contains no raw content.

use serde::{Deserialize, Serialize};

/// Explicit lifecycle state for an individual review request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewLifecycleState {
    Requested,
    InReview,
    ChangesRequested,
    FixSubmitted,
    Approved,
    Dismissed,
    Stale,
    Closed,
    Cancelled,
}

/// Explicit lifecycle state for a preserved immutable finding record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFindingState {
    Open,
    Acknowledged,
    Fixed,
    Verified,
    Dismissed,
    Stale,
}

/// Versioned request memento containing only a bounded subject reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequestV1 {
    pub schema_version: String,
    pub request_ref: String,
    pub subject_ref: String,
    pub subject_revision_hash: String,
    pub requester_ref: String,
    pub state: ReviewLifecycleState,
    pub redaction_profile: String,
}

/// Immutable review-round evidence retained when a later round is requested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRoundV1 {
    pub round_ref: String,
    pub request_ref: String,
    pub round_index: u32,
    pub subject_revision_hash: String,
    pub prior_round_ref: Option<String>,
    pub replay_ref: String,
}

/// Immutable finding state with a predecessor reference rather than raw text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFindingV1 {
    pub finding_ref: String,
    pub request_ref: String,
    pub round_ref: String,
    pub severity: String,
    pub blocking: bool,
    pub state: ReviewFindingState,
    pub evidence_ref: String,
    pub prior_finding_ref: Option<String>,
    pub redaction_profile: String,
}

/// Remediation request that links finding references to bounded fix evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixRequestV1 {
    pub fix_ref: String,
    pub request_ref: String,
    pub finding_refs: Vec<String>,
    pub evidence_ref: String,
    pub submitted_revision_hash: String,
}

/// Terminal or provisional outcome evidence, including any dismissal proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewOutcomeV1 {
    pub outcome_ref: String,
    pub request_ref: String,
    pub state: ReviewLifecycleState,
    pub approved_revision_hash: Option<String>,
    pub dismissal_authority_ref: Option<String>,
    pub dismissal_reason_ref: Option<String>,
    pub replay_ref: String,
}

/// Bounded closure decision used before terminal work state is committed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewClosureGateV1 {
    pub gate_ref: String,
    pub request_ref: String,
    pub subject_revision_hash: String,
    pub outcome_ref: Option<String>,
    pub unresolved_blocking_finding_refs: Vec<String>,
    pub stale_revision: bool,
    pub replay_ref: String,
}

/// Revision carry-forward is an explicit provider-selected Strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRevisionPolicy {
    InvalidateOnChange,
    CarryForwardWithEvidence,
}

/// Deterministic result of a concurrent gate-affecting command race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewGateRaceResult {
    Approved,
    BlockedByFinding,
    LosingCommandRejected,
}

/// State Specification defining legal lifecycle transitions without a provider call.
pub struct ReviewLifecycleSpec;
impl ReviewLifecycleSpec {
    /// Return whether the lifecycle transition is legal for a generic review.
    pub fn allows(from: ReviewLifecycleState, to: ReviewLifecycleState) -> bool {
        matches!(
            (from, to),
            (
                ReviewLifecycleState::Requested,
                ReviewLifecycleState::InReview
            ) | (
                ReviewLifecycleState::InReview,
                ReviewLifecycleState::ChangesRequested
            ) | (
                ReviewLifecycleState::ChangesRequested,
                ReviewLifecycleState::FixSubmitted
            ) | (
                ReviewLifecycleState::FixSubmitted,
                ReviewLifecycleState::InReview
            ) | (
                ReviewLifecycleState::InReview,
                ReviewLifecycleState::Approved
            ) | (
                ReviewLifecycleState::InReview,
                ReviewLifecycleState::Dismissed
            ) | (ReviewLifecycleState::Approved, ReviewLifecycleState::Stale)
                | (ReviewLifecycleState::Stale, ReviewLifecycleState::InReview)
                | (ReviewLifecycleState::Approved, ReviewLifecycleState::Closed)
                | (
                    ReviewLifecycleState::Dismissed,
                    ReviewLifecycleState::Closed
                )
                | (
                    ReviewLifecycleState::Requested,
                    ReviewLifecycleState::Cancelled
                )
                | (
                    ReviewLifecycleState::InReview,
                    ReviewLifecycleState::Cancelled
                )
                | (
                    ReviewLifecycleState::ChangesRequested,
                    ReviewLifecycleState::Cancelled
                )
        )
    }
}

/// Finding State Specification preserves history by allowing transitions only forward.
pub struct ReviewFindingLifecycleSpec;
impl ReviewFindingLifecycleSpec {
    /// Return whether a finding can move to its new state without overwriting history.
    pub fn allows(from: ReviewFindingState, to: ReviewFindingState) -> bool {
        matches!(
            (from, to),
            (ReviewFindingState::Open, ReviewFindingState::Acknowledged)
                | (ReviewFindingState::Open, ReviewFindingState::Fixed)
                | (ReviewFindingState::Acknowledged, ReviewFindingState::Fixed)
                | (ReviewFindingState::Fixed, ReviewFindingState::Verified)
                | (ReviewFindingState::Open, ReviewFindingState::Dismissed)
                | (
                    ReviewFindingState::Acknowledged,
                    ReviewFindingState::Dismissed
                )
                | (ReviewFindingState::Fixed, ReviewFindingState::Dismissed)
                | (ReviewFindingState::Open, ReviewFindingState::Stale)
                | (ReviewFindingState::Acknowledged, ReviewFindingState::Stale)
                | (ReviewFindingState::Fixed, ReviewFindingState::Stale)
        )
    }
}

/// Revision Specification checks whether historical outcomes remain usable.
pub struct ReviewRevisionSpec;
impl ReviewRevisionSpec {
    /// Matching revisions are current; carry-forward requires explicit evidence.
    pub fn outcome_is_current(
        approved: Option<&str>,
        current: &str,
        policy: ReviewRevisionPolicy,
        evidence: Option<&str>,
    ) -> bool {
        match approved {
            Some(hash) if hash == current && !hash.is_empty() => true,
            Some(_) if matches!(policy, ReviewRevisionPolicy::CarryForwardWithEvidence) => {
                evidence.is_some_and(bounded)
            }
            _ => false,
        }
    }
}

/// Closure Specification prevents terminal state when evidence is stale or blocked.
pub struct ReviewClosureGateSpec;
impl ReviewClosureGateSpec {
    /// Evaluate all closure invariants from bounded, replayable evidence.
    pub fn can_close(gate: &ReviewClosureGateV1) -> bool {
        bounded(&gate.gate_ref)
            && bounded(&gate.request_ref)
            && bounded(&gate.subject_revision_hash)
            && gate.outcome_ref.as_deref().is_some_and(bounded)
            && gate.unresolved_blocking_finding_refs.is_empty()
            && !gate.stale_revision
            && bounded(&gate.replay_ref)
    }
}

/// Dismissal Specification protects state from unaudited removal.
pub struct ReviewDismissalSpec;
impl ReviewDismissalSpec {
    /// A dismissal requires policy authorization, a principal reference, and a reason reference.
    pub fn is_authorized(
        authority: Option<&str>,
        reason: Option<&str>,
        policy_allowed: bool,
    ) -> bool {
        policy_allowed && authority.is_some_and(bounded) && reason.is_some_and(bounded)
    }
}

/// Paginate an already authorized projection so hidden findings never affect page shape.
pub fn filtered_finding_page(
    authorized: &[ReviewFindingV1],
    cursor: Option<usize>,
    page_size: usize,
) -> (Vec<ReviewFindingV1>, Option<String>) {
    let start = cursor.unwrap_or(0).min(authorized.len());
    let end = start.saturating_add(page_size.max(1)).min(authorized.len());
    (
        authorized[start..end].to_vec(),
        (end < authorized.len()).then(|| format!("cursor:{end}")),
    )
}

/// Resolve concurrent approval and blocking-finding proposals deterministically.
/// Providers persist the losing diagnostic with a trace reference for replay.
pub fn resolve_gate_race(
    approval_sequence: u64,
    blocking_finding_sequence: u64,
) -> ReviewGateRaceResult {
    if blocking_finding_sequence <= approval_sequence {
        ReviewGateRaceResult::BlockedByFinding
    } else {
        ReviewGateRaceResult::Approved
    }
}

fn bounded(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.contains('\n')
}
