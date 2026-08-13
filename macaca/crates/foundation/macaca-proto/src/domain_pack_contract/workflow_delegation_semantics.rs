//! Provider-neutral delegation semantics.
//!
//! The module applies State, Lease, Specification, and Memento patterns. It
//! models coordination facts only; workers, agents, schedulers, and provider
//! transports remain outside the protocol layer.

use serde::{Deserialize, Serialize};

/// Versioned lifecycle for delegated work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationLifecycleState {
    Requested,
    Queued,
    Claimed,
    InProgress,
    HandoffRequested,
    LeaseExpired,
    Completed,
    Failed,
    Cancelled,
}

/// Schema-versioned bounded delegation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRequestV1 {
    pub schema_version: String,
    pub request_ref: String,
    pub work_ref: String,
    pub requester_ref: String,
    pub candidate_pool_ref: String,
    pub state: DelegationLifecycleState,
    pub redaction_profile: String,
}

/// Atomic claim record. A request can have at most one active claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationClaimV1 {
    pub claim_ref: String,
    pub request_ref: String,
    pub assignee_ref: String,
    pub capacity_snapshot_ref: String,
    pub accepted_epoch_ms: u64,
}

/// Replayable lease state with deterministic time inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationLeaseV1 {
    pub lease_ref: String,
    pub claim_ref: String,
    pub owner_ref: String,
    pub issued_at_epoch_ms: u64,
    pub expires_at_epoch_ms: u64,
    pub renewable: bool,
    pub revoked: bool,
}

impl DelegationLeaseV1 {
    pub fn is_active_at(&self, now_epoch_ms: u64) -> bool {
        !self.revoked && now_epoch_ms < self.expires_at_epoch_ms
    }

    pub fn renew(&mut self, owner_ref: &str, now_epoch_ms: u64, new_expiry: u64) -> bool {
        if self.owner_ref == owner_ref
            && self.renewable
            && self.is_active_at(now_epoch_ms)
            && new_expiry > self.expires_at_epoch_ms
        {
            self.expires_at_epoch_ms = new_expiry;
            true
        } else {
            false
        }
    }
}

/// Handoff preserves the last opaque checkpoint reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationHandoffV1 {
    pub handoff_ref: String,
    pub request_ref: String,
    pub from_owner_ref: String,
    pub to_candidate_ref: String,
    pub checkpoint_ref: Option<String>,
}

/// Provider-neutral capacity fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacitySnapshotV1 {
    pub capacity_ref: String,
    pub subject_ref: String,
    pub available_units: u32,
    pub reserved_units: u32,
    pub evidence_ref: String,
}

impl CapacitySnapshotV1 {
    pub fn can_accept(&self, required_units: u32) -> bool {
        self.available_units.saturating_sub(self.reserved_units) >= required_units
    }
}

/// Bounded result projection for partial and terminal outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationResultOutcome {
    Success,
    Partial,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationResultV1 {
    pub result_ref: String,
    pub request_ref: String,
    pub outcome: DelegationResultOutcome,
    pub artifact_refs: Vec<String>,
    pub terminal: bool,
}

/// State Specification for legal delegation transitions and terminal races.
pub struct DelegationLifecycleSpec;

impl DelegationLifecycleSpec {
    pub fn allows(from: DelegationLifecycleState, to: DelegationLifecycleState) -> bool {
        matches!(
            (from, to),
            (
                DelegationLifecycleState::Requested,
                DelegationLifecycleState::Queued
            ) | (
                DelegationLifecycleState::Queued,
                DelegationLifecycleState::Claimed
            ) | (
                DelegationLifecycleState::Claimed,
                DelegationLifecycleState::InProgress
            ) | (
                DelegationLifecycleState::InProgress,
                DelegationLifecycleState::HandoffRequested
            ) | (
                DelegationLifecycleState::HandoffRequested,
                DelegationLifecycleState::Claimed
            ) | (
                DelegationLifecycleState::Claimed,
                DelegationLifecycleState::LeaseExpired
            ) | (
                DelegationLifecycleState::InProgress,
                DelegationLifecycleState::Completed
            ) | (
                DelegationLifecycleState::InProgress,
                DelegationLifecycleState::Failed
            ) | (
                DelegationLifecycleState::InProgress,
                DelegationLifecycleState::Cancelled
            ) | (
                DelegationLifecycleState::Queued,
                DelegationLifecycleState::Cancelled
            )
        )
    }

    pub fn terminal_race_winner(
        current: DelegationLifecycleState,
        proposed: DelegationLifecycleState,
    ) -> Option<DelegationLifecycleState> {
        if matches!(
            current,
            DelegationLifecycleState::Queued
                | DelegationLifecycleState::Claimed
                | DelegationLifecycleState::InProgress
        ) && matches!(
            proposed,
            DelegationLifecycleState::Completed
                | DelegationLifecycleState::Failed
                | DelegationLifecycleState::Cancelled
                | DelegationLifecycleState::LeaseExpired
        ) {
            Some(proposed)
        } else {
            None
        }
    }
}

/// Atomic claim Specification: only one active owner may claim a request.
pub struct DelegationClaimSpec;

impl DelegationClaimSpec {
    pub fn can_claim(existing: Option<&DelegationClaimV1>, candidate: &str) -> bool {
        !candidate.trim().is_empty() && existing.is_none()
    }
}

/// Handoff Specification for eligible candidates and checkpoint continuity.
pub struct DelegationHandoffSpec;

impl DelegationHandoffSpec {
    pub fn accepts(candidate: &str, eligible: bool, checkpoint_ref: Option<&str>) -> bool {
        !candidate.trim().is_empty()
            && eligible
            && checkpoint_ref.is_none_or(|value| !value.trim().is_empty() && value.len() <= 256)
    }
}
