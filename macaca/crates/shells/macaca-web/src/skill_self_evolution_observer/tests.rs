//! Unit tests for bounded Skill self-evolution observation and proposal building.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_host_composition::runtime_host::SkillExperienceEvidenceGateStatus;
use macaca_proto::{
    AgentExecutionResult, AgentExecutionStatus, ApplicationId, TaskId, TaskResult, TokenUsage,
    TraceContext,
};

use super::projection::{agent_execution_output_text, task_result_from_agent_execution_result};
use super::proposal_builder::build_skill_experience_proposal_command;
use super::semantic_signal::semantic_trigger_phrases;

/// Provider-neutral fixture agent ids for observer contract tests.
const FIXTURE_EXECUTOR_AGENT: &str = "fixture-executor-agent";
const FIXTURE_ENTRY_AGENT: &str = "fixture-entry-agent";

#[test]
fn command_uses_only_bounded_refs_for_successful_task_completion() {
    let app_id = ApplicationId::new();
    let result = TaskResult {
        task_id: TaskId::new(),
        success: true,
        output: "raw task output that must not be copied into proposal bodies".into(),
        error: None,
        artifacts: vec!["/tmp/very-useful-artifact.txt".into()],
        completed_at: Utc::now(),
        tokens_used: Some(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        }),
    };

    let command = build_skill_experience_proposal_command(
        &app_id,
        "session-live-loop",
        FIXTURE_EXECUTOR_AGENT,
        &result,
        "trace-live-loop",
    )
    .expect("successful task should produce a proposal command");

    assert_eq!(command.scope.application_id, Some(app_id));
    assert_eq!(command.candidate.task_id, result.task_id.to_string());
    assert!(command.candidate.verified_terminal_success);
    assert_eq!(
        command.candidate.evidence_gate,
        SkillExperienceEvidenceGateStatus::Accepted
    );
    assert!(command
        .candidate
        .evidence_ids
        .iter()
        .any(|id| id
            .starts_with("eventlog://sessions/session-live-loop/skill_self_evolution_observer/")));
    assert!(
        !command
            .candidate
            .bounded_summary
            .contains("raw task output"),
        "bounded summaries must not copy raw task output"
    );
    assert!(
        !command
            .candidate
            .reusable_procedure
            .contains("raw task output"),
        "procedure text must rely on refs instead of raw output"
    );
    assert!(command.candidate.validate().is_ok());
}

#[test]
fn command_is_skipped_without_replayable_completion_evidence() {
    let app_id = ApplicationId::new();
    let result = TaskResult {
        task_id: TaskId::new(),
        success: true,
        output: "  ".into(),
        error: None,
        artifacts: Vec::new(),
        completed_at: Utc::now(),
        tokens_used: None,
    };

    assert!(build_skill_experience_proposal_command(
        &app_id,
        "session-empty",
        FIXTURE_EXECUTOR_AGENT,
        &result,
        "trace-empty"
    )
    .is_none());
}

#[test]
fn agent_execution_metadata_artifact_ref_becomes_proposal_artifact_evidence() {
    let app_id = ApplicationId::new();
    let task_id = TaskId::new();
    let mut metadata = BTreeMap::new();
    metadata.insert("artifact_ref".into(), "tool:file_write:abcd1234".into());
    metadata.insert("artifact_digest".into(), "digest://artifact/1".into());
    let result = AgentExecutionResult {
        application_id: app_id,
        session_id: "session-artifact-evidence".into(),
        task_id: Some(task_id),
        target_agent: FIXTURE_ENTRY_AGENT.into(),
        status: AgentExecutionStatus::Completed,
        output: serde_json::json!({
            "output": "agent execution completed with a written artifact"
        }),
        context_snapshot: None,
        trace: TraceContext::new("test-artifact-evidence"),
        metadata,
    };

    let task_result = task_result_from_agent_execution_result(&result);
    let command = build_skill_experience_proposal_command(
        &app_id,
        &result.session_id,
        &result.target_agent,
        &task_result,
        &result.trace.trace_id,
    )
    .expect("artifact-backed execution should produce a proposal command");

    assert!(command
        .candidate
        .bounded_summary
        .contains("artifact_count=1"));
    assert_eq!(
        command
            .candidate
            .metadata
            .get("evidence_ref.artifact_0")
            .map(String::as_str),
        Some("tool:file_write:abcd1234")
    );
}

#[test]
fn command_derives_semantic_skill_creator_identity_from_bounded_completion_signal() {
    let app_id = ApplicationId::new();
    let result = TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: "Wrote a reusable verification note covering materialization, proposal-linked Skill package creation, registry load-path visibility, usage telemetry, semantic skill naming, and measurable follow-up optimization.".into(),
            error: None,
            artifacts: vec!["tool:file_write:semantic-live-note".into()],
            completed_at: Utc::now(),
            tokens_used: None,
        };

    let command = build_skill_experience_proposal_command(
        &app_id,
        "session-semantic-identity",
        FIXTURE_ENTRY_AGENT,
        &result,
        "trace-semantic-identity",
    )
    .expect("bounded semantic completion should produce a proposal command");

    assert_eq!(
        command.candidate.target_skill_name.as_deref(),
        Some("materialization-proposal-linked-skill-package-registry-load-path")
    );
    assert!(command
        .candidate
        .reusable_procedure
        .contains("materialization"));
    assert!(command.candidate.reusable_procedure.contains("telemetry"));
    assert!(
        !command
            .candidate
            .reusable_procedure
            .contains("semantic-live-note"),
        "Skill Creator-facing trigger text must not copy raw artifact refs"
    );
    assert!(command.candidate.validate().is_ok());
}

#[test]
fn semantic_identity_uses_priority_phrases_across_realistic_artifact_summary() {
    let phrases = semantic_trigger_phrases(
        "Self-Evolution Materialization. Proposal-Linked Skill Creation. \
             Every Skill package should be traceable. Registry Load-Path Visibility. \
             Usage Telemetry and measurable follow-up optimization.",
    );

    assert_eq!(
        phrases,
        vec![
            "materialization",
            "proposal-linked",
            "skill-package",
            "registry-load-path"
        ]
    );
}

#[tokio::test]
async fn agent_execution_result_converts_to_bounded_completion_event() {
    let app_id = ApplicationId::new();
    let task_id = TaskId::new();
    let result = AgentExecutionResult {
        application_id: app_id,
        session_id: "session-agent-execution".into(),
        task_id: Some(task_id),
        target_agent: FIXTURE_ENTRY_AGENT.into(),
        status: AgentExecutionStatus::Completed,
        output: serde_json::json!({
            "output": "agent execution completed with reusable evidence"
        }),
        context_snapshot: None,
        trace: TraceContext::new("test-agent-execution-result-observer"),
        metadata: BTreeMap::new(),
    };

    let output = agent_execution_output_text(&result.output);
    let task_result = TaskResult {
        task_id,
        success: true,
        output,
        error: None,
        artifacts: Vec::new(),
        completed_at: Utc::now(),
        tokens_used: None,
    };

    let command = build_skill_experience_proposal_command(
        &app_id,
        &result.session_id,
        &result.target_agent,
        &task_result,
        &result.trace.trace_id,
    )
    .expect("completed agent execution should produce a proposal command");

    assert_eq!(command.candidate.task_id, task_id.to_string());
    assert_eq!(
        command.candidate.agent_name.as_deref(),
        Some(FIXTURE_ENTRY_AGENT)
    );
    assert!(command
        .candidate
        .evidence_ids
        .iter()
        .any(|id| id == "trace://service.agent_execution/test-agent-execution-result-observer"));
    assert!(
        !command
            .candidate
            .bounded_summary
            .contains("agent execution completed"),
        "proposal summaries must carry counts and refs instead of raw output"
    );
}
