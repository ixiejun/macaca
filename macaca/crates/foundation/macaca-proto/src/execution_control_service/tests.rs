//! Contract tests for execution-control service DTOs and scope validation.

use super::*;
use super::*;

#[test]
fn execution_control_policy_roundtrips() {
    let policy = ExecutionControlPolicy::enabled(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    )
    .allow_command_overrides(true);

    let encoded = serde_json::to_string(&policy).unwrap();
    let decoded: ExecutionControlPolicy = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.mode, ExecutionControlMode::Enabled);
    assert!(decoded.allow_command_overrides);
    assert_eq!(decoded.triggers, policy.triggers);
    assert_eq!(decoded.resume_sources, policy.resume_sources);
}

#[test]
fn execution_control_scope_requires_trace() {
    let mut trace = TraceContext::new("trace-execution-control");
    trace.trace_id.clear();

    let result = ExecutionControlScope::new(
        ApplicationId::from_name("demo"),
        "session-a",
        "execution-a",
        trace,
        "test",
    );

    assert!(result.is_err());
}
