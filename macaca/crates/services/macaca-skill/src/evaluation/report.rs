use serde_json::{json, Value};

use super::{SelfEvolutionEvaluationRecord, SelfEvolutionScore};

const REDACTED: &str = "[redacted]";
const MAX_RENDERED_FIELD_LEN: usize = 160;

/// Report builder for self-evolution evaluation results.
///
/// The builder is a thin presentation boundary over already-scored records. It
/// emits JSON-compatible and Markdown summaries made only from bounded refs,
/// counts, states, and reason codes. It intentionally does not render raw
/// prompts, raw provider payloads, package bytes, manifests, credentials, or
/// full skill bodies, which keeps shell adapters simple and audit-friendly.
pub struct SelfEvolutionReportBuilder;

impl SelfEvolutionReportBuilder {
    /// Build a sanitized JSON summary suitable for durable report storage.
    ///
    /// The returned `serde_json::Value` keeps the wire shape flexible for early
    /// provider integration while avoiding a shell-owned schema. Service and SDK
    /// layers can later wrap this value in typed command results without moving
    /// scoring semantics out of the evaluation provider.
    pub fn json_summary(
        record: &SelfEvolutionEvaluationRecord,
        score: &SelfEvolutionScore,
    ) -> Value {
        tracing::info!(
            evaluation_id = %record.evaluation_id,
            trace_id = %record.trace_id,
            task_family_id = %record.task_family_id,
            result_state = ?score.lifecycle,
            passed = score.passed,
            reason_count = score.reason_codes.len(),
            "building sanitized self-evolution evaluation JSON report"
        );

        json!({
            "evaluation_id": sanitize(&record.evaluation_id),
            "trace_id": sanitize(&record.trace_id),
            "task_family_id": sanitize(&record.task_family_id),
            "result": {
                "state": format!("{:?}", score.lifecycle),
                "passed": score.passed,
                "reason_codes": score.reason_codes,
            },
            "white_box": {
                "verified_task_completion_ref": sanitize_opt(&record.white_box.verified_task_completion_ref),
                "experience_candidate_ref": sanitize_opt(&record.white_box.experience_candidate_ref),
                "classification_ref": sanitize_opt(&record.white_box.classification_ref),
                "proposal_id": sanitize_opt(&record.white_box.proposal_id),
                "curation_run_id": sanitize_opt(&record.white_box.curation_run_id),
                "promotion_or_apply_ref": sanitize_opt(&record.white_box.promotion_or_apply_ref),
                "active_catalog_snapshot_ref": sanitize_opt(&record.white_box.active_catalog_snapshot_ref),
                "later_skill_activation_ref": sanitize_opt(&record.white_box.later_skill_activation_ref),
                "policy_decision_id": sanitize_opt(&record.white_box.policy_decision_id),
                "audit_event_count": record.white_box.audit_event_ids.len(),
                "before_snapshot_ref": sanitize_opt(&record.white_box.before_snapshot_ref),
                "after_snapshot_ref": sanitize_opt(&record.white_box.after_snapshot_ref),
                "rollback_ref": sanitize_opt(&record.white_box.rollback_ref),
            },
            "baseline": metrics_json(record.baseline.human_intervention_count, record.baseline.retry_count, record.baseline.elapsed_seconds, record.baseline.tool_call_count, record.baseline.verified_artifact_count, record.baseline.skill_activation_count),
            "evolved": metrics_json(record.evolved.human_intervention_count, record.evolved.retry_count, record.evolved.elapsed_seconds, record.evolved.tool_call_count, record.evolved.verified_artifact_count, record.evolved.skill_activation_count),
        })
    }

    /// Build a sanitized Markdown summary for operator review.
    ///
    /// The summary is deliberately compact: it names the evaluation, result,
    /// bounded reason codes, and metric counts. Detailed replay still belongs to
    /// Store/EventLog refs and audit tools, not to a Markdown blob.
    pub fn markdown_summary(
        record: &SelfEvolutionEvaluationRecord,
        score: &SelfEvolutionScore,
    ) -> String {
        tracing::info!(
            evaluation_id = %record.evaluation_id,
            trace_id = %record.trace_id,
            task_family_id = %record.task_family_id,
            result_state = ?score.lifecycle,
            passed = score.passed,
            reason_count = score.reason_codes.len(),
            "building sanitized self-evolution evaluation Markdown report"
        );

        let reasons = if score.reason_codes.is_empty() {
            "none".to_string()
        } else {
            score.reason_codes.join(", ")
        };

        format!(
            "# Self-Evolution Evaluation\n\n\
             - evaluation_id: {}\n\
             - trace_id: {}\n\
             - task_family_id: {}\n\
             - state: {:?}\n\
             - passed: {}\n\
             - reasons: {}\n\n\
             ## Baseline\n\n\
             - human_intervention_count: {}\n\
             - retry_count: {}\n\
             - elapsed_seconds: {}\n\
             - tool_call_count: {}\n\
             - verified_artifact_count: {}\n\
             - skill_activation_count: {}\n\n\
             ## Evolved\n\n\
             - human_intervention_count: {}\n\
             - retry_count: {}\n\
             - elapsed_seconds: {}\n\
             - tool_call_count: {}\n\
             - verified_artifact_count: {}\n\
             - skill_activation_count: {}\n",
            sanitize(&record.evaluation_id),
            sanitize(&record.trace_id),
            sanitize(&record.task_family_id),
            score.lifecycle,
            score.passed,
            reasons,
            record.baseline.human_intervention_count,
            record.baseline.retry_count,
            optional_u64(record.baseline.elapsed_seconds),
            record.baseline.tool_call_count,
            record.baseline.verified_artifact_count,
            record.baseline.skill_activation_count,
            record.evolved.human_intervention_count,
            record.evolved.retry_count,
            optional_u64(record.evolved.elapsed_seconds),
            record.evolved.tool_call_count,
            record.evolved.verified_artifact_count,
            record.evolved.skill_activation_count,
        )
    }
}

fn metrics_json(
    human_intervention_count: u32,
    retry_count: u32,
    elapsed_seconds: Option<u64>,
    tool_call_count: u32,
    verified_artifact_count: u32,
    skill_activation_count: u32,
) -> Value {
    json!({
        "human_intervention_count": human_intervention_count,
        "retry_count": retry_count,
        "elapsed_seconds": elapsed_seconds,
        "tool_call_count": tool_call_count,
        "verified_artifact_count": verified_artifact_count,
        "skill_activation_count": skill_activation_count,
    })
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn sanitize_opt(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|value| sanitize(value))
}

fn sanitize(value: &str) -> String {
    if looks_sensitive(value) {
        return REDACTED.into();
    }

    value.chars().take(MAX_RENDERED_FIELD_LEN).collect()
}

fn looks_sensitive(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("secret")
        || lowered.contains("raw_prompt")
        || lowered.contains("provider-payload")
        || lowered.contains("provider_payload")
        || lowered.contains("credential")
        || lowered.contains("full skill body")
}
