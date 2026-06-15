//! Contract test for governance event journal replay read model sanitization.

use macaca_proto::TraceContext;
use macaca_skill::{
    SkillCurationRunRecord, SkillExperienceProposalRecord, SkillGovernanceEventPayload,
    SkillGovernanceEventRecord, SkillGovernanceReadModel, SkillGovernanceSnapshotRefRecord,
    SkillProvenanceAction, SkillRollbackRefRecord, SkillServiceScope, SkillUsageEventKind,
};

use super::fixtures::{alias_record, observation, reusable_experience_candidate};

#[test]
fn skill_governance_event_replay_restores_read_model_without_skill_bodies() {
    let trace = TraceContext::new("trace-skill-governance-replay");
    let created_at = chrono::Utc::now();
    let proposal = macaca_skill::SkillExperienceProposalRecord::from_candidate(
        &trace,
        reusable_experience_candidate(vec!["artifact-proof-1".into()]),
        created_at,
    );
    let run = SkillCurationRunRecord {
        run_id: "curation-run-1".into(),
        trace_id: trace.trace_id.clone(),
        provider_id: "builtin-local-governance-store".into(),
        dry_run: true,
        candidate_count: 1,
        actions: vec!["skill://agent/example:WouldArchive".into()],
        snapshot_refs: vec!["store://skill-curation/run-1/snapshot".into()],
        started_at: created_at,
        finished_at: Some(created_at),
        run_json_ref: Some("store://skill-curation/run-1/run.json".into()),
        report_ref: Some("store://skill-curation/run-1/report".into()),
        rollback_ref: None,
        policy_decision_ids: vec!["policy-decision-1".into()],
        audit_event_ids: vec!["audit-event-1".into()],
    };
    let rollback = SkillRollbackRefRecord {
        rollback_ref: "store://skill-curation/run-1/rollback".into(),
        run_id: run.run_id.clone(),
        trace_id: trace.trace_id.clone(),
        before_snapshot_ref: "store://skill-curation/run-1/before".into(),
        after_snapshot_ref: Some("store://skill-curation/run-1/after".into()),
        report_ref: run.report_ref.clone(),
        captured_at: created_at,
    };
    let events = vec![
        SkillGovernanceEventRecord::new(
            "event-discovery-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Created,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-usage-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Used,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-patch-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Patched,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-lifecycle-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::LifecycleApplied(observation(
                SkillUsageEventKind::Archived,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-alias-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::AliasUpserted(alias_record()),
        ),
        SkillGovernanceEventRecord::new(
            "event-proposal-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal),
        ),
        SkillGovernanceEventRecord::new(
            "event-run-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::CurationRunRecorded(run),
        ),
        SkillGovernanceEventRecord::new(
            "event-snapshot-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::SnapshotRefRecorded(SkillGovernanceSnapshotRefRecord {
                snapshot_ref: "store://skill-curation/run-1/snapshot".into(),
                trace_id: trace.trace_id.clone(),
                record_count: 1,
                captured_at: created_at,
                report_ref: Some("store://skill-curation/run-1/report".into()),
            }),
        ),
        SkillGovernanceEventRecord::new(
            "event-rollback-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::RollbackRefRecorded(rollback),
        ),
    ];

    let read_model = SkillGovernanceReadModel::from_events(events);

    assert_eq!(read_model.records.len(), 1);
    assert_eq!(read_model.records[0].telemetry.use_count, 1);
    assert_eq!(read_model.records[0].telemetry.patch_count, 1);
    assert_eq!(
        read_model.records[0].lifecycle,
        macaca_skill::SkillLifecycleState::Archived
    );
    assert_eq!(read_model.aliases.len(), 1);
    assert_eq!(read_model.proposals.len(), 1);
    assert_eq!(read_model.curation_runs.len(), 1);
    assert_eq!(read_model.snapshot_refs.len(), 1);
    assert_eq!(read_model.rollback_refs.len(), 1);
    assert_eq!(read_model.replayed_events, 9);
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::Discovered)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::Patched)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::CurationRecorded)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::RollbackRecorded)));
    assert!(serde_json::to_string(&read_model)
        .expect("read model should serialize")
        .contains("store://skill-curation/run-1/report"));
    assert!(
        !serde_json::to_string(&read_model)
            .expect("read model should serialize")
            .contains("SKILL.md body"),
        "governance replay must not materialize full skill instruction bodies"
    );
}
