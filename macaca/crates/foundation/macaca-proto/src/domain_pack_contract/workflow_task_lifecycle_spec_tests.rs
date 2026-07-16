use super::super::workflow_task::{WorkflowTaskResultStatus, WorkflowTaskState};
use super::super::workflow_task_lifecycle_spec::*;

#[test]
fn workflow_task_lifecycle_spec_validates_every_lifecycle_facet() {
    assert_eq!(
        WorkflowTaskLifecycleSpec.evaluate(&valid_running()),
        WorkflowTaskResultStatus::Success
    );
    for invalid in invalid_snapshots() {
        assert_ne!(
            WorkflowTaskLifecycleSpec.evaluate(&invalid),
            WorkflowTaskResultStatus::Success
        );
    }
}

fn valid_running() -> WorkflowTaskLifecycleSnapshot {
    WorkflowTaskLifecycleSnapshot {
        task: WorkflowTaskState::Running,
        queue: WorkflowTaskQueueState::Claimed,
        lease_active: true,
        attempt: WorkflowTaskAttemptState::Active,
        retry: WorkflowTaskRetryState::NotScheduled,
        dependency: WorkflowTaskDependencyState::Satisfied,
        concurrency: WorkflowTaskConcurrencyState::Reserved,
        cancellation: WorkflowTaskCancellationState::NotRequested,
    }
}

fn invalid_snapshots() -> Vec<WorkflowTaskLifecycleSnapshot> {
    vec![
        WorkflowTaskLifecycleSnapshot {
            dependency: WorkflowTaskDependencyState::Blocking,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            concurrency: WorkflowTaskConcurrencyState::Saturated,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            lease_active: false,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            retry: WorkflowTaskRetryState::Scheduled,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            task: WorkflowTaskState::Queued,
            attempt: WorkflowTaskAttemptState::Retrying,
            retry: WorkflowTaskRetryState::Exhausted,
            queue: WorkflowTaskQueueState::Ready,
            lease_active: false,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            task: WorkflowTaskState::Completed,
            queue: WorkflowTaskQueueState::Claimed,
            attempt: WorkflowTaskAttemptState::Finished,
            lease_active: false,
            ..valid_running()
        },
        WorkflowTaskLifecycleSnapshot {
            task: WorkflowTaskState::Cancelled,
            queue: WorkflowTaskQueueState::Drained,
            attempt: WorkflowTaskAttemptState::Finished,
            lease_active: false,
            cancellation: WorkflowTaskCancellationState::Requested,
            ..valid_running()
        },
    ]
}
