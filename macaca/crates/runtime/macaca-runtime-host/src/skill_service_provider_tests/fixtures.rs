//! Object Mother fixtures for Skill service provider contract tests.
//!
//! Builders construct traced service commands and reusable governance observations
//! so individual test cases assert one behavioral contract at a time.

use std::collections::BTreeMap;

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs,
    SelfEvolutionRunMetrics, SelfEvolutionWhiteBoxEvidence, SkillAliasKind, SkillAliasRecord,
    SkillAliasResolutionPolicy, SkillAuthorKind, SkillEvolutionCandidateClassification,
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillUsageEventKind, SkillUsageObservation,
};

pub(super) fn traced_command<T: serde::Serialize>(
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

pub(super) fn observation(
    event: SkillUsageEventKind,
    pinned: Option<bool>,
) -> SkillUsageObservation {
    SkillUsageObservation {
        skill_id: "skill://agent/example".into(),
        name: "agent-example".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        event,
        author_kind: SkillAuthorKind::Agent,
        created_by: Some("agent".into()),
        pinned,
        evidence_id: Some("event-1".into()),
        metadata: BTreeMap::new(),
    }
}

pub(super) fn alias_record() -> SkillAliasRecord {
    let now = chrono::Utc::now();
    SkillAliasRecord {
        source_skill_id: "skill://agent/old".into(),
        source_name: "old-skill".into(),
        target_skill_id: "skill://agent/new".into(),
        target_name: "new-skill".into(),
        kind: SkillAliasKind::AbsorbedInto,
        resolution_policy: SkillAliasResolutionPolicy::Redirect,
        valid_from: now,
        valid_until: None,
        rationale: "old skill was absorbed into the broader replacement skill".into(),
        created_at: now,
        updated_at: now,
        evidence_ids: vec!["curation-run-1".into()],
    }
}

pub(super) fn reusable_experience_candidate(evidence_ids: Vec<String>) -> SkillExperienceCandidate {
    SkillExperienceCandidate {
        task_id: "task-verified-1".into(),
        session_id: Some("session-verified-1".into()),
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "A verified task produced a reusable skill maintenance procedure.".into(),
        trace_digest: Some("trace-digest://task/verified-1".into()),
        memory_digest_refs: vec!["memory://digest/task-verified-1".into()],
        reusable_procedure: "Record governance evidence, propose a draft skill, and keep active skill files unchanged until approval.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some("skill-experience-maintenance".into()),
        evidence_ids,
        metadata: BTreeMap::new(),
    }
}

pub(super) fn complete_evaluation_record() -> SelfEvolutionEvaluationRecord {
    SelfEvolutionEvaluationRecord {
        evaluation_id: "eval-runtime".into(),
        trace_id: "trace-runtime".into(),
        task_family_id: "runtime_verification_loop".into(),
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
            human_intervention_count: 2,
            elapsed_seconds: Some(100),
            tool_call_count: 18,
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
            elapsed_seconds: Some(80),
            tool_call_count: 14,
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
