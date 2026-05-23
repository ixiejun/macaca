//! State-machine validation for governed Skill lifecycle metadata.
//!
//! The validator is a small State-pattern guard owned by the Skill service
//! contract.  Providers call it before mutating lifecycle read models, while
//! shells and SDK clients continue to submit typed commands without embedding
//! lifecycle policy locally.

use crate::SkillLifecycleState;

/// Service-owned lifecycle transition validator.
pub struct SkillLifecycleStateMachine;

impl SkillLifecycleStateMachine {
    /// Validate one lifecycle transition before a provider mutates metadata.
    ///
    /// Self-transitions are accepted because pin, unpin, telemetry, and
    /// provenance updates can keep the same lifecycle while still appending
    /// audit evidence.  Destructive transitions stay explicit so later policy
    /// decorators can require approvals, aliases, or rollback mementos.
    pub fn validate_transition(
        from: &SkillLifecycleState,
        to: &SkillLifecycleState,
    ) -> Result<(), String> {
        if from == to || allowed_transition(from, to) {
            return Ok(());
        }
        Err(format!(
            "skill lifecycle transition from {from:?} to {to:?} is not allowed"
        ))
    }
}

fn allowed_transition(from: &SkillLifecycleState, to: &SkillLifecycleState) -> bool {
    matches!(
        (from, to),
        (SkillLifecycleState::Draft, SkillLifecycleState::Active)
            | (SkillLifecycleState::Draft, SkillLifecycleState::Rejected)
            | (SkillLifecycleState::Active, SkillLifecycleState::Stale)
            | (SkillLifecycleState::Active, SkillLifecycleState::Archived)
            | (
                SkillLifecycleState::Active,
                SkillLifecycleState::Quarantined
            )
            | (SkillLifecycleState::Active, SkillLifecycleState::Superseded)
            | (SkillLifecycleState::Stale, SkillLifecycleState::Active)
            | (SkillLifecycleState::Stale, SkillLifecycleState::Archived)
            | (SkillLifecycleState::Stale, SkillLifecycleState::Quarantined)
            | (SkillLifecycleState::Stale, SkillLifecycleState::Superseded)
            | (SkillLifecycleState::Archived, SkillLifecycleState::Active)
            | (
                SkillLifecycleState::Quarantined,
                SkillLifecycleState::Active
            )
            | (
                SkillLifecycleState::Quarantined,
                SkillLifecycleState::Archived
            )
            | (
                SkillLifecycleState::Quarantined,
                SkillLifecycleState::Rejected
            )
    )
}
