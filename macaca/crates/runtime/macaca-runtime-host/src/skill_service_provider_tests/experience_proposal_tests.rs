//! Contract tests for experience proposal creation and snapshot listing.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction,
    SkillExperienceCandidateDestination, SkillExperienceProposalCommand,
    SkillExperienceProposalResult, SkillExperienceProposalSnapshotCommand,
    SkillExperienceProposalSnapshotResult, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillServiceScope, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{reusable_experience_candidate, traced_command};

#[tokio::test]
async fn skill_experience_proposal_creates_draft_without_mutating_governance_records() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-proposal");
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate: reusable_experience_candidate(vec!["artifact-proof-1".into()]),
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("verified reusable task evidence should create a proposal");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert!(!proposal.mutated);
    assert_eq!(
        proposal.proposal.recommended_action,
        SkillEvolutionProposalAction::CreateDraft
    );
    assert_eq!(
        proposal.proposal.classification,
        SkillEvolutionCandidateClassification::ReusableProcedure
    );
    assert_eq!(
        proposal.proposal.destination,
        SkillExperienceCandidateDestination::NewSkillDraft
    );
    assert_eq!(
        proposal.proposal.trace_digest.as_deref(),
        Some("trace-digest://task/verified-1")
    );
    assert_eq!(
        proposal.proposal.memory_digest_refs,
        vec!["memory://digest/task-verified-1"]
    );
    assert_eq!(proposal.proposal.evidence_ids, vec!["artifact-proof-1"]);
    assert!(proposal.proposal.proposal_id.starts_with("skill-exp-"));

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("governance snapshot should remain available");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("snapshot result should decode");
    assert!(
        snapshot.records.is_empty(),
        "draft proposals must not become active governance records"
    );
}

#[tokio::test]
async fn skill_experience_snapshot_lists_draft_proposals_without_mutation() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-snapshot");

    for evidence_id in ["artifact-proof-2", "artifact-proof-1"] {
        let command = SkillExperienceProposalCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            candidate: reusable_experience_candidate(vec![evidence_id.into()]),
        };
        provider
            .call(traced_command(
                SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
                command,
                trace.clone(),
            ))
            .await
            .expect("verified task evidence should create a proposal");
    }

    let snapshot_command = SkillExperienceProposalSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_discarded: false,
    };
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            snapshot_command,
            trace,
        ))
        .await
        .expect("proposal snapshot should be available");
    let snapshot: SkillExperienceProposalSnapshotResult =
        serde_json::from_value(result.output).expect("proposal snapshot should decode");

    assert_eq!(snapshot.proposals.len(), 2);
    assert!(!snapshot.mutated);
    assert!(snapshot.proposals[0].proposal_id <= snapshot.proposals[1].proposal_id);
    assert_eq!(
        snapshot.proposals[0].trace_id,
        "trace-skill-experience-snapshot"
    );
    assert!(snapshot
        .proposals
        .iter()
        .all(|proposal| proposal.target_skill_name.as_deref()
            == Some("skill-experience-maintenance")));
}
