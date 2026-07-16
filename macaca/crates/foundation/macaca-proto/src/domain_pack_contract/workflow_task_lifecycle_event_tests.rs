use super::super::workflow_task::WorkflowTaskState;
use super::super::workflow_task_lifecycle_event::{
    WorkflowTaskLifecycleEvent, WorkflowTaskLifecycleEventKind,
};

#[test]
fn task_lifecycle_event_rejects_raw_or_unbounded_observability_values() {
    let event = valid_event();
    assert!(event.is_trace_safe());
    for invalid_ref in ["", "raw_prompt", "secret:token", &"a".repeat(129)] {
        let invalid = WorkflowTaskLifecycleEvent {
            task_ref: invalid_ref.into(),
            ..event.clone()
        };
        assert!(!invalid.is_trace_safe());
    }
}

fn valid_event() -> WorkflowTaskLifecycleEvent {
    WorkflowTaskLifecycleEvent {
        kind: WorkflowTaskLifecycleEventKind::Completed,
        trace_id: "trace:one".into(),
        task_ref: "task:one".into(),
        state: WorkflowTaskState::Completed,
        version_ref: "version:one".into(),
        queue_ref_hash: "hash:one".into(),
        attempt_index: 1,
        replay_ref: "replay:one".into(),
    }
}
