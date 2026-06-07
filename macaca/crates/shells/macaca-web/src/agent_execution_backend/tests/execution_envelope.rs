//! Agent execution backend test module — autonomous execution envelope and runtime budget.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.


use super::support::*;
#[test]
fn execution_envelope_is_rendered_as_highest_priority_contract() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-envelope-rendering"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run heartbeat work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    let prompt = user_prompt_with_context(&command);

    assert!(prompt.contains("Highest-priority delegated execution contract"));
    assert!(prompt.contains("\"source_kind\": \"heartbeat_profile\""));
    assert!(prompt.contains("\"kind\": \"require_artifact\""));
    assert!(prompt.contains("/workspace/agents/a/sentinel.md"));
}

#[test]
fn artifact_completion_policy_uses_short_runtime_loop_budget() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-artifact-budget"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run heartbeat work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    assert_eq!(runtime_agent_max_iters(&command), 12);
}

#[test]
fn artifact_completion_policy_requires_authorized_tool_use() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run artifact work",
        macaca_proto::TraceContext::new("trace-artifact-tool-choice"),
    )
    .unwrap();
    command.execution_envelope = Some(
        macaca_proto::AutonomousExecutionEnvelope::compile(
            macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile,
            "run artifact work",
            &std::collections::BTreeMap::from([(
                "evidence.expected_artifact_path".into(),
                "/workspace/agents/a/sentinel.md".into(),
            )]),
        )
        .unwrap(),
    );

    assert_eq!(
        runtime_agent_tool_choice(&command),
        Some(macaca_framework::model::ToolChoice::Required)
    );
}

#[test]
fn agent_result_completion_policy_keeps_automatic_tool_choice() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::TaskWorker,
        "summarize work",
        macaca_proto::TraceContext::new("trace-default-tool-choice"),
    )
    .unwrap();

    assert_eq!(runtime_agent_tool_choice(&command), None);
}

#[test]
fn agent_result_completion_policy_keeps_default_runtime_loop_budget() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::TaskWorker,
        "summarize work",
        macaca_proto::TraceContext::new("trace-default-budget"),
    )
    .unwrap();

    assert_eq!(runtime_agent_max_iters(&command), 25);
}
