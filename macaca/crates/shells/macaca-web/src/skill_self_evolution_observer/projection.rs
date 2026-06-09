//! Adapter: project `AgentExecutionResult` into bounded `TaskResult` evidence.

use chrono::Utc;
use macaca_proto::{AgentExecutionResult, TaskId};
use macaca_sdk::runtime_host::executor::TaskResult;

use super::types::{MAX_ARTIFACT_REF_CHARS, MAX_ARTIFACT_REFS};

/// Build the task-result projection used by the Skill proposal command.
///
/// Agent Execution returns a structured service result instead of a kernel
/// executor event. The projection preserves only the stable task id, success
/// status, bounded output text, and completion time required to generate counts
/// and references; raw prompts and context snapshots never enter the proposal.
pub(crate) fn task_result_from_agent_execution_result(result: &AgentExecutionResult) -> TaskResult {
    let task_id = result.task_id.unwrap_or_else(TaskId::new);
    let output = agent_execution_output_text(&result.output);
    let artifacts = artifact_refs_from_agent_execution_result(result);
    if !artifacts.is_empty() {
        tracing::info!(
            trace_id = %result.trace.trace_id,
            task_id = %task_id,
            artifact_refs = artifacts.len(),
            "Skill self-evolution observer preserved bounded artifact evidence from Agent Execution metadata"
        );
    }
    TaskResult {
        task_id,
        success: true,
        output,
        error: None,
        artifacts,
        completed_at: Utc::now(),
        tokens_used: None,
    }
}
/// Return a bounded artifact reference suitable for proposal metadata.
pub(super) fn bounded_artifact_ref(artifact: &str) -> String {
    artifact.chars().take(MAX_ARTIFACT_REF_CHARS).collect()
}

/// Extract bounded artifact evidence that Web already collected for the
/// service.agent_execution result.
///
/// Agent Execution exposes durable write evidence as sanitized metadata instead
/// of raw file paths or file bodies.  The self-evolution observer must preserve
/// those refs when building the Skill proposal; otherwise downstream proposal
/// quality and materialization readiness cannot distinguish "useful artifact
/// produced" from "chat-only completion".  This helper deliberately accepts
/// only stable ref/digest fields and caps the number and size of refs.
fn artifact_refs_from_agent_execution_result(result: &AgentExecutionResult) -> Vec<String> {
    let mut refs = Vec::new();
    push_bounded_artifact_ref(&mut refs, result.metadata.get("artifact_ref"));
    for (key, value) in &result.metadata {
        if key.starts_with("evidence_ref.artifact_") {
            push_bounded_artifact_ref(&mut refs, Some(value));
        }
    }
    if refs.is_empty() {
        push_bounded_artifact_ref(&mut refs, result.metadata.get("artifact_digest"));
    }
    refs.sort();
    refs.dedup();
    refs.truncate(MAX_ARTIFACT_REFS);
    refs
}

fn push_bounded_artifact_ref(refs: &mut Vec<String>, value: Option<&String>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    refs.push(bounded_artifact_ref(trimmed));
}
/// Extract a bounded textual completion signal from Agent Execution output.
///
/// Service results commonly store chat output under the `output` key.  When a
/// provider returns a different bounded JSON shape, falling back to the compact
/// JSON string preserves replay evidence without copying prompts or context
/// bodies; the proposal command still records only character counts and refs.
pub(crate) fn agent_execution_output_text(output: &serde_json::Value) -> String {
    output
        .get("output")
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| output.to_string())
}
