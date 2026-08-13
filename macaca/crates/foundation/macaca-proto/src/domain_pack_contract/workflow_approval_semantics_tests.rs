use super::workflow_approval_semantics::*;

#[test]
fn lifecycle_spec_covers_terminal_races_and_escalation() {
    assert!(ApprovalLifecycleSpec::allows(
        ApprovalLifecycleState::Requested,
        ApprovalLifecycleState::Pending
    ));
    assert!(ApprovalLifecycleSpec::allows(
        ApprovalLifecycleState::Pending,
        ApprovalLifecycleState::Escalated
    ));
    assert!(ApprovalLifecycleSpec::allows(
        ApprovalLifecycleState::Claimed,
        ApprovalLifecycleState::Cancelled
    ));
    assert!(!ApprovalLifecycleSpec::allows(
        ApprovalLifecycleState::Cancelled,
        ApprovalLifecycleState::Decided
    ));
    assert!(!ApprovalLifecycleSpec::allows(
        ApprovalLifecycleState::Consumed,
        ApprovalLifecycleState::Cancelled
    ));
}

#[test]
fn decision_gate_rechecks_lineage_expiry_and_consumes_once() {
    let decision = ApprovalDecisionV1 {
        decision_ref: "decision:one".into(),
        request_ref: "request:one".into(),
        approver_ref: "principal:approver".into(),
        outcome: "approved".into(),
        policy_hash: "policy:one".into(),
        source_trace_ref: "trace:one".into(),
        expires_at_epoch_ms: Some(100),
        consumed: false,
    };
    let mut gate = ApprovalDecisionGateV1 {
        gate_ref: "gate:one".into(),
        request_ref: "request:one".into(),
        subject_ref: "subject:one".into(),
        required_outcome: "approved".into(),
        policy_hash: "policy:one".into(),
        decision: Some(decision),
        valid_until_epoch_ms: Some(100),
        consumption_mode: ApprovalConsumptionMode::SingleUse,
    };
    assert!(gate.consume("subject:one", "policy:one", 100));
    assert!(!gate.consume("subject:one", "policy:one", 100));
    assert!(!gate.evaluate("subject:other", "policy:one", 1));
    assert!(!ApprovalEligibilitySpec::accepts(
        &ApprovalEligibilityEvidence {
            principal_ref: "principal:approver".into(),
            identity_active: true,
            membership_active: false,
            delegation_active: true,
            tenant_matches: true,
        }
    ));
}

#[test]
fn eligibility_and_idempotency_are_provider_neutral() {
    assert!(ApprovalEligibilitySpec::accepts(
        &ApprovalEligibilityEvidence {
            principal_ref: "principal:approver".into(),
            identity_active: true,
            membership_active: true,
            delegation_active: true,
            tenant_matches: true,
        }
    ));
    assert_eq!(
        check_idempotency(Some("idem:one"), Some("hash:one"), "idem:one", "hash:one"),
        ApprovalIdempotencyResult::Replay(())
    );
    assert_eq!(
        check_idempotency(Some("idem:one"), Some("hash:one"), "idem:one", "hash:two"),
        ApprovalIdempotencyResult::Conflict
    );
    assert_eq!(
        check_idempotency(None, None, "idem:one", "hash:one"),
        ApprovalIdempotencyResult::New(())
    );
}

#[test]
fn deadline_escalation_assignment_and_terminal_race_are_deterministic() {
    assert!(!ApprovalDeadlineSpec::is_expired(Some(100), 99));
    assert!(ApprovalDeadlineSpec::is_expired(Some(100), 100));
    let assignment = ApprovalAssignmentV1 {
        assignment_ref: "assignment:one".into(),
        request_ref: "request:one".into(),
        eligible_principal_refs: ["principal:one".into()].into_iter().collect(),
        claimed_by_ref: None,
        escalated_from_ref: None,
    };
    let escalated = assignment.escalate(
        "assignment:two",
        ["principal:two".into()].into_iter().collect(),
    );
    assert_eq!(
        escalated.escalated_from_ref.as_deref(),
        Some("assignment:one")
    );
    assert!(ApprovalLifecycleSpec::terminal_race_winner(
        ApprovalLifecycleState::Pending,
        ApprovalLifecycleState::Cancelled
    )
    .is_some());
    assert!(ApprovalLifecycleSpec::terminal_race_winner(
        ApprovalLifecycleState::Cancelled,
        ApprovalLifecycleState::Decided
    )
    .is_none());
}

#[test]
fn filtered_pending_page_hides_unauthorized_queue_shape() {
    let items = (0..3)
        .map(|index| ApprovalPendingProjection {
            approval_ref: format!("approval:{index}"),
            state: ApprovalLifecycleState::Pending,
            replay_ref: format!("replay:{index}"),
        })
        .collect();
    let (page, next) = filtered_pending_page(items, None, 2);
    assert_eq!(page.len(), 2);
    assert_eq!(next.as_deref(), Some("cursor:2"));
}
