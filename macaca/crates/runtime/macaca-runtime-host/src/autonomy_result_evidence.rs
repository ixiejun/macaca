//! Provider-neutral evidence gate for autonomous execution completion.
//!
//! Runtime Host must never infer "task completed" from a transport success or
//! from `AgentExecutionStatus::Completed` alone.  This small Specification
//! object centralizes the safe evidence vocabulary that autonomy dispatchers
//! may use when classifying results.  The accepted keys are generic audit,
//! artifact, and output-reference concepts; they deliberately avoid
//! application names, workflow names, provider names, model names, business
//! domains, raw prompts, and unbounded provider payloads.

use macaca_proto::{
    AgentExecutionResult, AgentExecutionStatus, AutonomousCompletionPolicy,
    AutonomousCompletionPolicyKind,
};

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
        first_present_metadata_key(
            result,
            &[
                "result_evidence_ref",
                "artifact_ref",
                "artifact_digest",
                "audit_id",
            ],
        )
        .or_else(|| {
            first_present_output_string_key(
                result,
                &[
                    "evidence_ref",
                    "artifact_ref",
                    "artifact_digest",
                    "audit_id",
                ],
            )
        })
        .or_else(|| first_present_output_array_key(result, &["evidence_refs", "artifacts"]))
        .map(|evidence_key| AgentExecutionEvidenceDecision::Verified { evidence_key })
        .unwrap_or(AgentExecutionEvidenceDecision::MissingEvidence)
    }

    /// Evaluate completion against a compiled autonomous completion policy.
    ///
    /// This Strategy entrypoint is used by scheduled and heartbeat dispatch once
    /// Runtime Host has compiled a generic `AutonomousExecutionEnvelope`. It
    /// avoids the old one-size-fits-all rule where every autonomous run needed
    /// artifact-style evidence. Natural-language tasks can now require only a
    /// completed agent result with bounded result evidence, while artifact
    /// tasks still need durable artifact metadata.
    pub(crate) fn evaluate_with_policy(
        result: &AgentExecutionResult,
        policy: &AutonomousCompletionPolicy,
    ) -> AgentExecutionEvidenceDecision {
        if result.status != AgentExecutionStatus::Completed {
            return AgentExecutionEvidenceDecision::NotCompleted;
        }
        match policy.kind {
            AutonomousCompletionPolicyKind::RequireAgentResult => first_present_metadata_key(
                result,
                &["result_output_hash", "result_evidence_ref", "audit_id"],
            )
            .or_else(|| {
                first_present_output_string_key(result, &["evidence_ref", "audit_id", "output_ref"])
            })
            .map(|evidence_key| AgentExecutionEvidenceDecision::Verified { evidence_key })
            .unwrap_or(AgentExecutionEvidenceDecision::MissingEvidence),
            AutonomousCompletionPolicyKind::RequireArtifact => first_present_metadata_key(
                result,
                &["artifact_ref", "artifact_digest", "result_evidence_ref"],
            )
            .or_else(|| {
                first_present_output_string_key(
                    result,
                    &["artifact_ref", "artifact_digest", "evidence_ref"],
                )
            })
            .or_else(|| first_present_output_array_key(result, &["artifacts", "evidence_refs"]))
            .map(|evidence_key| AgentExecutionEvidenceDecision::Verified { evidence_key })
            .unwrap_or(AgentExecutionEvidenceDecision::MissingEvidence),
            AutonomousCompletionPolicyKind::RequireStructuredOutput => first_present_metadata_key(
                result,
                &[
                    "structured_output_ref",
                    "structured_output_digest",
                    "result_evidence_ref",
                ],
            )
            .or_else(|| {
                first_present_output_string_key(
                    result,
                    &[
                        "structured_output_ref",
                        "structured_output_digest",
                        "evidence_ref",
                    ],
                )
            })
            .or_else(|| first_present_output_object_key(result, &["structured_output", "result"]))
            .map(|evidence_key| AgentExecutionEvidenceDecision::Verified { evidence_key })
            .unwrap_or(AgentExecutionEvidenceDecision::MissingEvidence),
            AutonomousCompletionPolicyKind::BestEffortWithAudit => first_present_metadata_key(
                result,
                &["audit_id", "result_output_hash", "result_evidence_ref"],
            )
            .or_else(|| {
                first_present_output_string_key(result, &["audit_id", "evidence_ref", "output_ref"])
            })
            .map(|evidence_key| AgentExecutionEvidenceDecision::Verified { evidence_key })
            .unwrap_or(AgentExecutionEvidenceDecision::MissingEvidence),
        }
    }
}

fn first_present_metadata_key(
    result: &AgentExecutionResult,
    keys: &[&'static str],
) -> Option<&'static str> {
    keys.iter().copied().find(|key| {
        result
            .metadata
            .get(*key)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn first_present_output_string_key(
    result: &AgentExecutionResult,
    keys: &[&'static str],
) -> Option<&'static str> {
    let object = result.output.as_object()?;
    keys.iter().copied().find(|key| {
        object
            .get(*key)
            .and_then(|value| value.as_str())
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    })
}

fn first_present_output_array_key(
    result: &AgentExecutionResult,
    keys: &[&'static str],
) -> Option<&'static str> {
    let object = result.output.as_object()?;
    keys.iter().copied().find(|key| {
        object
            .get(*key)
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    })
}

fn first_present_output_object_key(
    result: &AgentExecutionResult,
    keys: &[&'static str],
) -> Option<&'static str> {
    let object = result.output.as_object()?;
    keys.iter().copied().find(|key| {
        object
            .get(*key)
            .and_then(|value| value.as_object())
            .map(|object| !object.is_empty())
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        AgentExecutionCommand, AgentExecutionIntent, ApplicationId, AutonomousCompletionPolicy,
        AutonomousCompletionPolicyKind, TraceContext,
    };

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

    #[test]
    fn require_agent_result_accepts_bounded_output_hash() {
        let mut result = completed_result();
        result
            .metadata
            .insert("result_output_hash".into(), "abcd1234".into());
        let policy = AutonomousCompletionPolicy {
            kind: AutonomousCompletionPolicyKind::RequireAgentResult,
            expected_artifact_path: None,
            required_contains: Vec::new(),
        };

        assert_eq!(
            AgentExecutionEvidenceGate::evaluate_with_policy(&result, &policy),
            AgentExecutionEvidenceDecision::Verified {
                evidence_key: "result_output_hash"
            }
        );
    }

    #[test]
    fn require_artifact_rejects_output_hash_without_artifact_evidence() {
        let mut result = completed_result();
        result
            .metadata
            .insert("result_output_hash".into(), "abcd1234".into());
        let policy = AutonomousCompletionPolicy {
            kind: AutonomousCompletionPolicyKind::RequireArtifact,
            expected_artifact_path: Some("/workspace/sentinel.md".into()),
            required_contains: Vec::new(),
        };

        assert_eq!(
            AgentExecutionEvidenceGate::evaluate_with_policy(&result, &policy),
            AgentExecutionEvidenceDecision::MissingEvidence
        );
    }

    #[test]
    fn require_artifact_accepts_artifact_digest() {
        let mut result = completed_result();
        result
            .metadata
            .insert("artifact_digest".into(), "digest1234".into());
        let policy = AutonomousCompletionPolicy {
            kind: AutonomousCompletionPolicyKind::RequireArtifact,
            expected_artifact_path: Some("/workspace/sentinel.md".into()),
            required_contains: Vec::new(),
        };

        assert_eq!(
            AgentExecutionEvidenceGate::evaluate_with_policy(&result, &policy),
            AgentExecutionEvidenceDecision::Verified {
                evidence_key: "artifact_digest"
            }
        );
    }
}
