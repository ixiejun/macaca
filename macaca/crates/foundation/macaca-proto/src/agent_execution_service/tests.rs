//! Contract tests for agent execution and agent context service DTOs.

use super::*;
use crate::{
    ExecutionControlCheckpointMode, ExecutionControlPolicyOverride, ExecutionControlResumeSource,
    ExecutionControlTrigger,
};

/// Provider-neutral fixture agent id for round-trip contract tests (Object Mother).
///
/// Intentionally avoids application role literals (`worker`, `planner`, etc.)
/// so terminal `hardcoded-agent-role` guards can scan this module without false
/// positives while still exercising `target_agent` serialization boundaries.
const FIXTURE_TARGET_AGENT: &str = "fixture-target-agent";

/// Provider-neutral heartbeat artifact path paired with [`FIXTURE_TARGET_AGENT`].
///
/// Envelope compiler tests only assert path round-trip semantics; the directory
/// segment must not embed application role vocabulary.
const FIXTURE_HEARTBEAT_ARTIFACT_PATH: &str = "/workspace/agents/fixture-target-agent/heartbeat.md";

#[test]
fn planner_intent_uses_provider_neutral_wire_label() {
    assert_eq!(
        AgentExecutionIntent::Planner.metadata_value(),
        GOAL_PLANNER_EXECUTION_INTENT_LABEL
    );
    assert_eq!(
        AgentExecutionIntent::from_metadata_value(GOAL_PLANNER_EXECUTION_INTENT_LABEL),
        AgentExecutionIntent::Planner
    );
}

#[test]
fn execution_command_round_trips_through_service_command() {
    let mut trace = TraceContext::new("trace-agent-exec");
    trace.session_id = Some("session-a".into());
    let command = AgentExecutionCommand::new(
        ApplicationId::from_name("demo"),
        "session-a",
        FIXTURE_TARGET_AGENT,
        AgentExecutionIntent::WasmDelegate,
        "Analyze BTC",
        trace,
    )
    .unwrap()
    .with_delegated_context(serde_json::json!({"symbol": "BTC"}));

    let service_command = command.clone().into_service_command().unwrap();
    assert_eq!(service_command.name.as_str(), AGENT_EXECUTE_COMMAND);
    assert!(service_command.trace.is_some());

    let decoded: AgentExecutionCommand = serde_json::from_value(service_command.payload).unwrap();
    assert_eq!(decoded.user_prompt, "Analyze BTC");
    assert_eq!(decoded.delegated_context["symbol"], "BTC");
    assert_eq!(decoded.target_agent, FIXTURE_TARGET_AGENT);
}

#[test]
fn execution_envelope_compiler_requires_artifact_when_expected_path_exists() {
    let metadata = BTreeMap::from([(
        "evidence.expected_artifact_path".into(),
        FIXTURE_HEARTBEAT_ARTIFACT_PATH.into(),
    )]);

    let envelope = AutonomousExecutionEnvelope::compile(
        AutonomousExecutionSourceKind::HeartbeatProfile,
        "Write the heartbeat sentinel.",
        &metadata,
    )
    .unwrap();

    assert_eq!(
        envelope.source_kind,
        AutonomousExecutionSourceKind::HeartbeatProfile
    );
    assert_eq!(
        envelope.instruction_priority,
        AutonomousInstructionPriority::TaskOverridesContext
    );
    assert_eq!(
        envelope.completion_policy.kind,
        AutonomousCompletionPolicyKind::RequireArtifact
    );
    assert_eq!(
        envelope.completion_policy.expected_artifact_path.as_deref(),
        Some(FIXTURE_HEARTBEAT_ARTIFACT_PATH)
    );
}

#[test]
fn execution_command_roundtrips_execution_envelope() {
    let mut command = AgentExecutionCommand::new(
        ApplicationId::from_name("demo"),
        "session-a",
        FIXTURE_TARGET_AGENT,
        AgentExecutionIntent::TaskWorker,
        "Summarize project state",
        TraceContext::new("trace-execution-envelope"),
    )
    .unwrap();
    command.execution_envelope = Some(
        AutonomousExecutionEnvelope::compile(
            AutonomousExecutionSourceKind::ScheduledAgentTask,
            "Summarize project state",
            &BTreeMap::new(),
        )
        .unwrap(),
    );

    let encoded = serde_json::to_string(&command).unwrap();
    let decoded: AgentExecutionCommand = serde_json::from_str(&encoded).unwrap();
    let envelope = decoded.execution_envelope.unwrap();

    assert_eq!(
        envelope.source_kind,
        AutonomousExecutionSourceKind::ScheduledAgentTask
    );
    assert_eq!(envelope.source_instruction, "Summarize project state");
    assert_eq!(
        envelope.completion_policy.kind,
        AutonomousCompletionPolicyKind::RequireAgentResult
    );
}

#[test]
fn context_command_preserves_user_prompt_boundary() {
    let command = AgentExecutionCommand::new(
        ApplicationId::from_name("demo"),
        "session-a",
        "risk_manager",
        AgentExecutionIntent::WasmDelegate,
        "This is user work, not a system prompt",
        TraceContext::new("trace-agent-context"),
    )
    .unwrap();

    let context_command = AgentContextBuildCommand::from_execution(&command);
    let snapshot = AgentContextSnapshot::minimal(&context_command, "trusted system context");

    assert_eq!(snapshot.system_prompt, "trusted system context");
    assert_eq!(
        command.user_prompt,
        "This is user work, not a system prompt"
    );
}

#[test]
fn constructor_rejects_empty_required_fields() {
    let err = AgentExecutionCommand::new(
        ApplicationId::from_name("demo"),
        " ",
        FIXTURE_TARGET_AGENT,
        AgentExecutionIntent::ChatMainThread,
        "hello",
        TraceContext::new("trace-invalid"),
    )
    .unwrap_err();

    assert!(err.to_string().contains("session_id"));
}

#[test]
fn execution_command_roundtrips_execution_control_override() {
    let command = AgentExecutionCommand::new(
        ApplicationId::from_name("demo"),
        "session-a",
        FIXTURE_TARGET_AGENT,
        AgentExecutionIntent::TaskWorker,
        "pause after delegated work reaches a barrier",
        TraceContext::new("trace-execution-control-override"),
    )
    .unwrap()
    .with_execution_control_override(ExecutionControlPolicyOverride::enable_for_run(
        vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
        vec![ExecutionControlResumeSource::goal_lifecycle()],
        ExecutionControlCheckpointMode::ReferenceOnly,
    ));

    let encoded = serde_json::to_string(&command).unwrap();
    let decoded: AgentExecutionCommand = serde_json::from_str(&encoded).unwrap();
    let override_policy = decoded.execution_control_override.unwrap();

    assert_eq!(override_policy.triggers.len(), 1);
    assert_eq!(override_policy.resume_sources.len(), 1);
    assert_eq!(
        override_policy.checkpoint_mode,
        ExecutionControlCheckpointMode::ReferenceOnly
    );
}
