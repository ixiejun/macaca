//! Lifecycle-state contract tests for governed skills.
//!
//! The Skill service exposes lifecycle states as provider-neutral DTOs.  This
//! test keeps the wire contract explicit so SDK, shell, Context, Task, and
//! future Store/EventLog providers do not invent lifecycle labels locally.

use macaca_proto::TraceContext;

use crate::{
    SkillAliasKind, SkillAliasRecord, SkillAuthorKind, SkillCurationLifecycleCommand,
    SkillCurationSupersedeCommand, SkillLifecycleState, SkillLifecycleStateMachine,
    SkillPinnedMutationGuard, SkillPinnedMutationOperation, SkillServicePolicyHints,
    SkillServiceScope,
};

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
    let allowed = [
        (SkillLifecycleState::Draft, SkillLifecycleState::Active),
        (SkillLifecycleState::Draft, SkillLifecycleState::Rejected),
        (SkillLifecycleState::Active, SkillLifecycleState::Stale),
        (SkillLifecycleState::Active, SkillLifecycleState::Archived),
        (
            SkillLifecycleState::Active,
            SkillLifecycleState::Quarantined,
        ),
        (SkillLifecycleState::Active, SkillLifecycleState::Superseded),
        (SkillLifecycleState::Stale, SkillLifecycleState::Active),
        (SkillLifecycleState::Stale, SkillLifecycleState::Archived),
        (SkillLifecycleState::Stale, SkillLifecycleState::Quarantined),
        (SkillLifecycleState::Stale, SkillLifecycleState::Superseded),
        (SkillLifecycleState::Archived, SkillLifecycleState::Active),
        (
            SkillLifecycleState::Quarantined,
            SkillLifecycleState::Active,
        ),
        (
            SkillLifecycleState::Quarantined,
            SkillLifecycleState::Archived,
        ),
        (
            SkillLifecycleState::Quarantined,
            SkillLifecycleState::Rejected,
        ),
    ];
    for (from, to) in allowed {
        assert!(
            SkillLifecycleStateMachine::validate_transition(&from, &to).is_ok(),
            "{from:?} -> {to:?} should be allowed"
        );
    }
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

#[test]
fn supersede_command_requires_alias_redirect_evidence() {
    let lifecycle = SkillCurationLifecycleCommand {
        trace: TraceContext::new("trace-supersede-command-contract"),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/source".into(),
        name: "source".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        author_kind: SkillAuthorKind::Agent,
        reason: "validate alias-before-supersede contract".into(),
        evidence_ids: vec!["evidence://task/supersede".into()],
        task_id: Some("task-supersede".into()),
        policy_decision_refs: vec!["policy://decision/supersede".into()],
        policy: SkillServicePolicyHints::default(),
    };
    let now = chrono::Utc::now();
    let mut command = SkillCurationSupersedeCommand {
        lifecycle,
        alias: SkillAliasRecord {
            source_skill_id: "skill://agent/source".into(),
            source_name: "source".into(),
            target_skill_id: "skill://agent/target".into(),
            target_name: "target".into(),
            kind: SkillAliasKind::SupersededBy,
            rationale: "source is redirected to target".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/supersede".into()],
        },
    };

    assert!(command.validate().is_ok());
    command.alias.evidence_ids.clear();
    assert!(command
        .validate()
        .expect_err("alias evidence is required")
        .contains("alias evidence"));
}

#[test]
fn lifecycle_command_requires_evidence_and_policy_decision_refs() {
    let mut command = SkillCurationLifecycleCommand {
        trace: TraceContext::new("trace-lifecycle-command-contract"),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/governed".into(),
        name: "governed".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        author_kind: SkillAuthorKind::Agent,
        reason: "validate lifecycle mutation audit contract".into(),
        evidence_ids: vec!["evidence://task/1".into()],
        task_id: Some("task-lifecycle-contract".into()),
        policy_decision_refs: vec!["policy://decision/1".into()],
        policy: SkillServicePolicyHints::default(),
    };

    assert!(command.validate().is_ok());
    command.policy_decision_refs.clear();
    assert!(command
        .validate()
        .expect_err("policy refs must be mandatory")
        .contains("policy decision references"));
    command.policy_decision_refs = vec!["policy://decision/1".into()];
    command.evidence_ids.clear();
    assert!(command
        .validate()
        .expect_err("evidence refs must be mandatory")
        .contains("evidence references"));
}

#[test]
fn pinned_mutation_guard_denies_all_destructive_operations_without_override() {
    let destructive_operations = [
        SkillPinnedMutationOperation::Archive,
        SkillPinnedMutationOperation::Supersede,
        SkillPinnedMutationOperation::MergeApply,
        SkillPinnedMutationOperation::Delete,
    ];

    for operation in destructive_operations {
        assert!(
            SkillPinnedMutationGuard::ensure_not_pinned(false, &operation).is_ok(),
            "unpinned skills should allow {operation:?}"
        );
        let err = SkillPinnedMutationGuard::ensure_not_pinned(true, &operation)
            .expect_err("pinned destructive operations require a future override contract");
        assert!(err.contains("pinned skill"));
        assert!(err.contains("approval override"));
    }
}
