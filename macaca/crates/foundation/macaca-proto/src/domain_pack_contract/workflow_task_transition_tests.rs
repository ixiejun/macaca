use super::super::workflow_task::{TaskLease, WorkflowTaskResultStatus, WorkflowTaskState};
use super::super::workflow_task_transition::{
    WorkflowTaskTransitionRequest, WorkflowTaskTransitionSpec,
};

#[test]
fn workflow_task_transition_spec_accepts_valid_lease_backed_completion() {
    let request = valid_request(WorkflowTaskState::Running, WorkflowTaskState::Completed);
    let decision = WorkflowTaskTransitionSpec.evaluate(&request);
    assert!(decision.allowed);
    assert_eq!(decision.status, WorkflowTaskResultStatus::Success);
}

#[test]
fn workflow_task_transition_spec_rejects_each_pre_dispatch_failure_state() {
    let cases = [
        (
            WorkflowTaskTransitionRequest {
                idempotency_key: None,
                expected_version: None,
                ..valid_request(WorkflowTaskState::Draft, WorkflowTaskState::Queued)
            },
            WorkflowTaskResultStatus::VersionMismatch,
        ),
        (
            WorkflowTaskTransitionRequest {
                dependencies_satisfied: false,
                ..valid_request(WorkflowTaskState::Draft, WorkflowTaskState::Queued)
            },
            WorkflowTaskResultStatus::DependencyBlocked,
        ),
        (
            WorkflowTaskTransitionRequest {
                active_concurrency: 2,
                max_concurrency: 2,
                ..valid_request(WorkflowTaskState::Queued, WorkflowTaskState::Claimed)
            },
            WorkflowTaskResultStatus::ConcurrencyBlocked,
        ),
        (
            WorkflowTaskTransitionRequest {
                lease: None,
                ..valid_request(WorkflowTaskState::Claimed, WorkflowTaskState::Running)
            },
            WorkflowTaskResultStatus::LeaseExpired,
        ),
        (
            WorkflowTaskTransitionRequest {
                now_epoch_ms: 20,
                ..valid_request(WorkflowTaskState::Claimed, WorkflowTaskState::Running)
            },
            WorkflowTaskResultStatus::LeaseExpired,
        ),
        (
            WorkflowTaskTransitionRequest {
                lease: Some(TaskLease {
                    revoked: true,
                    ..active_lease()
                }),
                ..valid_request(WorkflowTaskState::Claimed, WorkflowTaskState::Running)
            },
            WorkflowTaskResultStatus::LeaseRevoked,
        ),
        (
            WorkflowTaskTransitionRequest {
                attempt_index: 3,
                max_attempts: 3,
                ..valid_request(WorkflowTaskState::Running, WorkflowTaskState::Failed)
            },
            WorkflowTaskResultStatus::RetryExhausted,
        ),
        (
            WorkflowTaskTransitionRequest {
                timeout_within_limit: false,
                ..valid_request(WorkflowTaskState::Running, WorkflowTaskState::Completed)
            },
            WorkflowTaskResultStatus::QuotaExceeded,
        ),
        (
            WorkflowTaskTransitionRequest {
                artifact_reference_valid: false,
                ..valid_request(WorkflowTaskState::Running, WorkflowTaskState::Completed)
            },
            WorkflowTaskResultStatus::ArtifactBlocked,
        ),
        (
            WorkflowTaskTransitionRequest {
                redaction_policy_valid: false,
                ..valid_request(WorkflowTaskState::Running, WorkflowTaskState::Completed)
            },
            WorkflowTaskResultStatus::Denied,
        ),
        (
            WorkflowTaskTransitionRequest {
                terminal_transition_approved: false,
                ..valid_request(WorkflowTaskState::Running, WorkflowTaskState::Cancelled)
            },
            WorkflowTaskResultStatus::Denied,
        ),
        (
            valid_request(WorkflowTaskState::Completed, WorkflowTaskState::Running),
            WorkflowTaskResultStatus::InvalidState,
        ),
    ];

    for (request, expected) in cases {
        let decision = WorkflowTaskTransitionSpec.evaluate(&request);
        assert!(!decision.allowed);
        assert_eq!(decision.status, expected);
    }
}

fn valid_request(from: WorkflowTaskState, to: WorkflowTaskState) -> WorkflowTaskTransitionRequest {
    WorkflowTaskTransitionRequest {
        from,
        to,
        idempotency_key: Some("idempotency:task".into()),
        expected_version: None,
        dependencies_satisfied: true,
        queue_available: true,
        active_concurrency: 0,
        max_concurrency: 2,
        lease: Some(active_lease()),
        now_epoch_ms: 10,
        attempt_index: 1,
        max_attempts: 3,
        timeout_within_limit: true,
        artifact_reference_valid: true,
        redaction_policy_valid: true,
        terminal_transition_approved: true,
    }
}

fn active_lease() -> TaskLease {
    TaskLease {
        lease_ref: "lease:one".into(),
        task_ref: "task:one".into(),
        owner_ref: "owner:one".into(),
        expires_at_epoch_ms: 20,
        heartbeat_deadline_epoch_ms: 15,
        revoked: false,
    }
}
