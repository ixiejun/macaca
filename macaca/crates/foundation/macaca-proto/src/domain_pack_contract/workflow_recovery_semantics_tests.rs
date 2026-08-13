use super::workflow_recovery_semantics::*;

#[test]
fn recovery_state_machine_and_failure_classification_are_provider_neutral() {
    assert!(RecoveryLifecycleSpec::allows(
        RecoveryLifecycleState::Failed,
        RecoveryLifecycleState::Classified
    ));
    assert!(RecoveryLifecycleSpec::allows(
        RecoveryLifecycleState::Repairing,
        RecoveryLifecycleState::Compensating
    ));
    assert!(!RecoveryLifecycleSpec::allows(
        RecoveryLifecycleState::Terminal,
        RecoveryLifecycleState::Retrying
    ));
    assert_eq!(
        FailureClassificationSpec::classify("timeout"),
        FailureClass::Transient
    );
    assert_eq!(
        FailureClassificationSpec::classify("checkpoint_corrupt"),
        FailureClass::CorruptedCheckpoint
    );
    assert_eq!(
        FailureClassificationSpec::classify("unknown-code"),
        FailureClass::Unknown
    );
}

#[test]
fn checkpoint_retry_resume_and_replay_are_deterministic() {
    let point = RecoveryPointV1 {
        point_ref: "point:one".into(),
        owner_service_ref: "service:one".into(),
        checkpoint_ref: "checkpoint:one".into(),
        integrity_hash: "hash:one".into(),
        compatibility_version: "v1".into(),
        replay_cursor: "cursor:one".into(),
    };
    assert!(RecoveryPointSpec::is_compatible(&point, "hash:one", "v1"));
    assert!(!RecoveryPointSpec::is_compatible(&point, "hash:two", "v1"));
    let policy = RetryPolicyV1 {
        policy_ref: "policy:one".into(),
        max_attempts: 2,
        backoff_ms: 10,
        terminal_on_exhaustion: true,
    };
    assert!(policy.allows_attempt(1));
    assert!(!policy.allows_attempt(2));
    assert_eq!(policy.backoff_for(1), 20);
    let resume = ResumePlanV1 {
        resume_ref: "resume:one".into(),
        recovery_point_ref: "point:one".into(),
        target_service_ref: "service:one".into(),
        replay_cursor: "cursor:one".into(),
        compatibility_checked: true,
    };
    assert!(resume.can_resume());
    let export = ReplayExportV1 {
        export_ref: "export:one".into(),
        trace_ref: "trace:one".into(),
        redacted_bundle_ref: "bundle:redacted".into(),
        event_count: 2,
        payloads_redacted: true,
    };
    assert!(export.is_safe());
}
