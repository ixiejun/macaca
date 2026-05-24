use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceEvidenceGateStatus,
    SkillExperienceProposalCommand, SkillProposalProcessingRunCommand,
    SkillProposalProcessingRunResult, SkillProposalProcessingSnapshotCommand,
    SkillProposalProcessingSnapshotResult, SkillProposalProcessingState, SkillServicePolicyHints,
    SkillServiceScope, SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
    SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
};

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test payload should serialize"),
        trace,
    )
}

fn candidate(task_id: &str, evidence_id: &str) -> SkillExperienceCandidate {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "evidence_ref.artifact_0".into(),
        format!("tool:file_write:{task_id}"),
    );
    SkillExperienceCandidate {
        task_id: task_id.into(),
        session_id: Some("session-processing".into()),
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "Verified terminal task completion observed through service.agent_execution; output_chars=10, artifact_count=0, token_total=unavailable.".into(),
        trace_digest: Some(format!("trace-digest://{task_id}")),
        memory_digest_refs: Vec::new(),
        reusable_procedure: "Review bounded service evidence, extract reusable steps, and keep promotion behind governed policy.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some("processing-candidate".into()),
        evidence_ids: vec![evidence_id.into()],
        metadata,
    }
}

fn policy() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.proposal_processing".into()],
        entitlement_ready: Some(true),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

async fn create_proposal(provider: &SkillSystemServiceProvider, task_id: &str, evidence_id: &str) {
    let trace = TraceContext::new(format!("trace-{task_id}"));
    provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: candidate(task_id, evidence_id),
            },
            trace,
        ))
        .await
        .expect("verified task evidence should create a proposal");
}

#[tokio::test]
async fn skill_proposal_processing_apply_marks_one_ready_and_suppresses_duplicate() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(&provider, "task-processing-1", "evidence://task/1").await;
    create_proposal(&provider, "task-processing-2", "evidence://task/2").await;
    let trace = TraceContext::new("trace-processing-apply");

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
            SkillProposalProcessingRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                min_ready_score: 40,
                evidence_ids: vec!["evidence://processing/apply".into()],
                policy_decision_refs: vec!["policy://processing/approved".into()],
                audit_event_ids: vec!["audit://processing/apply".into()],
                policy: policy(),
            },
            trace.clone(),
        ))
        .await
        .expect("proposal processing apply should be accepted");
    let processed: SkillProposalProcessingRunResult =
        serde_json::from_value(result.output).expect("processing result should decode");

    assert!(processed.mutated);
    assert_eq!(processed.records.len(), 2);
    assert_eq!(processed.state_counts.ready_for_materialization, 1);
    assert_eq!(processed.state_counts.suppressed_duplicate, 1);
    assert!(processed
        .records
        .iter()
        .any(|record| record.state == SkillProposalProcessingState::ReadyForMaterialization));
    assert!(processed
        .records
        .iter()
        .any(|record| record.state == SkillProposalProcessingState::SuppressedDuplicate));

    let snapshot = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
            SkillProposalProcessingSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace,
        ))
        .await
        .expect("processing snapshot should be readable");
    let snapshot: SkillProposalProcessingSnapshotResult =
        serde_json::from_value(snapshot.output).expect("processing snapshot should decode");
    assert_eq!(snapshot.records.len(), 2);
    assert_eq!(snapshot.waiting_proposal_count, 0);
    assert_eq!(snapshot.state_counts.ready_for_materialization, 1);
}

#[tokio::test]
async fn skill_proposal_processing_apply_rejects_missing_policy_refs_without_mutation() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-processing-denied",
        "evidence://task/denied",
    )
    .await;
    let trace = TraceContext::new("trace-processing-denied");

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
            SkillProposalProcessingRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                min_ready_score: 40,
                evidence_ids: vec!["evidence://processing/apply".into()],
                policy_decision_refs: Vec::new(),
                audit_event_ids: Vec::new(),
                policy: policy(),
            },
            trace.clone(),
        ))
        .await
        .expect_err("apply processing requires policy refs");

    assert!(err.to_string().contains("policy decision refs"));
}

#[tokio::test]
async fn skill_proposal_processing_snapshot_reports_unprocessed_proposals_as_queued() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-processing-queued",
        "evidence://task/queued",
    )
    .await;
    let trace = TraceContext::new("trace-processing-queued");

    let snapshot = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
            SkillProposalProcessingSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace,
        ))
        .await
        .expect("processing snapshot should expose queued proposals");
    let snapshot: SkillProposalProcessingSnapshotResult =
        serde_json::from_value(snapshot.output).expect("processing snapshot should decode");

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.waiting_proposal_count, 1);
    assert_eq!(snapshot.state_counts.queued, 1);
    assert_eq!(
        snapshot.records[0].state,
        SkillProposalProcessingState::Queued
    );
    assert!(snapshot.records[0]
        .quality
        .reasons
        .iter()
        .any(|reason| reason == "proposal_reports_artifact_refs"));
}
