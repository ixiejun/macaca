use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolveCommand, SkillAliasResolveResult,
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillAliasUpsertResult, SkillAuthorKind, SkillCurationAction, SkillCurationDryRunCommand,
    SkillCurationDryRunResult, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillServiceScope,
    SkillUsageEventKind, SkillUsageObservation, SKILL_ALIAS_RESOLVE_COMMAND,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND,
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

fn observation(event: SkillUsageEventKind, pinned: Option<bool>) -> SkillUsageObservation {
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

fn alias_record() -> SkillAliasRecord {
    let now = chrono::Utc::now();
    SkillAliasRecord {
        source_skill_id: "skill://agent/old".into(),
        source_name: "old-skill".into(),
        target_skill_id: "skill://agent/new".into(),
        target_name: "new-skill".into(),
        kind: SkillAliasKind::AbsorbedInto,
        rationale: "old skill was absorbed into the broader replacement skill".into(),
        created_at: now,
        updated_at: now,
        evidence_ids: vec!["curation-run-1".into()],
    }
}

#[tokio::test]
async fn skill_governance_records_usage_and_snapshots_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-governance-record");
    let payload = SkillGovernanceRecordUsageCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        observation: observation(SkillUsageEventKind::Used, None),
    };

    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            payload,
            trace.clone(),
        ))
        .await
        .expect("usage recording should succeed");
    let typed: SkillGovernanceRecordUsageResult =
        serde_json::from_value(result.output).expect("usage result should decode");

    assert_eq!(typed.record.provenance.name, "agent-example");
    assert_eq!(typed.record.telemetry.use_count, 1);
    assert_eq!(typed.record.provenance.author_kind, SkillAuthorKind::Agent);

    let snapshot_payload = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot_payload,
            trace,
        ))
        .await
        .expect("snapshot should succeed");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].telemetry.use_count, 1);
}

#[tokio::test]
async fn skill_governance_dry_run_keeps_pinned_skills_protected() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-dry-run");
    let payload = SkillGovernanceRecordUsageCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        observation: observation(SkillUsageEventKind::Pinned, Some(true)),
    };
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            payload,
            trace.clone(),
        ))
        .await
        .expect("pinned observation should succeed");

    let dry_run = SkillCurationDryRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        stale_after_days: 0,
        narrow_use_threshold: 0,
    };
    let result = provider
        .call(traced_command(
            SKILL_CURATION_DRY_RUN_COMMAND,
            dry_run,
            trace,
        ))
        .await
        .expect("dry-run should succeed");
    let result: SkillCurationDryRunResult =
        serde_json::from_value(result.output).expect("dry-run result should decode");

    assert!(!result.mutated);
    assert_eq!(result.recommendations.len(), 1);
    assert_eq!(
        result.recommendations[0].action,
        SkillCurationAction::Protected
    );
    assert!(result.recommendations[0].protected);
}

#[tokio::test]
async fn skill_alias_upsert_resolve_and_snapshot_are_service_owned() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-alias-upsert");
    let upsert = SkillAliasUpsertCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record: alias_record(),
    };

    let result = provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            upsert,
            trace.clone(),
        ))
        .await
        .expect("alias upsert should succeed");
    let upsert: SkillAliasUpsertResult =
        serde_json::from_value(result.output).expect("alias upsert result should decode");
    assert_eq!(upsert.record.kind, SkillAliasKind::AbsorbedInto);

    let resolve = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/old".into(),
        name: Some("old-skill".into()),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_RESOLVE_COMMAND,
            resolve,
            trace.clone(),
        ))
        .await
        .expect("alias resolve should succeed");
    let resolved: SkillAliasResolveResult =
        serde_json::from_value(result.output).expect("alias resolve result should decode");
    assert!(resolved.resolved);
    assert_eq!(
        resolved.target_skill_id.as_deref(),
        Some("skill://agent/new")
    );

    let snapshot = SkillAliasSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("alias snapshot should succeed");
    let snapshot: SkillAliasSnapshotResult =
        serde_json::from_value(result.output).expect("alias snapshot result should decode");
    assert_eq!(snapshot.aliases.len(), 1);
    assert_eq!(snapshot.aliases[0].target_name, "new-skill");
}

#[tokio::test]
async fn skill_alias_resolve_without_record_does_not_fake_fallback() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-alias-unresolved");
    let resolve = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/missing".into(),
        name: Some("missing-skill".into()),
    };

    let result = provider
        .call(traced_command(SKILL_ALIAS_RESOLVE_COMMAND, resolve, trace))
        .await
        .expect("alias resolve should succeed");
    let resolved: SkillAliasResolveResult =
        serde_json::from_value(result.output).expect("alias resolve result should decode");

    assert!(!resolved.resolved);
    assert!(resolved.target_skill_id.is_none());
    assert!(resolved.kind.is_none());
}
