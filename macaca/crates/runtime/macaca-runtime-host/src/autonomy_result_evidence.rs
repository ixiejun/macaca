//! Provider-neutral evidence gate for autonomous execution completion.
//!
//! Runtime Host must never infer "task completed" from a transport success or
//! from `AgentExecutionStatus::Completed` alone.  This small Specification
//! object centralizes the safe evidence vocabulary that autonomy dispatchers
//! may use when classifying results.  The accepted keys are generic audit,
//! artifact, and output-reference concepts; they deliberately avoid
//! application names, workflow names, provider names, model names, business
//! domains, raw prompts, and unbounded provider payloads.

use macaca_proto::{AgentExecutionResult, AgentExecutionStatus};

/// Classification returned after inspecting an Agent Execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentExecutionEvidenceDecision {
    /// The result is completed and carries at least one sanitized evidence ref.
    Verified { evidence_key: &'static str },
    /// Agent Execution completed but did not expose replayable result evidence.
    MissingEvidence,
    /// The result was not completed, so callers should use the status-specific
    /// scheduler/heartbeat outcome rather than evidence-gate success.
    NotCompleted,
}

/// Specification object for autonomy result evidence.
pub(crate) struct AgentExecutionEvidenceGate;

impl AgentExecutionEvidenceGate {
    /// Evaluate whether a completed Agent Execution result can be treated as
    /// autonomous task success.
    ///
    /// The gate accepts bounded evidence references from metadata first because
    /// metadata is designed for audit correlation. It also accepts explicit
    /// structured evidence fields in the sanitized output object. It never
    /// parses natural-language model text or application-specific file paths.
    pub(crate) fn evaluate(result: &AgentExecutionResult) -> AgentExecutionEvidenceDecision {
        if result.status != AgentExecutionStatus::Completed {
            return AgentExecutionEvidenceDecision::NotCompleted;
        }
        for key in [
            "result_evidence_ref",
            "artifact_ref",
            "artifact_digest",
            "audit_id",
        ] {
            if result
                .metadata
                .get(key)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
            {
                return AgentExecutionEvidenceDecision::Verified { evidence_key: key };
            }
        }
        if let Some(object) = result.output.as_object() {
            for key in [
                "evidence_ref",
                "artifact_ref",
                "artifact_digest",
                "audit_id",
            ] {
                if object
                    .get(key)
                    .and_then(|value| value.as_str())
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
                {
                    return AgentExecutionEvidenceDecision::Verified { evidence_key: key };
                }
            }
            for key in ["evidence_refs", "artifacts"] {
                if object
                    .get(key)
                    .and_then(|value| value.as_array())
                    .map(|items| !items.is_empty())
                    .unwrap_or(false)
                {
                    return AgentExecutionEvidenceDecision::Verified { evidence_key: key };
                }
            }
        }
        AgentExecutionEvidenceDecision::MissingEvidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentExecutionCommand, AgentExecutionIntent, ApplicationId, TraceContext};

    fn completed_result() -> AgentExecutionResult {
        let command = AgentExecutionCommand::new(
            ApplicationId::from_name("evidence-gate-test"),
            "session-a",
            "worker",
            AgentExecutionIntent::TaskWorker,
            "perform durable work",
            TraceContext::new("trace-evidence-gate"),
        )
        .unwrap();
        AgentExecutionResult::completed(&command, serde_json::json!({"output": "done"}))
    }

    #[test]
    fn output_hash_alone_is_not_autonomous_completion_evidence() {
        let mut result = completed_result();
        result
            .metadata
            .insert("result_output_hash".into(), "abcd1234".into());

        assert_eq!(
            AgentExecutionEvidenceGate::evaluate(&result),
            AgentExecutionEvidenceDecision::MissingEvidence
        );
    }

    #[test]
    fn artifact_metadata_is_autonomous_completion_evidence() {
        let mut result = completed_result();
        result
            .metadata
            .insert("artifact_ref".into(), "tool:file_write:abcd1234".into());

        assert_eq!(
            AgentExecutionEvidenceGate::evaluate(&result),
            AgentExecutionEvidenceDecision::Verified {
                evidence_key: "artifact_ref"
            }
        );
    }
}
