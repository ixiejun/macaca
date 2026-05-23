use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionPromoteDraftResult, SkillEvolutionProposalAction,
    SkillEvolutionProposalLifecycle, SkillEvolutionProposePatchCommand,
    SkillEvolutionProposePatchResult, SkillEvolutionRejectDraftCommand,
    SkillEvolutionRejectDraftResult, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalSnapshotCommand,
    SkillExperienceProposalSnapshotResult, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillLifecycleState, SkillServicePolicyHints, SkillServiceScope,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
    SKILL_EVOLUTION_REJECT_DRAFT_COMMAND, SKILL_EVOLUTION_SNAPSHOT_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test command payload must serialize"),
        trace,
    )
}

fn patch_candidate(evidence_ids: Vec<String>) -> SkillExperienceCandidate {
    SkillExperienceCandidate {
        task_id: "task-patch-1".into(),
        session_id: Some("session-patch-1".into()),
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "A verified task found a reusable improvement for an existing skill."
            .into(),
        trace_digest: Some("trace-digest://task/patch-1".into()),
        memory_digest_refs: vec!["memory://digest/task-patch-1".into()],
        reusable_procedure:
            "Add the bounded reusable step to the governed target skill after approval.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::ExistingSkillPatchProposal,
        recommended_action: SkillEvolutionProposalAction::ProposePatch,
        target_skill_id: Some("skill://agent/target".into()),
        target_skill_name: Some("target-skill".into()),
        evidence_ids,
        metadata: BTreeMap::new(),
    }
}

fn policy_hints(approved: bool) -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.evolution.promote".into()],
        entitlement_ready: Some(approved),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn propose_patch_promote_and_reject_are_governed_service_commands() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-proposal-lifecycle");
    let proposed = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
            SkillEvolutionProposePatchCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: patch_candidate(vec!["evidence://patch/1".into()]),
            },
            trace.clone(),
        ))
        .await
        .expect("patch proposal command should accept verified evidence");
    let proposed: SkillEvolutionProposePatchResult =
        serde_json::from_value(proposed.output).expect("patch proposal result should decode");

    assert!(!proposed.mutated);
    assert_eq!(
        proposed.proposal.lifecycle,
        SkillEvolutionProposalLifecycle::Draft
    );
    assert_eq!(
        proposed.proposal.recommended_action,
        SkillEvolutionProposalAction::ProposePatch
    );

    let promoted = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND,
            SkillEvolutionPromoteDraftCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                proposal_id: proposed.proposal.proposal_id.clone(),
                reason: "approval accepted the reusable patch proposal".into(),
                evidence_ids: vec!["evidence://approval/1".into()],
                policy_decision_refs: vec!["policy://decision/promote".into()],
                policy: policy_hints(true),
            },
            trace.clone(),
        ))
        .await
        .expect("promotion should mutate governance metadata after policy refs");
    let promoted: SkillEvolutionPromoteDraftResult =
        serde_json::from_value(promoted.output).expect("promotion result should decode");

    assert!(promoted.mutated);
    assert_eq!(
        promoted.proposal.lifecycle,
        SkillEvolutionProposalLifecycle::Promoted
    );
    assert_eq!(promoted.record.lifecycle, SkillLifecycleState::Active);
    assert_eq!(
        promoted.policy_decision_refs,
        vec!["policy://decision/promote"]
    );

    let duplicate_promotion = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND,
            SkillEvolutionPromoteDraftCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                proposal_id: proposed.proposal.proposal_id.clone(),
                reason: "approval should not apply twice".into(),
                evidence_ids: vec!["evidence://approval/2".into()],
                policy_decision_refs: vec!["policy://decision/promote-duplicate".into()],
                policy: policy_hints(true),
            },
            trace.clone(),
        ))
        .await
        .expect_err("promoted proposals must reject duplicate promotion");
    assert!(duplicate_promotion.to_string().contains("already promoted"));

    let rejected = provider
        .call(traced_command(
            SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
            SkillEvolutionRejectDraftCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                proposal_id: proposed.proposal.proposal_id.clone(),
                rationale: "promotion already consumed this draft".into(),
                evidence_ids: vec!["evidence://reject/duplicate".into()],
                policy_decision_refs: vec!["policy://decision/reject".into()],
                policy: policy_hints(true),
            },
            trace,
        ))
        .await
        .expect_err("promoted proposals must reject a later rejection command");

    assert!(rejected.to_string().contains("already promoted"));
}

#[tokio::test]
async fn reject_draft_keeps_proposal_durable_without_active_governance_mutation() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-proposal-reject");
    let proposed = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
            SkillEvolutionProposePatchCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: patch_candidate(vec!["evidence://patch/reject".into()]),
            },
            trace.clone(),
        ))
        .await
        .expect("patch proposal command should create a durable draft");
    let proposed: SkillEvolutionProposePatchResult =
        serde_json::from_value(proposed.output).expect("patch proposal result should decode");

    let rejected = provider
        .call(traced_command(
            SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
            SkillEvolutionRejectDraftCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                proposal_id: proposed.proposal.proposal_id.clone(),
                rationale: "bounded evidence did not justify a skill change".into(),
                evidence_ids: vec!["evidence://reject/1".into()],
                policy_decision_refs: vec!["policy://decision/reject".into()],
                policy: policy_hints(true),
            },
            trace.clone(),
        ))
        .await
        .expect("rejection should preserve the proposal as auditable metadata");
    let rejected: SkillEvolutionRejectDraftResult =
        serde_json::from_value(rejected.output).expect("rejection result should decode");

    assert!(rejected.mutated);
    assert_eq!(
        rejected.proposal.lifecycle,
        SkillEvolutionProposalLifecycle::Rejected
    );
    assert_eq!(
        rejected.policy_decision_refs,
        vec!["policy://decision/reject"]
    );

    let snapshot = provider
        .call(traced_command(
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            SkillExperienceProposalSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_discarded: true,
            },
            trace.clone(),
        ))
        .await
        .expect("proposal snapshot should include rejected proposals");
    let snapshot: SkillExperienceProposalSnapshotResult =
        serde_json::from_value(snapshot.output).expect("proposal snapshot should decode");
    assert_eq!(snapshot.proposals.len(), 1);
    assert_eq!(
        snapshot.proposals[0].lifecycle,
        SkillEvolutionProposalLifecycle::Rejected
    );

    let governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace,
        ))
        .await
        .expect("governance snapshot should remain readable");
    let governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(governance.output).expect("governance snapshot should decode");
    assert!(governance.records.is_empty());
}

#[tokio::test]
async fn proposal_lifecycle_rejects_missing_trace_evidence_and_policy_denial() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-proposal-denials");

    let missing_evidence = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
            SkillEvolutionProposePatchCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: patch_candidate(Vec::new()),
            },
            trace.clone(),
        ))
        .await
        .expect_err("patch proposals require evidence refs");
    assert!(missing_evidence.to_string().contains("evidence"));

    let no_trace = provider
        .call(ServiceCommand::without_trace(
            ServiceCommandName::new(SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND),
            serde_json::json!({}),
        ))
        .await
        .expect_err("proposal lifecycle commands require trace context");
    assert!(no_trace.to_string().contains("missing trace"));

    let proposed = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
            SkillEvolutionProposePatchCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: patch_candidate(vec!["evidence://patch/denied".into()]),
            },
            trace.clone(),
        ))
        .await
        .expect("proposal should exist before policy denial");
    let proposed: SkillEvolutionProposePatchResult =
        serde_json::from_value(proposed.output).expect("proposal result should decode");

    let denied = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND,
            SkillEvolutionPromoteDraftCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                proposal_id: proposed.proposal.proposal_id,
                reason: "policy denied promotion".into(),
                evidence_ids: vec!["evidence://approval/denied".into()],
                policy_decision_refs: vec!["policy://decision/denied".into()],
                policy: policy_hints(false),
            },
            trace,
        ))
        .await
        .expect_err("policy denial must run before promotion side effects");
    assert!(denied.to_string().contains("policy denied"));
}
