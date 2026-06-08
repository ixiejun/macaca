//! Contract tests for the SDK Skill client facade.
//!
//! Tests are flattened at the module root (no nested `mod tests`) so escape-hatch
//! gates can scan production modules without false positives from test-only literals.
//! Validates Null Object behavior: mutating lifecycle commands must fail while
//! read-only snapshots remain safe to query.

use std::collections::BTreeMap;

use macaca_proto::TraceContext;
use macaca_skill::{
    SelfEvolutionEvaluationCheckpoint, SelfEvolutionEvaluationCheckpointKind,
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs,
    SelfEvolutionRunMetrics, SelfEvolutionScore, SelfEvolutionWhiteBoxEvidence,
    SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationSnapshotCommand,
    SkillEvaluationCheckpointAppendCommand, SkillEvaluationReportCommand,
    SkillEvaluationScoreCommand, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionRejectDraftCommand, SkillProposalProcessingRunCommand,
    SkillProposalProcessingSnapshotCommand, SkillServicePolicyHints, SkillServiceScope,
};

use super::{SystemSkillClient, UnavailableSystemSkillClient};

fn policy_hints() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.evolution.promote".into()],
        entitlement_ready: Some(false),
        package_ready: Some(false),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn unavailable_skill_client_rejects_proposal_lifecycle_side_effects() {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("trace-sdk-skill-proposal-unavailable");
    let promote = SkillEvolutionPromoteDraftCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        proposal_id: "proposal-1".into(),
        reason: "approval cannot run without the Skill service".into(),
        evidence_ids: vec!["evidence://approval/1".into()],
        policy_decision_refs: vec!["policy://decision/1".into()],
        policy: policy_hints(),
    };
    let reject = SkillEvolutionRejectDraftCommand {
        trace,
        scope: SkillServiceScope::default(),
        proposal_id: "proposal-1".into(),
        rationale: "rejection cannot run without the Skill service".into(),
        evidence_ids: vec!["evidence://reject/1".into()],
        policy_decision_refs: vec!["policy://decision/2".into()],
        policy: policy_hints(),
    };

    assert!(client.promote_skill_draft(promote).await.is_err());
    assert!(client.reject_skill_draft(reject).await.is_err());
}

fn evaluation_record() -> SelfEvolutionEvaluationRecord {
    SelfEvolutionEvaluationRecord {
        evaluation_id: "eval-sdk".into(),
        trace_id: "trace-sdk".into(),
        task_family_id: "bug_trace_loop".into(),
        lifecycle: SelfEvolutionEvaluationLifecycle::Prepared,
        white_box: SelfEvolutionWhiteBoxEvidence::default(),
        baseline: SelfEvolutionRunMetrics::default(),
        evolved: SelfEvolutionRunMetrics::default(),
        report_refs: SelfEvolutionReportRefs::default(),
    }
}

#[tokio::test]
async fn unavailable_skill_client_rejects_self_evolution_evaluation_commands() {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("trace-sdk-skill-evaluation-unavailable");
    let score = SelfEvolutionScore {
        lifecycle: SelfEvolutionEvaluationLifecycle::Inconclusive,
        passed: false,
        reason_codes: vec!["missing_evidence".into()],
    };
    let score_command = SkillEvaluationScoreCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record: evaluation_record(),
    };
    let checkpoint_command = SkillEvaluationCheckpointAppendCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record: evaluation_record(),
        checkpoint: SelfEvolutionEvaluationCheckpoint {
            kind: SelfEvolutionEvaluationCheckpointKind::VerifiedTaskCompletion,
            evidence_ref: Some("evidence://task/1".into()),
            ..Default::default()
        },
    };
    let report_command = SkillEvaluationReportCommand {
        trace,
        scope: SkillServiceScope::default(),
        record: evaluation_record(),
        score,
        include_markdown: true,
    };

    assert!(client
        .append_self_evolution_checkpoint(checkpoint_command)
        .await
        .is_err());
    assert!(client.evaluate_self_evolution(score_command).await.is_err());
    assert!(client
        .self_evolution_evaluation_report(report_command)
        .await
        .is_err());
}

#[tokio::test]
async fn unavailable_skill_client_rejects_proposal_processing_apply_but_allows_snapshot() {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("trace-sdk-skill-processing-unavailable");
    let run = SkillProposalProcessingRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        dry_run: false,
        min_ready_score: 40,
        evidence_ids: vec!["evidence://processing/apply".into()],
        policy_decision_refs: vec!["policy://processing/approved".into()],
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    let snapshot = SkillProposalProcessingSnapshotCommand {
        trace,
        scope: SkillServiceScope::default(),
    };

    assert!(client.process_skill_proposals(run).await.is_err());
    let snapshot = client
        .skill_proposal_processing_snapshot(snapshot)
        .await
        .expect("unavailable client should expose an empty read-only snapshot");
    assert_eq!(snapshot.records.len(), 0);
    assert!(!snapshot.mutated);
}

#[tokio::test]
async fn unavailable_skill_client_rejects_autonomous_materialization_apply_but_allows_snapshot()
{
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("trace-sdk-skill-operator-unavailable");
    let run = SkillAutonomousMaterializationRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        dry_run: false,
        batch_limit: 1,
        min_ready_score: 40,
        package_collection_root: "/tmp/unused-operator-root".into(),
        ownership: macaca_skill::SkillPackageOwnershipClass::AgentPrivate,
        reason: "approved autonomous materialization run".into(),
        evidence_ids: vec!["evidence://operator/apply".into()],
        policy_decision_refs: vec!["policy://operator/approved".into()],
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    let snapshot = SkillAutonomousMaterializationSnapshotCommand {
        trace,
        scope: SkillServiceScope::default(),
        limit: 10,
    };

    assert!(client.run_autonomous_materialization(run).await.is_err());
    let snapshot = client
        .autonomous_materialization_snapshot(snapshot)
        .await
        .expect("unavailable client should expose an empty operator snapshot");
    assert_eq!(snapshot.recent_runs.len(), 0);
    assert!(snapshot.last_run_ref.is_none());
}
