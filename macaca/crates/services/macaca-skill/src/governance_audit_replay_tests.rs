use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::TraceContext;

use crate::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolutionPolicy, SkillAuthorKind,
    SkillCurationRunRecord, SkillEvolutionCandidateClassification, SkillEvolutionProposalAction,
    SkillEvolutionProposalLifecycle, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalRecord, SkillGovernanceEventPayload,
    SkillGovernanceEventRecord, SkillGovernanceReadModel, SkillRollbackRefRecord,
    SkillServiceScope, SkillUsageEventKind, SkillUsageObservation,
};

fn scope() -> SkillServiceScope {
    SkillServiceScope {
        application_id: None,
        session_id: Some("session-audit-replay".into()),
        tenant_id: Some("tenant-audit-replay".into()),
        agent_name: Some("agent-audit".into()),
    }
}

fn event(
    id: &str,
    trace: &TraceContext,
    payload: SkillGovernanceEventPayload,
) -> SkillGovernanceEventRecord {
    SkillGovernanceEventRecord::new(id, trace, scope(), Utc::now(), payload)
}

fn proposal_record(
    trace: &TraceContext,
    task_id: &str,
    lifecycle: SkillEvolutionProposalLifecycle,
) -> SkillExperienceProposalRecord {
    let mut record = SkillExperienceProposalRecord::from_candidate(
        trace,
        SkillExperienceCandidate {
            task_id: task_id.into(),
            session_id: Some("session-audit-replay".into()),
            application_id: None,
            agent_name: Some("agent-audit".into()),
            verified_terminal_success: true,
            evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
            bounded_summary: "Bounded proposal audit replay summary.".into(),
            trace_digest: Some(format!("trace-digest://{task_id}")),
            memory_digest_refs: vec![format!("memory://digest/{task_id}")],
            reusable_procedure: "Replay proposal lifecycle from Store/EventLog events.".into(),
            classification: SkillEvolutionCandidateClassification::ReusableProcedure,
            destination: SkillExperienceCandidateDestination::ExistingSkillPatchProposal,
            recommended_action: SkillEvolutionProposalAction::ProposePatch,
            target_skill_id: Some("skill://agent/audit-target".into()),
            target_skill_name: Some("audit-target".into()),
            evidence_ids: vec![format!("evidence://{task_id}")],
            metadata: BTreeMap::new(),
        },
        Utc::now(),
    );
    record.lifecycle = lifecycle;
    record
}

fn lifecycle_observation(event_kind: SkillUsageEventKind) -> SkillUsageObservation {
    let mut metadata = BTreeMap::new();
    metadata.insert("task_id".into(), "task-audit-replay".into());
    metadata.insert("action_ref".into(), "skill.content.mutate://audit".into());
    metadata.insert(
        "target_skill_id".into(),
        "skill://agent/audit-target".into(),
    );
    SkillUsageObservation {
        skill_id: "skill://agent/audit-target".into(),
        name: "audit-target".into(),
        source: "skill.governance.audit".into(),
        source_scope: "governance".into(),
        event: event_kind,
        author_kind: SkillAuthorKind::Agent,
        created_by: Some("agent-audit".into()),
        pinned: None,
        evidence_id: Some("evidence://audit-lifecycle".into()),
        metadata,
    }
}

fn alias_record() -> SkillAliasRecord {
    let now = Utc::now();
    SkillAliasRecord {
        source_skill_id: "skill://agent/audit-old".into(),
        source_name: "audit-old".into(),
        target_skill_id: "skill://agent/audit-target".into(),
        target_name: "audit-target".into(),
        kind: SkillAliasKind::SupersededBy,
        resolution_policy: SkillAliasResolutionPolicy::Redirect,
        valid_from: now,
        valid_until: None,
        rationale: "audit replay alias redirect".into(),
        created_at: now,
        updated_at: now,
        evidence_ids: vec!["evidence://audit-alias".into()],
    }
}

#[test]
fn audit_replay_reconstructs_skill_governance_histories_from_events() {
    let trace = TraceContext::new("trace-skill-audit-replay");
    let draft = proposal_record(
        &trace,
        "task-audit-proposal",
        SkillEvolutionProposalLifecycle::Draft,
    );
    let promoted = proposal_record(
        &trace,
        "task-audit-promotion",
        SkillEvolutionProposalLifecycle::Promoted,
    );
    let rejected = proposal_record(
        &trace,
        "task-audit-rejection",
        SkillEvolutionProposalLifecycle::Rejected,
    );
    let curation_run = SkillCurationRunRecord {
        run_id: "curation-run-audit".into(),
        trace_id: trace.trace_id.clone(),
        provider_id: "local-skill-governance".into(),
        dry_run: false,
        candidate_count: 2,
        actions: vec!["archive".into(), "alias".into()],
        snapshot_refs: vec![
            "store://snapshot/before".into(),
            "store://snapshot/after".into(),
        ],
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        run_json_ref: Some("store://reports/run.json".into()),
        report_ref: Some("store://reports/REPORT.md".into()),
        rollback_ref: Some("store://rollback/curation-run-audit".into()),
        policy_decision_ids: vec!["policy://decision/curation".into()],
        audit_event_ids: vec!["audit://event/curation".into()],
    };
    let rollback = SkillRollbackRefRecord {
        rollback_ref: "store://rollback/curation-run-audit".into(),
        run_id: "curation-run-audit".into(),
        trace_id: trace.trace_id.clone(),
        before_snapshot_ref: "store://snapshot/before".into(),
        after_snapshot_ref: Some("store://snapshot/after".into()),
        report_ref: Some("store://reports/REPORT.md".into()),
        captured_at: Utc::now(),
    };

    let read_model = SkillGovernanceReadModel::from_events([
        event(
            "event-proposal-draft",
            &trace,
            SkillGovernanceEventPayload::ProposalCreated(draft),
        ),
        event(
            "event-proposal-promoted",
            &trace,
            SkillGovernanceEventPayload::ProposalCreated(promoted),
        ),
        event(
            "event-proposal-rejected",
            &trace,
            SkillGovernanceEventPayload::ProposalCreated(rejected),
        ),
        event(
            "event-mutation",
            &trace,
            SkillGovernanceEventPayload::LifecycleApplied(lifecycle_observation(
                SkillUsageEventKind::Patched,
            )),
        ),
        event(
            "event-alias",
            &trace,
            SkillGovernanceEventPayload::AliasUpserted(alias_record()),
        ),
        event(
            "event-curation-run",
            &trace,
            SkillGovernanceEventPayload::CurationRunRecorded(curation_run),
        ),
        event(
            "event-rollback",
            &trace,
            SkillGovernanceEventPayload::RollbackRefRecorded(rollback),
        ),
    ]);

    assert_eq!(read_model.replayed_events, 7);
    assert_eq!(read_model.proposals.len(), 3);
    assert!(read_model
        .proposals
        .iter()
        .any(|record| record.lifecycle == SkillEvolutionProposalLifecycle::Draft));
    assert!(read_model
        .proposals
        .iter()
        .any(|record| record.lifecycle == SkillEvolutionProposalLifecycle::Promoted));
    assert!(read_model
        .proposals
        .iter()
        .any(|record| record.lifecycle == SkillEvolutionProposalLifecycle::Rejected));
    assert_eq!(read_model.records.len(), 1);
    assert_eq!(read_model.records[0].telemetry.patch_count, 1);
    assert_eq!(read_model.aliases.len(), 1);
    assert_eq!(
        read_model.aliases[0].target_skill_id,
        "skill://agent/audit-target"
    );
    assert_eq!(read_model.curation_runs.len(), 1);
    assert_eq!(
        read_model.curation_runs[0].policy_decision_ids,
        vec!["policy://decision/curation"]
    );
    assert_eq!(read_model.rollback_refs.len(), 1);
    assert_eq!(
        read_model.rollback_refs[0].before_snapshot_ref,
        "store://snapshot/before"
    );
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| event.action_ref.as_deref() == Some("skill.content.mutate://audit")));
}
