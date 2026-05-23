//! Lifecycle-state contract tests for governed skills.
//!
//! The Skill service exposes lifecycle states as provider-neutral DTOs.  This
//! test keeps the wire contract explicit so SDK, shell, Context, Task, and
//! future Store/EventLog providers do not invent lifecycle labels locally.

use crate::{SkillLifecycleState, SkillLifecycleStateMachine};

#[test]
fn skill_lifecycle_state_contract_includes_complete_governance_set() {
    let states = [
        SkillLifecycleState::Draft,
        SkillLifecycleState::Active,
        SkillLifecycleState::Stale,
        SkillLifecycleState::Archived,
        SkillLifecycleState::Quarantined,
        SkillLifecycleState::Superseded,
        SkillLifecycleState::Rejected,
    ];
    let serialized = states
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .expect("lifecycle states must serialize for service DTOs");

    assert_eq!(serialized.len(), 7);
    assert!(serialized.contains(&"\"Draft\"".to_string()));
    assert!(serialized.contains(&"\"Superseded\"".to_string()));
    assert!(serialized.contains(&"\"Rejected\"".to_string()));
}

#[test]
fn lifecycle_state_machine_allows_only_governed_transitions() {
    assert!(SkillLifecycleStateMachine::validate_transition(
        &SkillLifecycleState::Draft,
        &SkillLifecycleState::Active
    )
    .is_ok());
    assert!(SkillLifecycleStateMachine::validate_transition(
        &SkillLifecycleState::Active,
        &SkillLifecycleState::Superseded
    )
    .is_ok());
    assert!(SkillLifecycleStateMachine::validate_transition(
        &SkillLifecycleState::Archived,
        &SkillLifecycleState::Active
    )
    .is_ok());
    assert!(SkillLifecycleStateMachine::validate_transition(
        &SkillLifecycleState::Rejected,
        &SkillLifecycleState::Active
    )
    .is_err());
    assert!(SkillLifecycleStateMachine::validate_transition(
        &SkillLifecycleState::Draft,
        &SkillLifecycleState::Archived
    )
    .is_err());
}
