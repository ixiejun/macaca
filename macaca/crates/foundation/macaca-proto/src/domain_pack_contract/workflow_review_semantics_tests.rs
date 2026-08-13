use super::workflow_review_semantics::*;

fn finding(reference: &str, blocking: bool) -> ReviewFindingV1 {
    ReviewFindingV1 {
        finding_ref: reference.into(),
        request_ref: "request:one".into(),
        round_ref: "round:one".into(),
        severity: "bounded".into(),
        blocking,
        state: ReviewFindingState::Open,
        evidence_ref: "evidence:one".into(),
        prior_finding_ref: None,
        redaction_profile: "references_only".into(),
    }
}

#[test]
fn review_lifecycle_and_finding_state_machines_are_provider_neutral() {
    assert!(ReviewLifecycleSpec::allows(
        ReviewLifecycleState::Requested,
        ReviewLifecycleState::InReview
    ));
    assert!(ReviewLifecycleSpec::allows(
        ReviewLifecycleState::FixSubmitted,
        ReviewLifecycleState::InReview
    ));
    assert!(ReviewLifecycleSpec::allows(
        ReviewLifecycleState::Approved,
        ReviewLifecycleState::Stale
    ));
    assert!(!ReviewLifecycleSpec::allows(
        ReviewLifecycleState::Closed,
        ReviewLifecycleState::InReview
    ));
    assert!(ReviewFindingLifecycleSpec::allows(
        ReviewFindingState::Fixed,
        ReviewFindingState::Verified
    ));
    assert!(!ReviewFindingLifecycleSpec::allows(
        ReviewFindingState::Verified,
        ReviewFindingState::Open
    ));
}

#[test]
fn revision_policy_and_closure_gate_reject_stale_or_blocked_reviews() {
    assert!(!ReviewRevisionSpec::outcome_is_current(
        Some("revision:one"),
        "revision:two",
        ReviewRevisionPolicy::InvalidateOnChange,
        None
    ));
    assert!(ReviewRevisionSpec::outcome_is_current(
        Some("revision:one"),
        "revision:two",
        ReviewRevisionPolicy::CarryForwardWithEvidence,
        Some("policy:carry-forward")
    ));
    let mut gate = ReviewClosureGateV1 {
        gate_ref: "gate:one".into(),
        request_ref: "request:one".into(),
        subject_revision_hash: "revision:one".into(),
        outcome_ref: Some("outcome:one".into()),
        unresolved_blocking_finding_refs: vec!["finding:blocking".into()],
        stale_revision: false,
        replay_ref: "replay:one".into(),
    };
    assert!(!ReviewClosureGateSpec::can_close(&gate));
    gate.unresolved_blocking_finding_refs.clear();
    assert!(ReviewClosureGateSpec::can_close(&gate));
    gate.stale_revision = true;
    assert!(!ReviewClosureGateSpec::can_close(&gate));
}

#[test]
fn rereview_preserves_prior_finding_history_and_dismissal_requires_evidence() {
    let original = finding("finding:one", true);
    let rereviewed = ReviewFindingV1 {
        finding_ref: "finding:two".into(),
        state: ReviewFindingState::Verified,
        prior_finding_ref: Some(original.finding_ref.clone()),
        ..original.clone()
    };
    assert_eq!(original.state, ReviewFindingState::Open);
    assert_eq!(rereviewed.prior_finding_ref.as_deref(), Some("finding:one"));
    assert!(ReviewDismissalSpec::is_authorized(
        Some("principal:authorized"),
        Some("reason:bounded"),
        true
    ));
    assert!(!ReviewDismissalSpec::is_authorized(
        Some("principal:authorized"),
        None,
        true
    ));
}

#[test]
fn filtered_listing_and_concurrent_gate_resolution_are_deterministic() {
    let visible = vec![
        finding("finding:visible-one", false),
        finding("finding:visible-two", false),
    ];
    let (page, cursor) = filtered_finding_page(&visible, None, 1);
    assert_eq!(page.len(), 1);
    assert_eq!(cursor.as_deref(), Some("cursor:1"));
    assert_eq!(
        resolve_gate_race(5, 5),
        ReviewGateRaceResult::BlockedByFinding
    );
    assert_eq!(resolve_gate_race(5, 6), ReviewGateRaceResult::Approved);
}
