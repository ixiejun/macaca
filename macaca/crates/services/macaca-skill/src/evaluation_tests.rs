use crate::evaluation::{
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs,
    SelfEvolutionRunMetrics, SelfEvolutionScoringPolicy, SelfEvolutionWhiteBoxEvidence,
};

#[test]
fn self_evolution_evaluation_model_keeps_traceable_refs() {
    let record = SelfEvolutionEvaluationRecord {
        evaluation_id: "eval-1".into(),
        trace_id: "trace-1".into(),
        task_family_id: "spec_change_loop".into(),
        lifecycle: SelfEvolutionEvaluationLifecycle::Prepared,
        white_box: SelfEvolutionWhiteBoxEvidence {
            verified_task_completion_ref: Some("task-evidence-1".into()),
            experience_candidate_ref: Some("candidate-1".into()),
            classification_ref: Some("classification-1".into()),
            proposal_id: Some("proposal-1".into()),
            curation_run_id: Some("curation-1".into()),
            promotion_or_apply_ref: Some("promotion-1".into()),
            active_catalog_snapshot_ref: Some("catalog-after-1".into()),
            later_skill_activation_ref: Some("activation-1".into()),
            policy_decision_id: Some("policy-1".into()),
            audit_event_ids: vec!["audit-1".into()],
            before_snapshot_ref: Some("catalog-before-1".into()),
            after_snapshot_ref: Some("catalog-after-1".into()),
            rollback_ref: Some("rollback-1".into()),
        },
        baseline: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 3,
            elapsed_seconds: Some(120),
            tool_call_count: 20,
            retry_count: 2,
            policy_violation_count: 0,
            skill_activation_count: 0,
            accepted_proposal_count: 0,
            total_proposal_count: 1,
            reuse_score: 0,
            regression_count: 0,
        },
        evolved: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 1,
            elapsed_seconds: Some(90),
            tool_call_count: 16,
            retry_count: 1,
            policy_violation_count: 0,
            skill_activation_count: 1,
            accepted_proposal_count: 1,
            total_proposal_count: 1,
            reuse_score: 1,
            regression_count: 0,
        },
        report_refs: SelfEvolutionReportRefs {
            json_report_ref: Some("report.json".into()),
            markdown_report_ref: Some("REPORT.md".into()),
        },
    };

    assert_eq!(record.lifecycle, SelfEvolutionEvaluationLifecycle::Prepared);
    assert_eq!(record.white_box.audit_event_ids, vec!["audit-1"]);
    assert_eq!(record.evolved.skill_activation_count, 1);
}

fn complete_evaluation_record() -> SelfEvolutionEvaluationRecord {
    SelfEvolutionEvaluationRecord {
        evaluation_id: "eval-score".into(),
        trace_id: "trace-score".into(),
        task_family_id: "spec_change_loop".into(),
        lifecycle: SelfEvolutionEvaluationLifecycle::EvolvedRecorded,
        white_box: SelfEvolutionWhiteBoxEvidence {
            verified_task_completion_ref: Some("task-evidence".into()),
            experience_candidate_ref: Some("candidate".into()),
            classification_ref: Some("classification".into()),
            proposal_id: Some("proposal".into()),
            curation_run_id: Some("curation".into()),
            promotion_or_apply_ref: Some("promotion".into()),
            active_catalog_snapshot_ref: Some("catalog".into()),
            later_skill_activation_ref: Some("activation".into()),
            policy_decision_id: Some("policy".into()),
            audit_event_ids: vec!["audit".into()],
            before_snapshot_ref: Some("before".into()),
            after_snapshot_ref: Some("after".into()),
            rollback_ref: Some("rollback".into()),
        },
        baseline: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 3,
            elapsed_seconds: Some(120),
            tool_call_count: 20,
            retry_count: 2,
            policy_violation_count: 0,
            skill_activation_count: 0,
            accepted_proposal_count: 0,
            total_proposal_count: 1,
            reuse_score: 0,
            regression_count: 0,
        },
        evolved: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 1,
            elapsed_seconds: Some(90),
            tool_call_count: 16,
            retry_count: 1,
            policy_violation_count: 0,
            skill_activation_count: 1,
            accepted_proposal_count: 1,
            total_proposal_count: 1,
            reuse_score: 1,
            regression_count: 0,
        },
        report_refs: SelfEvolutionReportRefs::default(),
    }
}

#[test]
fn self_evolution_evaluation_scoring_passes_complete_chain_with_improvement() {
    let score = SelfEvolutionScoringPolicy::score(&complete_evaluation_record());

    assert!(score.passed);
    assert_eq!(score.lifecycle, SelfEvolutionEvaluationLifecycle::Passed);
    assert!(score.reason_codes.is_empty());
}

#[test]
fn self_evolution_evaluation_scoring_fails_without_later_activation() {
    let mut record = complete_evaluation_record();
    record.white_box.later_skill_activation_ref = None;
    record.evolved.skill_activation_count = 0;

    let score = SelfEvolutionScoringPolicy::score(&record);

    assert!(!score.passed);
    assert_eq!(score.lifecycle, SelfEvolutionEvaluationLifecycle::Failed);
    assert!(score
        .reason_codes
        .contains(&"missing_skill_activation".into()));
}

#[test]
fn self_evolution_evaluation_scoring_fails_completion_regression() {
    let mut record = complete_evaluation_record();
    record.evolved.completion_success = false;

    let score = SelfEvolutionScoringPolicy::score(&record);

    assert!(!score.passed);
    assert_eq!(score.lifecycle, SelfEvolutionEvaluationLifecycle::Failed);
    assert!(score.reason_codes.contains(&"completion_regression".into()));
}

#[test]
fn self_evolution_evaluation_scoring_fails_policy_regression() {
    let mut record = complete_evaluation_record();
    record.evolved.policy_violation_count = 1;

    let score = SelfEvolutionScoringPolicy::score(&record);

    assert!(!score.passed);
    assert_eq!(score.lifecycle, SelfEvolutionEvaluationLifecycle::Failed);
    assert!(score.reason_codes.contains(&"policy_regression".into()));
}

#[test]
fn self_evolution_evaluation_scoring_is_inconclusive_without_efficiency_gain() {
    let mut record = complete_evaluation_record();
    record.evolved.human_intervention_count = record.baseline.human_intervention_count;
    record.evolved.elapsed_seconds = record.baseline.elapsed_seconds;
    record.evolved.tool_call_count = record.baseline.tool_call_count;
    record.evolved.retry_count = record.baseline.retry_count;
    record.evolved.verified_artifact_count = record.baseline.verified_artifact_count;

    let score = SelfEvolutionScoringPolicy::score(&record);

    assert!(!score.passed);
    assert_eq!(
        score.lifecycle,
        SelfEvolutionEvaluationLifecycle::Inconclusive
    );
    assert!(score.reason_codes.contains(&"no_efficiency_gain".into()));
}
