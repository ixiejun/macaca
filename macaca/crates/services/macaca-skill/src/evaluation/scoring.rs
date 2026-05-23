use super::{SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionScore};

/// Deterministic Strategy for evaluating self-evolution evidence.
///
/// The scorer is intentionally pure and provider-neutral. It does not inspect
/// task text, application names, provider names, model names, or skill bodies.
/// This keeps the pass/fail decision replayable from sanitized refs and counts
/// stored in the governance/evaluation record.
pub struct SelfEvolutionScoringPolicy;

impl SelfEvolutionScoringPolicy {
    /// Score one evaluation record against the white-box and black-box gates.
    ///
    /// The method logs the final state and bounded reason count so operators can
    /// audit scoring decisions without exposing raw task output. Individual
    /// reason codes are fixed policy labels, not user- or provider-supplied
    /// strings.
    pub fn score(record: &SelfEvolutionEvaluationRecord) -> SelfEvolutionScore {
        let mut hard_failures = Vec::new();
        let mut inconclusive_reasons = Vec::new();

        Self::collect_white_box_failures(record, &mut hard_failures);
        Self::collect_black_box_failures(record, &mut hard_failures, &mut inconclusive_reasons);

        let score = if !hard_failures.is_empty() {
            SelfEvolutionScore {
                lifecycle: SelfEvolutionEvaluationLifecycle::Failed,
                passed: false,
                reason_codes: hard_failures,
            }
        } else if !inconclusive_reasons.is_empty() {
            SelfEvolutionScore {
                lifecycle: SelfEvolutionEvaluationLifecycle::Inconclusive,
                passed: false,
                reason_codes: inconclusive_reasons,
            }
        } else {
            SelfEvolutionScore {
                lifecycle: SelfEvolutionEvaluationLifecycle::Passed,
                passed: true,
                reason_codes: Vec::new(),
            }
        };

        tracing::info!(
            evaluation_id = %record.evaluation_id,
            trace_id = %record.trace_id,
            task_family_id = %record.task_family_id,
            result_state = ?score.lifecycle,
            passed = score.passed,
            reason_count = score.reason_codes.len(),
            "scored self-evolution evaluation"
        );

        score
    }

    fn collect_white_box_failures(
        record: &SelfEvolutionEvaluationRecord,
        failures: &mut Vec<String>,
    ) {
        let white_box = &record.white_box;

        Self::require_ref(
            &white_box.verified_task_completion_ref,
            "missing_verified_task_completion",
            failures,
        );
        Self::require_ref(
            &white_box.experience_candidate_ref,
            "missing_experience_candidate",
            failures,
        );
        Self::require_ref(
            &white_box.classification_ref,
            "missing_classification",
            failures,
        );
        Self::require_ref(&white_box.proposal_id, "missing_proposal", failures);
        Self::require_ref(&white_box.curation_run_id, "missing_curation_run", failures);
        Self::require_ref(
            &white_box.promotion_or_apply_ref,
            "missing_promotion_or_apply",
            failures,
        );
        Self::require_ref(
            &white_box.active_catalog_snapshot_ref,
            "missing_active_catalog_snapshot",
            failures,
        );
        Self::require_ref(
            &white_box.later_skill_activation_ref,
            "missing_skill_activation",
            failures,
        );

        if white_box.audit_event_ids.is_empty() {
            failures.push("missing_audit_events".into());
        }
    }

    fn collect_black_box_failures(
        record: &SelfEvolutionEvaluationRecord,
        failures: &mut Vec<String>,
        inconclusive: &mut Vec<String>,
    ) {
        if record.baseline.completion_success && !record.evolved.completion_success {
            failures.push("completion_regression".into());
        }

        if record.evolved.verified_artifact_count < record.baseline.verified_artifact_count {
            failures.push("artifact_regression".into());
        }

        if record.evolved.policy_violation_count > record.baseline.policy_violation_count
            || record.evolved.policy_violation_count > 0
        {
            failures.push("policy_regression".into());
        }

        if record.evolved.regression_count > record.baseline.regression_count
            || record.evolved.regression_count > 0
        {
            failures.push("behavior_regression".into());
        }

        if record.evolved.skill_activation_count == 0 {
            failures.push("missing_skill_activation".into());
        }

        if !Self::has_efficiency_gain(record) {
            inconclusive.push("no_efficiency_gain".into());
        }
    }

    fn require_ref(value: &Option<String>, reason: &str, failures: &mut Vec<String>) {
        if value.as_deref().unwrap_or_default().trim().is_empty() {
            failures.push(reason.into());
        }
    }

    fn has_efficiency_gain(record: &SelfEvolutionEvaluationRecord) -> bool {
        record.evolved.human_intervention_count < record.baseline.human_intervention_count
            || record.evolved.retry_count < record.baseline.retry_count
            || record.evolved.tool_call_count < record.baseline.tool_call_count
            || record.evolved.verified_artifact_count > record.baseline.verified_artifact_count
            || Self::elapsed_improved(record)
    }

    fn elapsed_improved(record: &SelfEvolutionEvaluationRecord) -> bool {
        match (
            record.baseline.elapsed_seconds,
            record.evolved.elapsed_seconds,
        ) {
            (Some(baseline), Some(evolved)) => evolved < baseline,
            _ => false,
        }
    }
}
