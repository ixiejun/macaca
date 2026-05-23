use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SelfEvolutionEvaluationCheckpointKind, SkillEvaluationReportCommand,
    SkillEvaluationReportResult, SkillEvaluationScoreCommand, SkillEvaluationScoreResult,
    SKILL_EVALUATION_REPORT_COMMAND, SKILL_EVALUATION_SCORE_COMMAND,
};

use crate::skill_self_evolution_evaluation_harness_fixture::{
    append_checkpoint, checkpoint, command, initial_record, produce_service_owned_evidence, scope,
};
use crate::SkillSystemServiceProvider;

#[tokio::test]
async fn service_owned_self_evolution_chain_passes_white_box_and_black_box_gates() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-service-owned-self-evolution");
    let evidence = produce_service_owned_evidence(&provider, &trace).await;
    let mut record = initial_record();

    let mut completion = checkpoint(SelfEvolutionEvaluationCheckpointKind::VerifiedTaskCompletion);
    completion.evidence_ref = evidence.proposed.proposal.evidence_ids.first().cloned();
    let mut candidate = checkpoint(SelfEvolutionEvaluationCheckpointKind::ExperienceCandidate);
    candidate.evidence_ref = Some(format!(
        "proposal://{}/candidate",
        evidence.proposed.proposal.proposal_id
    ));
    let mut classification = checkpoint(SelfEvolutionEvaluationCheckpointKind::Classification);
    classification.evidence_ref = Some(format!(
        "proposal://{}/classification/{:?}",
        evidence.proposed.proposal.proposal_id, evidence.proposed.proposal.classification
    ));
    let mut proposal = checkpoint(SelfEvolutionEvaluationCheckpointKind::Proposal);
    proposal.proposal_id = Some(evidence.proposed.proposal.proposal_id.clone());
    let mut curation = checkpoint(SelfEvolutionEvaluationCheckpointKind::Curation);
    curation.curation_run_id = Some(evidence.curation.run.run_id.clone());
    curation.audit_event_ids = vec!["audit://curation/dry-run".into()];
    let mut promotion = checkpoint(SelfEvolutionEvaluationCheckpointKind::PromotionOrApply);
    promotion.evidence_ref = evidence.promoted.evidence_ids.first().cloned();
    promotion.policy_decision_id = evidence.promoted.policy_decision_refs.first().cloned();
    promotion.audit_event_ids = vec!["audit://promotion/evaluation-harness".into()];
    let mut catalog = checkpoint(SelfEvolutionEvaluationCheckpointKind::CatalogVisibility);
    catalog.evidence_ref = Some(evidence.snapshot.snapshot.snapshot_ref.clone());
    catalog.before_snapshot_ref = Some(evidence.snapshot.snapshot.snapshot_ref.clone());
    catalog.after_snapshot_ref = Some(evidence.snapshot.snapshot.snapshot_ref.clone());
    let mut activation = checkpoint(SelfEvolutionEvaluationCheckpointKind::SkillActivation);
    activation.evidence_ref = Some(evidence.activation_ref.clone());

    for checkpoint in [
        completion,
        candidate,
        classification,
        proposal,
        curation,
        promotion,
        catalog,
        activation,
    ] {
        record = append_checkpoint(&provider, &trace, record, checkpoint).await;
    }

    let score = score_record(&provider, &trace, record.clone()).await;
    assert!(score.score.passed);

    let report = provider
        .call(command(
            SKILL_EVALUATION_REPORT_COMMAND,
            SkillEvaluationReportCommand {
                trace: trace.clone(),
                scope: scope(),
                record: record.clone(),
                score: score.score.clone(),
                include_markdown: true,
            },
            trace,
        ))
        .await
        .expect("report should render sanitized service-owned refs");
    let report: SkillEvaluationReportResult =
        serde_json::from_value(report.output).expect("report result should decode");
    assert!(report.score.passed);
    assert!(report
        .markdown_report
        .as_deref()
        .unwrap_or_default()
        .contains("Self-Evolution Evaluation"));

    // Black-box no-activation failure uses the same service-owned record, then
    // removes only the evolved activation evidence to verify the gate rejects
    // unproven self-evolution instead of claiming completion.
    let mut no_activation = record;
    no_activation.white_box.later_skill_activation_ref = None;
    no_activation.evolved.skill_activation_count = 0;
    let failed_trace = TraceContext::new("trace-service-owned-self-evolution-no-activation");
    let failed = score_record(&provider, &failed_trace, no_activation).await;
    assert!(!failed.score.passed);
    assert!(failed
        .score
        .reason_codes
        .contains(&"missing_skill_activation".into()));
}

async fn score_record(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    record: macaca_skill::SelfEvolutionEvaluationRecord,
) -> SkillEvaluationScoreResult {
    let result = provider
        .call(command(
            SKILL_EVALUATION_SCORE_COMMAND,
            SkillEvaluationScoreCommand {
                trace: trace.clone(),
                scope: scope(),
                record,
            },
            trace.clone(),
        ))
        .await
        .expect("evaluation scoring should stay behind Skill service");
    serde_json::from_value(result.output).expect("score result should decode")
}
