use super::workflow_delegation_semantics::*;

#[test]
fn delegation_state_machine_covers_handoff_and_terminal_races() {
    assert!(DelegationLifecycleSpec::allows(
        DelegationLifecycleState::Requested,
        DelegationLifecycleState::Queued
    ));
    assert!(DelegationLifecycleSpec::allows(
        DelegationLifecycleState::InProgress,
        DelegationLifecycleState::HandoffRequested
    ));
    assert!(!DelegationLifecycleSpec::allows(
        DelegationLifecycleState::Completed,
        DelegationLifecycleState::Cancelled
    ));
    assert!(DelegationLifecycleSpec::terminal_race_winner(
        DelegationLifecycleState::InProgress,
        DelegationLifecycleState::Cancelled
    )
    .is_some());
    assert!(DelegationLifecycleSpec::terminal_race_winner(
        DelegationLifecycleState::Completed,
        DelegationLifecycleState::Cancelled
    )
    .is_none());
}

#[test]
fn atomic_claim_and_capacity_are_provider_neutral() {
    let claim = DelegationClaimV1 {
        claim_ref: "claim:one".into(),
        request_ref: "request:one".into(),
        assignee_ref: "assignee:one".into(),
        capacity_snapshot_ref: "capacity:one".into(),
        accepted_epoch_ms: 10,
    };
    assert!(!DelegationClaimSpec::can_claim(
        Some(&claim),
        "assignee:two"
    ));
    assert!(DelegationClaimSpec::can_claim(None, "assignee:two"));
    let capacity = CapacitySnapshotV1 {
        capacity_ref: "capacity:one".into(),
        subject_ref: "subject:one".into(),
        available_units: 5,
        reserved_units: 2,
        evidence_ref: "evidence:one".into(),
    };
    assert!(capacity.can_accept(3));
    assert!(!capacity.can_accept(4));
}

#[test]
fn lease_renewal_expiry_handoff_and_bounded_result_are_deterministic() {
    let mut lease = DelegationLeaseV1 {
        lease_ref: "lease:one".into(),
        claim_ref: "claim:one".into(),
        owner_ref: "owner:one".into(),
        issued_at_epoch_ms: 10,
        expires_at_epoch_ms: 20,
        renewable: true,
        revoked: false,
    };
    assert!(lease.is_active_at(19));
    assert!(lease.renew("owner:one", 19, 30));
    assert!(!lease.renew("owner:two", 20, 40));
    assert!(DelegationHandoffSpec::accepts(
        "candidate:two",
        true,
        Some("checkpoint:one")
    ));
    assert!(!DelegationHandoffSpec::accepts(
        "candidate:two",
        false,
        Some("checkpoint:one")
    ));
    let result = DelegationResultV1 {
        result_ref: "result:one".into(),
        request_ref: "request:one".into(),
        outcome: DelegationResultOutcome::Partial,
        artifact_refs: vec!["artifact:summary".into()],
        terminal: false,
    };
    assert!(!result.terminal);
}
