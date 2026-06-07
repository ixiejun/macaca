//! Agent execution backend test module — heartbeat evidence and prompt context.
//!
//! Part of the `agent_execution_backend/tests/` Facade module tree (P3 §4.5.1).
//! Validates execution control, heartbeat evidence, envelope contracts, and
//! architecture governance boundaries without application-specific business logic.


use super::support::*;
#[test]
fn heartbeat_intent_requires_heartbeat_source_evidence() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-missing-profile"),
    )
    .unwrap();
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");

    assert!(should_skip_heartbeat_without_source(&command, &snapshot));
}

#[test]
fn heartbeat_intent_runs_when_heartbeat_source_evidence_exists() {
    let command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-profile-present"),
    )
    .unwrap();
    let context_command = AgentContextBuildCommand::from_execution(&command);
    let mut snapshot = AgentContextSnapshot::minimal(&context_command, "trusted context");
    snapshot.sources.push(macaca_proto::AgentContextSource {
        kind: "profile_file".into(),
        name: "HEARTBEAT.md".into(),
        location: Some("personas/coordinator/HEARTBEAT.md".into()),
        metadata: Default::default(),
    });

    assert!(!should_skip_heartbeat_without_source(&command, &snapshot));
}

#[test]
fn evidence_metadata_is_rendered_as_structured_execution_context() {
    let mut command = AgentExecutionCommand::new(
        macaca_proto::ApplicationId::from_name("demo"),
        "session-a",
        "coordinator",
        AgentExecutionIntent::Heartbeat,
        "run heartbeat work",
        macaca_proto::TraceContext::new("trace-heartbeat-evidence-context"),
    )
    .unwrap();
    command.delegated_context = serde_json::json!({
        "heartbeat": {
            "run_id": "run-a"
        }
    });
    command.metadata.insert(
        "evidence.expected_artifact_path".into(),
        "/workspace/agents/a/sentinel.md".into(),
    );

    let prompt = user_prompt_with_context(&command);

    assert!(prompt.contains("Structured evidence context"));
    assert!(prompt.contains("\"delegated_context\""));
    assert!(prompt.contains("\"evidence_requirements\""));
    assert!(prompt.contains("\"expected_artifact_path\""));
    assert!(prompt.contains("/workspace/agents/a/sentinel.md"));
}
