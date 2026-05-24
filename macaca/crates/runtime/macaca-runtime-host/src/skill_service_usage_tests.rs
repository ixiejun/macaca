use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillAuthorKind, SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillServiceScope, SkillUsageEventKind, SkillUsageObservation,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
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

fn observation(event: SkillUsageEventKind) -> SkillUsageObservation {
    SkillUsageObservation {
        skill_id: "skill://agent/usage-kinds".into(),
        name: "usage-kinds".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        event,
        author_kind: SkillAuthorKind::Agent,
        created_by: Some("agent".into()),
        pinned: None,
        evidence_id: Some("usage-kind-evidence".into()),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn skill_governance_records_distinct_usage_event_counters() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-governance-usage-kinds");

    for event in [
        SkillUsageEventKind::Viewed,
        SkillUsageEventKind::Activated,
        SkillUsageEventKind::ResourceRead,
        SkillUsageEventKind::Patched,
        SkillUsageEventKind::SuccessfulTask,
        SkillUsageEventKind::FailedTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("usage event should update sanitized governance telemetry");
    }

    let snapshot_payload = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot_payload,
            trace,
        ))
        .await
        .expect("snapshot should expose aggregate counters");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    let telemetry = &snapshot.records[0].telemetry;
    assert_eq!(telemetry.view_count, 1);
    assert_eq!(telemetry.activation_count, 1);
    assert_eq!(telemetry.resource_read_count, 1);
    assert_eq!(telemetry.patch_count, 1);
    assert_eq!(telemetry.successful_task_count, 1);
    assert_eq!(telemetry.failed_task_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.view_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.activation_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.resource_read_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.patch_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.successful_task_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.failed_task_count, 1);
}

#[tokio::test]
async fn skill_governance_usage_telemetry_replays_from_durable_journal_after_restart() {
    let temp = tempfile::tempdir().expect("temp governance journal directory should exist");
    let journal_path = temp.path().join("skill-governance-events.jsonl");
    let trace = TraceContext::new("trace-skill-governance-usage-replay");

    let provider =
        SkillSystemServiceProvider::new().with_governance_event_journal_path(journal_path.clone());
    provider
        .start()
        .await
        .expect("first provider should start with a writable journal");

    for event in [
        SkillUsageEventKind::Created,
        SkillUsageEventKind::Activated,
        SkillUsageEventKind::SuccessfulTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("usage event should be appended to the durable journal");
    }

    let restarted_provider =
        SkillSystemServiceProvider::new().with_governance_event_journal_path(journal_path);
    restarted_provider
        .start()
        .await
        .expect("second provider should replay the same durable journal");

    let snapshot_payload = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = restarted_provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot_payload,
            trace,
        ))
        .await
        .expect("snapshot should expose replayed aggregate counters");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    let telemetry = &snapshot.records[0].telemetry;

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(telemetry.activation_count, 1);
    assert_eq!(telemetry.use_count, 1);
    assert_eq!(telemetry.successful_task_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.activation_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.use_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.successful_task_count, 1);
}
