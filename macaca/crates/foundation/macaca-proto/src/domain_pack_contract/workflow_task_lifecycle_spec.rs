use serde::{Deserialize, Serialize};

use super::workflow_task::{WorkflowTaskResultStatus, WorkflowTaskState};

/// Bounded lifecycle states for provider-owned queue records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskQueueState {
    Ready,
    Claimed,
    Released,
    Blocked,
    Drained,
}

/// Bounded lifecycle states for one provider-owned task attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskAttemptState {
    Pending,
    Active,
    Retrying,
    Finished,
}

/// Bounded lifecycle states for retry scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskRetryState {
    NotScheduled,
    Scheduled,
    Exhausted,
}

/// Bounded lifecycle states for an individual dependency decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskDependencyState {
    Satisfied,
    Blocking,
    Cancelled,
}

/// Bounded lifecycle states for a concurrency-group reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskConcurrencyState {
    Reserved,
    Available,
    Saturated,
    Released,
}

/// Bounded lifecycle states for cancellation propagation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskCancellationState {
    NotRequested,
    Requested,
    Propagated,
    Acknowledged,
}

/// Sanitized provider snapshot used to verify that task facets form one valid lifecycle.
///
/// It carries enum states only. Identifiers, prompts, worker details, provider responses,
/// and raw history remain provider-owned references outside this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskLifecycleSnapshot {
    pub task: WorkflowTaskState,
    pub queue: WorkflowTaskQueueState,
    pub lease_active: bool,
    pub attempt: WorkflowTaskAttemptState,
    pub retry: WorkflowTaskRetryState,
    pub dependency: WorkflowTaskDependencyState,
    pub concurrency: WorkflowTaskConcurrencyState,
    pub cancellation: WorkflowTaskCancellationState,
}

/// Composite State/Specification contract shared by all workflow-task provider classes.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkflowTaskLifecycleSpec;

impl WorkflowTaskLifecycleSpec {
    /// Reject incoherent lifecycle facets before a provider emits a state mutation event.
    pub fn evaluate(&self, snapshot: &WorkflowTaskLifecycleSnapshot) -> WorkflowTaskResultStatus {
        if matches!(snapshot.dependency, WorkflowTaskDependencyState::Blocking)
            && !matches!(
                snapshot.task,
                WorkflowTaskState::Blocked | WorkflowTaskState::Queued
            )
        {
            return WorkflowTaskResultStatus::DependencyBlocked;
        }
        if matches!(
            snapshot.concurrency,
            WorkflowTaskConcurrencyState::Saturated
        ) && matches!(
            snapshot.task,
            WorkflowTaskState::Claimed | WorkflowTaskState::Running
        ) {
            return WorkflowTaskResultStatus::ConcurrencyBlocked;
        }
        if matches!(
            snapshot.task,
            WorkflowTaskState::Claimed | WorkflowTaskState::Running
        ) && (!snapshot.lease_active
            || !matches!(snapshot.attempt, WorkflowTaskAttemptState::Active))
        {
            return WorkflowTaskResultStatus::LeaseExpired;
        }
        if matches!(snapshot.retry, WorkflowTaskRetryState::Scheduled)
            && !matches!(snapshot.attempt, WorkflowTaskAttemptState::Retrying)
        {
            return WorkflowTaskResultStatus::InvalidState;
        }
        if matches!(snapshot.retry, WorkflowTaskRetryState::Exhausted)
            && !matches!(snapshot.task, WorkflowTaskState::Failed)
        {
            return WorkflowTaskResultStatus::RetryExhausted;
        }
        if is_terminal(snapshot.task)
            && (!matches!(snapshot.queue, WorkflowTaskQueueState::Drained)
                || snapshot.lease_active
                || !matches!(snapshot.attempt, WorkflowTaskAttemptState::Finished))
        {
            return WorkflowTaskResultStatus::InvalidState;
        }
        if snapshot.task == WorkflowTaskState::Cancelled
            && !matches!(
                snapshot.cancellation,
                WorkflowTaskCancellationState::Acknowledged
            )
        {
            return WorkflowTaskResultStatus::InvalidState;
        }
        WorkflowTaskResultStatus::Success
    }
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
