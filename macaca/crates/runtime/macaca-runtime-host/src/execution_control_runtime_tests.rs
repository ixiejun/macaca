use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use macaca_proto::{
    ApplicationId, ExecutionControlCancelWaitCommand, ExecutionControlCheckpointCommand,
    ExecutionControlCheckpointMode, ExecutionControlEvent, ExecutionControlPauseCommand,
    ExecutionControlPolicy, ExecutionControlRegisterExecutionCommand,
    ExecutionControlResolutionStatus, ExecutionControlResolvedPolicy,
    ExecutionControlResumeCommand, ExecutionControlResumeSource, ExecutionControlScope,
    ExecutionControlState, ExecutionControlStateCommand, ExecutionControlTrigger, TraceContext,
};

use super::{ExecutionControlObserver, ExecutionControlRuntimeCapability};

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<ExecutionControlEvent>>,
}

impl ExecutionControlObserver for RecordingObserver {
    fn observe(&self, event: &ExecutionControlEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

fn scope() -> ExecutionControlScope {
    let mut scope = ExecutionControlScope::new(
        ApplicationId::from_name("demo"),
        "session-a",
        "execution-a",
        TraceContext::new("trace-execution-control-runtime"),
        "unit-test",
    )
    .unwrap();
    scope
        .metadata
        .insert("raw_prompt".into(), "do something private".into());
    scope.metadata.insert("safe".into(), "visible".into());
    scope
}

fn enabled_policy() -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Enabled,
        policy: Some(ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        )),
        reason_code: "execution_control.enabled.test".into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

fn enabled_policy_for(
    triggers: Vec<ExecutionControlTrigger>,
    resume_sources: Vec<ExecutionControlResumeSource>,
) -> ExecutionControlResolvedPolicy {
    ExecutionControlResolvedPolicy {
        status: ExecutionControlResolutionStatus::Enabled,
        policy: Some(ExecutionControlPolicy::enabled(
            triggers,
            resume_sources,
            ExecutionControlCheckpointMode::ReferenceOnly,
        )),
        reason_code: "execution_control.enabled.test".into(),
        metadata: BTreeMap::new(),
        events: Vec::new(),
    }
}

#[test]
fn runtime_capability_registers_execution_and_notifies_observer() {
    let observer = Arc::new(RecordingObserver::default());
    let runtime = ExecutionControlRuntimeCapability::with_observer(observer.clone());

    let result = runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope(),
            policy: enabled_policy(),
        })
        .unwrap();

    assert_eq!(result.state, ExecutionControlState::Running);
    assert_eq!(result.events.len(), 1);
    assert_eq!(
        observer.events.lock().unwrap()[0].reason_code,
        "execution_control.execution_registered"
    );
}

#[test]
fn runtime_capability_records_pause_checkpoint_resume_and_snapshot() {
    let runtime = ExecutionControlRuntimeCapability::new();
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy(),
        })
        .unwrap();

    let pause = runtime
        .request_pause(ExecutionControlPauseCommand {
            scope: scope.clone(),
            trigger: ExecutionControlTrigger::tool_call_barrier("create_goal"),
            reason_code: "execution_control.pause.test".into(),
        })
        .unwrap();
    assert_eq!(pause.state, ExecutionControlState::Paused);

    let checkpoint = runtime
        .record_checkpoint(ExecutionControlCheckpointCommand {
            scope: scope.clone(),
            checkpoint_ref: "checkpoint://safe-reference".into(),
            checkpoint_mode: ExecutionControlCheckpointMode::ReferenceOnly,
        })
        .unwrap();
    assert_eq!(checkpoint.state, ExecutionControlState::Paused);

    let resume = runtime
        .request_resume(ExecutionControlResumeCommand {
            scope: scope.clone(),
            resume_source: ExecutionControlResumeSource::goal_lifecycle(),
            reason_code: "execution_control.resume.test".into(),
        })
        .unwrap();
    assert_eq!(resume.state, ExecutionControlState::Resuming);

    let queried = runtime
        .query_state(ExecutionControlStateCommand {
            scope: scope.clone(),
        })
        .unwrap();
    assert_eq!(queried.state, ExecutionControlState::Resuming);

    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.executions.len(), 1);
    assert_eq!(
        snapshot.executions[0].checkpoint_refs,
        vec!["checkpoint://safe-reference".to_string()]
    );
    assert!(snapshot.executions[0]
        .events
        .iter()
        .all(|event| !event.metadata.contains_key("raw_prompt")));
}

#[test]
fn runtime_capability_replays_goal_pause_resume_audit_chain() {
    let runtime = ExecutionControlRuntimeCapability::new();
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy_for(
                vec![ExecutionControlTrigger::GoalLifecycle],
                vec![ExecutionControlResumeSource::goal_lifecycle()],
            ),
        })
        .unwrap();
    runtime
        .request_pause(ExecutionControlPauseCommand {
            scope: scope.clone(),
            trigger: ExecutionControlTrigger::GoalLifecycle,
            reason_code: "execution_control.pause.goal".into(),
        })
        .unwrap();
    runtime
        .record_checkpoint(ExecutionControlCheckpointCommand {
            scope: scope.clone(),
            checkpoint_ref: "checkpoint://goal-lifecycle".into(),
            checkpoint_mode: ExecutionControlCheckpointMode::ReferenceOnly,
        })
        .unwrap();
    runtime
        .request_resume(ExecutionControlResumeCommand {
            scope,
            resume_source: ExecutionControlResumeSource::goal_lifecycle(),
            reason_code: "execution_control.resume.goal".into(),
        })
        .unwrap();

    let snapshot = runtime.snapshot();
    let replayed_reason_codes: Vec<&str> = snapshot.executions[0]
        .events
        .iter()
        .map(|event| event.reason_code.as_str())
        .collect();

    assert_eq!(
        replayed_reason_codes,
        vec![
            "execution_control.execution_registered",
            "execution_control.pause_requested",
            "execution_control.pause_entered",
            "execution_control.checkpoint_recorded",
            "execution_control.resume_requested",
            "execution_control.resume_delivered",
        ]
    );
}

#[test]
fn runtime_capability_replays_app_selected_pause_resume_audit_chain() {
    let runtime = ExecutionControlRuntimeCapability::new();
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy_for(
                vec![ExecutionControlTrigger::ApplicationLifecycle],
                vec![ExecutionControlResumeSource::ApplicationLifecycle],
            ),
        })
        .unwrap();
    runtime
        .request_pause(ExecutionControlPauseCommand {
            scope: scope.clone(),
            trigger: ExecutionControlTrigger::ApplicationLifecycle,
            reason_code: "execution_control.pause.application_lifecycle".into(),
        })
        .unwrap();
    runtime
        .request_resume(ExecutionControlResumeCommand {
            scope,
            resume_source: ExecutionControlResumeSource::ApplicationLifecycle,
            reason_code: "execution_control.resume.application_lifecycle".into(),
        })
        .unwrap();

    let snapshot = runtime.snapshot();
    let replayed_states: Vec<ExecutionControlState> = snapshot.executions[0]
        .events
        .iter()
        .map(|event| event.state.clone())
        .collect();

    assert_eq!(
        replayed_states,
        vec![
            ExecutionControlState::Running,
            ExecutionControlState::PauseRequested,
            ExecutionControlState::Paused,
            ExecutionControlState::ResumeRequested,
            ExecutionControlState::Resuming,
        ]
    );
    assert!(snapshot.executions[0]
        .events
        .iter()
        .all(|event| !event.metadata.contains_key("raw_prompt")));
}

#[test]
fn runtime_capability_records_sanitized_state_query_event() {
    let observer = Arc::new(RecordingObserver::default());
    let runtime = ExecutionControlRuntimeCapability::with_observer(observer.clone());
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy(),
        })
        .unwrap();

    let queried = runtime
        .query_state(ExecutionControlStateCommand {
            scope: scope.clone(),
        })
        .unwrap();

    assert_eq!(queried.state, ExecutionControlState::Running);
    assert_eq!(
        queried.events.last().unwrap().reason_code,
        "execution_control.state_queried"
    );
    let observed = observer.events.lock().unwrap();
    let event = observed
        .last()
        .expect("state query should be observed before diagnostics are returned");
    assert_eq!(event.reason_code, "execution_control.state_queried");
    assert_eq!(
        event.metadata.get("safe").map(String::as_str),
        Some("visible")
    );
    assert!(!event.metadata.contains_key("raw_prompt"));
}

#[test]
fn runtime_capability_records_timeout_as_terminal_wait_event() {
    let observer = Arc::new(RecordingObserver::default());
    let runtime = ExecutionControlRuntimeCapability::with_observer(observer.clone());
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy(),
        })
        .unwrap();

    let timed_out = runtime
        .cancel_wait(ExecutionControlCancelWaitCommand {
            scope,
            reason_code: "execution_control.timeout.test".into(),
            timeout_ms: Some(25),
        })
        .unwrap();

    assert_eq!(timed_out.state, ExecutionControlState::TimedOut);
    assert_eq!(
        timed_out.events.last().unwrap().reason_code,
        "execution_control.wait_timed_out"
    );
    assert_eq!(
        timed_out.events.last().unwrap().metadata.get("timeout_ms"),
        Some(&"25".to_string())
    );
    assert_eq!(
        observer.events.lock().unwrap().last().unwrap().state,
        ExecutionControlState::TimedOut
    );
}

#[test]
fn runtime_capability_records_cancellation_as_terminal_wait_event() {
    let runtime = ExecutionControlRuntimeCapability::new();
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy(),
        })
        .unwrap();

    let cancelled = runtime
        .cancel_wait(ExecutionControlCancelWaitCommand {
            scope,
            reason_code: "execution_control.cancel.test".into(),
            timeout_ms: None,
        })
        .unwrap();

    assert_eq!(cancelled.state, ExecutionControlState::Cancelled);
    assert_eq!(
        cancelled.events.last().unwrap().reason_code,
        "execution_control.wait_cancelled"
    );
}

#[test]
fn runtime_capability_rejects_duplicate_resume_after_delivery() {
    let runtime = ExecutionControlRuntimeCapability::new();
    let scope = scope();
    runtime
        .register_execution(ExecutionControlRegisterExecutionCommand {
            scope: scope.clone(),
            policy: enabled_policy(),
        })
        .unwrap();
    runtime
        .request_resume(ExecutionControlResumeCommand {
            scope: scope.clone(),
            resume_source: ExecutionControlResumeSource::goal_lifecycle(),
            reason_code: "execution_control.resume.first".into(),
        })
        .unwrap();

    let duplicate = runtime
        .request_resume(ExecutionControlResumeCommand {
            scope,
            resume_source: ExecutionControlResumeSource::goal_lifecycle(),
            reason_code: "execution_control.resume.second".into(),
        })
        .unwrap();

    assert_eq!(duplicate.state, ExecutionControlState::Resuming);
    assert_eq!(
        duplicate.events.last().unwrap().reason_code,
        "execution_control.resume_rejected"
    );
}
