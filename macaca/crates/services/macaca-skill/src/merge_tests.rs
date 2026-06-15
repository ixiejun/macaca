use crate::{
    SkillAliasKind, SkillMergeAliasEffect, SkillMergeRiskScore, SkillMergeSupportFileDestination,
    SkillMergeSupportFileMovement, SkillSupportFileDemotionKind, SkillSupportFileDemotionProposal,
    SkillUmbrellaMergeApplyCommand, SkillUmbrellaMergeProposal,
};

#[test]
fn umbrella_merge_proposal_carries_metadata_only_contract() {
    let proposal = SkillUmbrellaMergeProposal {
        proposal_id: "merge://proposal/agent-debugging".into(),
        source_skill_ids: vec!["skill://agent/debug-heartbeat".into()],
        target_umbrella_skill_id: "skill://agent/debugging".into(),
        support_file_movements: vec![SkillMergeSupportFileMovement {
            source_skill_id: "skill://agent/debug-heartbeat".into(),
            source_path: "SKILL.md#case-study".into(),
            destination: SkillMergeSupportFileDestination::References,
            destination_path: "references/heartbeat-live-proof.md".into(),
            rationale: "session-specific proof steps belong in references".into(),
            evidence_ids: vec!["evidence://merge/reference-demotion".into()],
        }],
        alias_effects: vec![SkillMergeAliasEffect {
            source_skill_id: "skill://agent/debug-heartbeat".into(),
            source_name: "debug-heartbeat".into(),
            target_skill_id: "skill://agent/debugging".into(),
            target_name: "debugging".into(),
            kind: SkillAliasKind::AbsorbedInto,
            rationale: "source skill becomes a redirect to the umbrella flow".into(),
            evidence_ids: vec!["evidence://merge/alias".into()],
        }],
        risk: SkillMergeRiskScore {
            score: 0.35,
            reasons: vec!["aligned diagnostic flow".into()],
        },
        policy_decision_refs: vec!["policy://skill-merge/aligned".into()],
        evidence_ids: vec!["evidence://merge/proposal".into()],
        rationale: "merge keeps reusable debugging flow in the umbrella skill".into(),
    };

    assert!(proposal.validate().is_ok());
    assert_eq!(proposal.alias_effects[0].kind, SkillAliasKind::AbsorbedInto);
    assert_eq!(
        proposal.support_file_movements[0].destination,
        SkillMergeSupportFileDestination::References
    );
}

#[test]
fn umbrella_merge_proposal_rejects_invalid_identity_and_risk() {
    let mut proposal = SkillUmbrellaMergeProposal::metadata_only(
        "merge://proposal/invalid",
        vec!["skill://agent/source".into()],
        "skill://agent/target",
        "bounded rationale",
    );
    proposal.risk.score = 1.5;
    assert!(proposal.validate().is_err());

    proposal.risk.score = 0.5;
    proposal.source_skill_ids.clear();
    assert!(proposal.validate().is_err());
}

#[test]
fn support_file_demotion_proposal_matches_destination_class() {
    let proposal = SkillSupportFileDemotionProposal {
        skill_id: "skill://agent/debugging".into(),
        source_path: "SKILL.md#incident-notes".into(),
        destination: SkillMergeSupportFileDestination::References,
        kind: SkillSupportFileDemotionKind::SessionSpecificReference,
        destination_path: "references/incident-notes.md".into(),
        rationale: "incident notes are useful evidence but not core workflow".into(),
        evidence_ids: vec!["evidence://demotion/reference".into()],
    };

    assert!(proposal.validate().is_ok());

    let mut invalid = proposal;
    invalid.destination = SkillMergeSupportFileDestination::Scripts;
    assert!(invalid.validate().is_err());
}

#[test]
fn umbrella_merge_apply_requires_approval_and_command_refs() {
    let proposal = SkillUmbrellaMergeProposal::metadata_only(
        "merge://proposal/apply",
        vec!["skill://agent/source".into()],
        "skill://agent/target",
        "bounded rationale",
    );
    let mut command = SkillUmbrellaMergeApplyCommand {
        trace: macaca_proto::TraceContext::new("trace-merge-apply"),
        scope: Default::default(),
        proposal,
        approval_refs: Vec::new(),
        policy_decision_refs: Vec::new(),
        lifecycle_transition_refs: Vec::new(),
        safe_mutation_refs: Vec::new(),
        audit_event_ids: Vec::new(),
    };

    assert!(command.validate().is_err());

    command.approval_refs = vec!["approval://merge/apply".into()];
    command.policy_decision_refs = vec!["policy://merge/apply".into()];
    command.lifecycle_transition_refs = vec!["skill.curation.supersede://source".into()];
    command.safe_mutation_refs = vec!["skill.content.mutate://support-file".into()];
    assert!(command.validate().is_ok());
}
