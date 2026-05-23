use crate::evaluation::{
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs,
    SelfEvolutionRunMetrics, SelfEvolutionWhiteBoxEvidence,
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
