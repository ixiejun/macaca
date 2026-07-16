use std::collections::BTreeMap;

use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackResourceReservation,
};
use super::super::workflow_task::{TaskLease, WorkflowTaskResultStatus, WorkflowTaskState};
use super::super::workflow_task_dispatch_gate::WorkflowTaskDispatchGate;
use super::super::workflow_task_transition::WorkflowTaskTransitionRequest;

#[test]
fn workflow_task_gate_never_dispatches_rejected_provider_commands() {
    let gate = WorkflowTaskDispatchGate::new(["workflow_task.complete"]);
    let allowed_preflight = valid_preflight();
    let allowed_transition = valid_transition();
    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_after_validation(&allowed_preflight, &allowed_transition, || {
            dispatched = true;
            "provider-result"
        }),
        Ok("provider-result")
    );
    assert!(dispatched);

    for (preflight, transition, expected) in rejected_cases(&allowed_preflight, &allowed_transition)
    {
        let mut rejected_dispatched = false;
        let rejection = gate
            .dispatch_after_validation(&preflight, &transition, || rejected_dispatched = true)
            .expect_err("rejected task commands must not reach the provider");
        assert_eq!(rejection.status, expected);
        assert!(!rejected_dispatched);
    }
}

fn rejected_cases(
    preflight: &DomainPackCommandPreflight,
    transition: &WorkflowTaskTransitionRequest,
) -> Vec<(
    DomainPackCommandPreflight,
    WorkflowTaskTransitionRequest,
    WorkflowTaskResultStatus,
)> {
    vec![
        (
            DomainPackCommandPreflight {
                policy: DomainPackPolicyEvidence {
                    allowed: false,
                    ..preflight.policy.clone()
                },
                ..preflight.clone()
            },
            transition.clone(),
            WorkflowTaskResultStatus::Denied,
        ),
        (
            DomainPackCommandPreflight {
                entitlement: DomainPackEntitlementEvidence {
                    provider_available: false,
                    ..preflight.entitlement.clone()
                },
                ..preflight.clone()
            },
            transition.clone(),
            WorkflowTaskResultStatus::Unavailable,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                from: WorkflowTaskState::Completed,
                to: WorkflowTaskState::Running,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::InvalidState,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                dependencies_satisfied: false,
                from: WorkflowTaskState::Draft,
                to: WorkflowTaskState::Queued,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::DependencyBlocked,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                lease: None,
                from: WorkflowTaskState::Claimed,
                to: WorkflowTaskState::Running,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::LeaseExpired,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                lease: Some(TaskLease {
                    revoked: true,
                    ..active_lease()
                }),
                from: WorkflowTaskState::Claimed,
                to: WorkflowTaskState::Running,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::LeaseRevoked,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                attempt_index: 3,
                max_attempts: 3,
                to: WorkflowTaskState::Failed,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::RetryExhausted,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                active_concurrency: 2,
                max_concurrency: 2,
                from: WorkflowTaskState::Queued,
                to: WorkflowTaskState::Claimed,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::ConcurrencyBlocked,
        ),
        (
            preflight.clone(),
            WorkflowTaskTransitionRequest {
                artifact_reference_valid: false,
                ..transition.clone()
            },
            WorkflowTaskResultStatus::ArtifactBlocked,
        ),
        (
            DomainPackCommandPreflight {
                reserved_resources: DomainPackResourceReservation::default(),
                ..preflight.clone()
            },
            transition.clone(),
            WorkflowTaskResultStatus::QuotaExceeded,
        ),
    ]
}

fn valid_preflight() -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: "workflow_task.complete".into(),
        requested_scope: "workflow.task.complete".into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:granted".into(),
            allowed: true,
            reason_code: "granted".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:granted".into(),
            approved: true,
            reason_code: "granted".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:granted".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "granted".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("task_mutation".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("task_mutation".into(), 1)]),
        },
    }
}

fn valid_transition() -> WorkflowTaskTransitionRequest {
    WorkflowTaskTransitionRequest {
        from: WorkflowTaskState::Running,
        to: WorkflowTaskState::Completed,
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
