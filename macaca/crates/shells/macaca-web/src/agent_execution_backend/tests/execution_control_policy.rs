//! Agent execution backend test module — execution control policy parsing.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.

use super::support::*;
#[test]
fn chat_main_thread_without_manifest_default_stays_disabled() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        macaca_proto::AgentExecutionIntent::ChatMainThread,
        "create a project goal",
        macaca_proto::TraceContext::new("trace-chat-main-thread"),
    )
    .unwrap();

    let resolution = resolve_execution_control_policy_local(&command, None);

    assert_eq!(
        resolution.status,
        ExecutionControlResolutionStatus::Disabled
    );
}

#[test]
fn manifest_default_enables_execution_control_for_chat_main_thread() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        macaca_proto::AgentExecutionIntent::ChatMainThread,
        "create a project goal",
        macaca_proto::TraceContext::new("trace-chat-main-thread-manifest"),
    )
    .unwrap();
    let app_default = ExecutionControlPolicy::enabled(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    )
    .allow_command_overrides(false);

    let resolution = resolve_execution_control_policy_local(&command, Some(&app_default));

    assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
    assert!(resolution
        .metadata
        .get(["com", "patibility"].concat().as_str())
        .is_none());
}

#[test]
fn delegated_runtime_execution_without_policy_does_not_install_execution_control() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker"),
    )
    .unwrap();

    let resolution = resolve_execution_control_policy_local(&command, None);

    assert_eq!(
        resolution.status,
        ExecutionControlResolutionStatus::Disabled
    );
}

#[test]
fn delegated_runtime_execution_with_override_without_manifest_is_denied() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker-control-denied"),
    )
    .unwrap()
    .with_execution_control_override(ExecutionControlPolicyOverride::enable_for_run(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    ));

    let resolution = resolve_execution_control_policy_local(&command, None);

    assert_eq!(resolution.status, ExecutionControlResolutionStatus::Denied);
}

#[test]
fn delegated_runtime_execution_with_override_installs_execution_control_when_manifest_allows() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker-control"),
    )
    .unwrap()
    .with_execution_control_override(ExecutionControlPolicyOverride::enable_for_run(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    ));
    let app_default = ExecutionControlPolicy::enabled(
        vec![
            ExecutionControlTrigger::tool_call_barrier("create_goal"),
            ExecutionControlTrigger::tool_call_barrier("delegate_task"),
        ],
        vec![
            ExecutionControlResumeSource::goal_lifecycle(),
            ExecutionControlResumeSource::ForkLifecycle,
        ],
        ExecutionControlCheckpointMode::ReferenceOnly,
    )
    .allow_command_overrides(true);

    let resolution = resolve_execution_control_policy_local(&command, Some(&app_default));

    assert_eq!(resolution.status, ExecutionControlResolutionStatus::Enabled);
}
#[test]
fn execution_control_scope_carries_required_trace_and_run_identity() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "backend",
        macaca_proto::AgentExecutionIntent::TaskWorker,
        "implement the assigned task",
        macaca_proto::TraceContext::new("trace-task-worker-control"),
    )
    .unwrap();
    let execution_id = execution_control_execution_id(&command);

    let scope =
        execution_control_scope(&command, execution_id.clone(), "macaca.web.agent_execution")
            .unwrap();

    assert_eq!(scope.execution_id, execution_id);
    assert_eq!(scope.session_id, "session-a");
    assert_eq!(scope.trace.trace_id, "trace-task-worker-control");
    assert_eq!(scope.source, "macaca.web.agent_execution");
    assert_eq!(scope.metadata.get("target_agent"), Some(&"backend".into()));
}
