use crate::evaluation::{
    SelfEvolutionEvaluationCheckpoint, SelfEvolutionEvaluationCheckpointKind,
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportBuilder,
    SelfEvolutionReportRefs, SelfEvolutionRunMetrics, SelfEvolutionScoringPolicy,
    SelfEvolutionWhiteBoxEvidence,
};
use crate::service_contract::{
    SkillEvaluationCheckpointAppendCommand, SkillEvaluationCheckpointAppendResult,
    SkillEvaluationReportCommand, SkillEvaluationReportResult, SkillEvaluationScoreCommand,
    SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND, SKILL_EVALUATION_REPORT_COMMAND,
    SKILL_EVALUATION_SCORE_COMMAND,
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
fn self_evolution_evaluation_scoring_fails_each_required_white_box_checkpoint() {
    let cases: Vec<(&str, Box<dyn Fn(&mut SelfEvolutionEvaluationRecord)>)> = vec![
        (
            "missing_proposal",
            Box::new(|record| record.white_box.proposal_id = None),
        ),
        (
            "missing_curation_run",
            Box::new(|record| record.white_box.curation_run_id = None),
        ),
        (
            "missing_promotion_or_apply",
            Box::new(|record| record.white_box.promotion_or_apply_ref = None),
        ),
        (
            "missing_active_catalog_snapshot",
            Box::new(|record| record.white_box.active_catalog_snapshot_ref = None),
        ),
    ];

    for (reason, mutate) in cases {
        let mut record = complete_evaluation_record();
        mutate(&mut record);

        let score = SelfEvolutionScoringPolicy::score(&record);

        assert!(!score.passed, "{reason} should not pass");
        assert_eq!(score.lifecycle, SelfEvolutionEvaluationLifecycle::Failed);
        assert!(
            score.reason_codes.contains(&reason.to_string()),
            "{reason} should be reported as a bounded reason"
        );
    }
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

#[test]
fn self_evolution_evaluation_report_renders_sanitized_json_summary() {
    let mut record = complete_evaluation_record();
    record.trace_id = "trace-RAW_PROMPT_SECRET".into();
    record.white_box.rollback_ref = Some("store://rollback/SECRET-provider-payload".into());
    let score = SelfEvolutionScoringPolicy::score(&record);

    let report = SelfEvolutionReportBuilder::json_summary(&record, &score);
    let encoded = serde_json::to_string(&report).expect("report should serialize");

    assert!(encoded.contains("eval-score"));
    assert!(encoded.contains("spec_change_loop"));
    assert!(encoded.contains("\"human_intervention_count\":1"));
    assert!(!encoded.contains("RAW_PROMPT_SECRET"));
    assert!(!encoded.contains("SECRET-provider-payload"));
    assert!(!encoded.contains("full skill body"));
}

#[test]
fn self_evolution_evaluation_report_renders_markdown_summary_without_raw_payloads() {
    let mut record = complete_evaluation_record();
    record.white_box.later_skill_activation_ref = None;
    record.evolved.skill_activation_count = 0;
    let score = SelfEvolutionScoringPolicy::score(&record);

    let report = SelfEvolutionReportBuilder::markdown_summary(&record, &score);

    assert!(report.contains("# Self-Evolution Evaluation"));
    assert!(report.contains("eval-score"));
    assert!(report.contains("missing_skill_activation"));
    assert!(report.contains("skill_activation_count"));
    assert!(!report.contains("raw provider payload"));
    assert!(!report.contains("full skill body"));
}

#[test]
fn self_evolution_evaluation_report_includes_memento_refs_without_audit_payloads() {
    let record = complete_evaluation_record();
    let score = SelfEvolutionScoringPolicy::score(&record);

    let markdown = SelfEvolutionReportBuilder::markdown_summary(&record, &score);
    let json = SelfEvolutionReportBuilder::json_summary(&record, &score);

    assert!(markdown.contains("before_snapshot_ref: before"));
    assert!(markdown.contains("after_snapshot_ref: after"));
    assert!(markdown.contains("rollback_ref: rollback"));
    assert_eq!(json["white_box"]["before_snapshot_ref"], "before");
    assert_eq!(json["white_box"]["after_snapshot_ref"], "after");
    assert_eq!(json["white_box"]["rollback_ref"], "rollback");
    assert!(!markdown.contains("audit"));
}

#[test]
fn self_evolution_evaluation_checkpoint_append_maps_refs_without_raw_payloads() {
    let mut record = complete_evaluation_record();
    record.white_box = SelfEvolutionWhiteBoxEvidence::default();

    let command = SkillEvaluationCheckpointAppendCommand {
        trace: macaca_proto::TraceContext::new("trace-evaluation-checkpoint"),
        scope: crate::SkillServiceScope::default(),
        record,
        checkpoint: SelfEvolutionEvaluationCheckpoint {
            kind: SelfEvolutionEvaluationCheckpointKind::Proposal,
            evidence_ref: Some("proposal-ref-raw_prompt-secret".into()),
            proposal_id: Some("proposal-2".into()),
            curation_run_id: None,
            policy_decision_id: Some("policy-2".into()),
            audit_event_ids: vec!["audit-2".into(), "audit-3".into()],
            before_snapshot_ref: Some("before-2".into()),
            after_snapshot_ref: Some("after-2".into()),
            rollback_ref: Some("rollback-2".into()),
        },
    };

    let result = SkillEvaluationCheckpointAppendResult::from_command(&command);

    assert_eq!(
        result.record.white_box.proposal_id.as_deref(),
        Some("proposal-2")
    );
    assert_eq!(
        result.record.white_box.before_snapshot_ref.as_deref(),
        Some("before-2")
    );
    assert_eq!(
        result.record.white_box.after_snapshot_ref.as_deref(),
        Some("after-2")
    );
    assert_eq!(
        result.record.white_box.rollback_ref.as_deref(),
        Some("rollback-2")
    );
    assert_eq!(result.checkpoint_count, 1);
    assert_eq!(result.audit_event_count, 2);
    assert_ne!(
        result.record.white_box.proposal_id.as_deref(),
        Some("proposal-ref-raw_prompt-secret")
    );
}

#[test]
fn self_evolution_evaluation_service_contract_scores_and_reports_generically() {
    let record = complete_evaluation_record();
    let score = SelfEvolutionScoringPolicy::score(&record);
    let score_command = SkillEvaluationScoreCommand {
        trace: macaca_proto::TraceContext::new("trace-evaluation-score"),
        scope: crate::SkillServiceScope::default(),
        record: record.clone(),
    };
    let report_command = SkillEvaluationReportCommand {
        trace: macaca_proto::TraceContext::new("trace-evaluation-report"),
        scope: crate::SkillServiceScope::default(),
        record,
        score: score.clone(),
        include_markdown: true,
    };
    let report_result = SkillEvaluationReportResult {
        score,
        json_report: SelfEvolutionReportBuilder::json_summary(
            &report_command.record,
            &report_command.score,
        ),
        markdown_report: Some(SelfEvolutionReportBuilder::markdown_summary(
            &report_command.record,
            &report_command.score,
        )),
    };

    assert_eq!(SKILL_EVALUATION_SCORE_COMMAND, "skill.evaluation.score");
    assert_eq!(SKILL_EVALUATION_REPORT_COMMAND, "skill.evaluation.report");
    assert_eq!(
        SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND,
        "skill.evaluation.checkpoint.append"
    );
    assert_eq!(score_command.record.evaluation_id, "eval-score");
    assert!(report_result.score.passed);
    assert!(report_result
        .markdown_report
        .as_deref()
        .unwrap_or_default()
        .contains("Self-Evolution Evaluation"));
}
