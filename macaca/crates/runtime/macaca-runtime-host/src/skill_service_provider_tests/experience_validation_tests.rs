//! Contract tests for experience candidate validation rejection paths.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand, SkillServiceScope,
    SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{reusable_experience_candidate, traced_command};

#[tokio::test]
async fn skill_experience_proposal_rejects_missing_evidence() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-missing-evidence");
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate: reusable_experience_candidate(Vec::new()),
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("proposal without evidence must be rejected");

    assert!(
        err.to_string().contains("evidence"),
        "validation error should explain the missing evidence"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_unverified_terminal_status() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-unverified-status");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.verified_terminal_success = false;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("unverified terminal task status must be rejected");

    assert!(
        err.to_string().contains("verified terminal"),
        "validation error should explain terminal success requirement"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_rejected_evidence_gate() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-evidence-rejected");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.evidence_gate = SkillExperienceEvidenceGateStatus::Rejected;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("rejected evidence gate must be rejected before proposal creation");

    assert!(
        err.to_string().contains("evidence gate"),
        "validation error should explain evidence gate requirement"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_oversize_summary() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-oversize-summary");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.bounded_summary = "x".repeat(2_049);
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("oversize bounded summary must be rejected");

    assert!(
        err.to_string().contains("too large"),
        "validation error should explain bounded summary limit"
    );
}
