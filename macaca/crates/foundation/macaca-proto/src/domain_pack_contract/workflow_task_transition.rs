use serde::{Deserialize, Serialize};

use super::workflow_task::{TaskLease, WorkflowTaskResultStatus, WorkflowTaskState};

/// Reference-only facts used to validate one workflow task transition.
///
/// Provider implementations retain their own task data. This value object only
/// captures bounded policy outcomes and deterministic timing inputs needed to
/// decide whether a transition may reach a provider dispatch boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskTransitionRequest {
    pub from: WorkflowTaskState,
    pub to: WorkflowTaskState,
    pub idempotency_key: Option<String>,
    pub expected_version: Option<String>,
    pub dependencies_satisfied: bool,
    pub queue_available: bool,
    pub active_concurrency: u32,
    pub max_concurrency: u32,
    pub lease: Option<TaskLease>,
    pub now_epoch_ms: u64,
    pub attempt_index: u32,
    pub max_attempts: u32,
    pub timeout_within_limit: bool,
    pub artifact_reference_valid: bool,
    pub redaction_policy_valid: bool,
    pub terminal_transition_approved: bool,
}

/// Specification result that maps directly to typed workflow task outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskTransitionDecision {
    pub status: WorkflowTaskResultStatus,
    pub allowed: bool,
}

/// State-machine Specification shared by durable, remote, plugin, mock, and unavailable providers.
///
/// It deliberately has no persistence, clock, queue, worker, application, or
/// provider dependency. Runtime providers call it before side effects and emit
/// their own trace/audit events around the resulting bounded status.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkflowTaskTransitionSpec;

impl WorkflowTaskTransitionSpec {
    /// Validate one state transition without mutating a provider-owned task record.
    pub fn evaluate(
        &self,
        request: &WorkflowTaskTransitionRequest,
    ) -> WorkflowTaskTransitionDecision {
        if !is_declared_transition(request.from, request.to) {
            return denied(WorkflowTaskResultStatus::InvalidState);
        }
        if is_mutating(request.to)
            && !has_bounded_mutation_identity(
                request.idempotency_key.as_deref(),
                request.expected_version.as_deref(),
            )
        {
            return denied(WorkflowTaskResultStatus::VersionMismatch);
        }
        if request.to == WorkflowTaskState::Queued
            && (!request.dependencies_satisfied || !request.queue_available)
        {
            return denied(WorkflowTaskResultStatus::DependencyBlocked);
        }
        if matches!(
            request.to,
            WorkflowTaskState::Claimed | WorkflowTaskState::Running
        ) {
            if request.max_concurrency == 0 || request.active_concurrency >= request.max_concurrency
            {
                return denied(WorkflowTaskResultStatus::ConcurrencyBlocked);
            }
        }
        if request.to == WorkflowTaskState::Running {
            match request.lease.as_ref() {
                Some(lease) if lease.revoked => {
                    return denied(WorkflowTaskResultStatus::LeaseRevoked)
                }
                Some(lease) if !lease.is_active_at(request.now_epoch_ms) => {
                    return denied(WorkflowTaskResultStatus::LeaseExpired)
                }
                Some(_) => {}
                None => return denied(WorkflowTaskResultStatus::LeaseExpired),
            }
        }
        if request.to == WorkflowTaskState::Failed
            && (request.max_attempts == 0 || request.attempt_index >= request.max_attempts)
        {
            return denied(WorkflowTaskResultStatus::RetryExhausted);
        }
        if !request.timeout_within_limit {
            return denied(WorkflowTaskResultStatus::QuotaExceeded);
        }
        if request.to == WorkflowTaskState::Completed && !request.artifact_reference_valid {
            return denied(WorkflowTaskResultStatus::ArtifactBlocked);
        }
        if !request.redaction_policy_valid {
            return denied(WorkflowTaskResultStatus::Denied);
        }
        if is_terminal(request.to) && !request.terminal_transition_approved {
            return denied(WorkflowTaskResultStatus::Denied);
        }
        WorkflowTaskTransitionDecision {
            status: WorkflowTaskResultStatus::Success,
            allowed: true,
        }
    }
}

fn is_declared_transition(from: WorkflowTaskState, to: WorkflowTaskState) -> bool {
    matches!(
        (from, to),
        (WorkflowTaskState::Draft, WorkflowTaskState::Queued)
            | (WorkflowTaskState::Draft, WorkflowTaskState::Cancelled)
            | (WorkflowTaskState::Queued, WorkflowTaskState::Claimed)
            | (WorkflowTaskState::Queued, WorkflowTaskState::Blocked)
            | (WorkflowTaskState::Queued, WorkflowTaskState::Cancelled)
            | (WorkflowTaskState::Claimed, WorkflowTaskState::Running)
            | (WorkflowTaskState::Claimed, WorkflowTaskState::Queued)
            | (WorkflowTaskState::Claimed, WorkflowTaskState::Cancelled)
            | (WorkflowTaskState::Running, WorkflowTaskState::Review)
            | (WorkflowTaskState::Running, WorkflowTaskState::Completed)
            | (WorkflowTaskState::Running, WorkflowTaskState::Failed)
            | (WorkflowTaskState::Running, WorkflowTaskState::Cancelled)
            | (WorkflowTaskState::Running, WorkflowTaskState::Queued)
            | (WorkflowTaskState::Blocked, WorkflowTaskState::Queued)
            | (WorkflowTaskState::Review, WorkflowTaskState::Completed)
            | (WorkflowTaskState::Review, WorkflowTaskState::Running)
            | (WorkflowTaskState::Review, WorkflowTaskState::Cancelled)
    )
}

fn is_mutating(target: WorkflowTaskState) -> bool {
    !matches!(target, WorkflowTaskState::Draft)
}

fn is_terminal(state: WorkflowTaskState) -> bool {
    matches!(
        state,
        WorkflowTaskState::Completed
            | WorkflowTaskState::Failed
            | WorkflowTaskState::Cancelled
            | WorkflowTaskState::Skipped
    )
}

fn has_bounded_mutation_identity(
    idempotency_key: Option<&str>,
    expected_version: Option<&str>,
) -> bool {
    [idempotency_key, expected_version]
        .into_iter()
        .flatten()
        .any(|value| {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        })
}

fn denied(status: WorkflowTaskResultStatus) -> WorkflowTaskTransitionDecision {
    WorkflowTaskTransitionDecision {
        status,
        allowed: false,
    }
}
