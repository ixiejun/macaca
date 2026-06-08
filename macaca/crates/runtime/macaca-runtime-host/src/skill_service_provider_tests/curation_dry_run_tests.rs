//! Contract tests for Skill curation dry-run paths that must not mutate active state.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillCurationAction, SkillCurationDryRunCommand, SkillCurationDryRunResult,
    SkillCurationPhase, SkillCurationRunCommand, SkillCurationRunResult, SkillCurationSnapshotCommand,
    SkillCurationSnapshotResult, SkillCurationStatusCommand, SkillCurationStatusResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillSemanticReviewStatus, SkillServiceScope,
    SkillUsageEventKind, SkillAuthorKind, SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND,
    SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_CURATION_STATUS_COMMAND, SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{alias_record, observation, traced_command};

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
async fn skill_curation_status_reports_read_only_local_provider_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-status");
    let result = provider
        .call(traced_command(
            SKILL_CURATION_STATUS_COMMAND,
            SkillCurationStatusCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                interval_ms: 60_000,
                idle_budget_ms: Some(10_000),
            },
            trace,
        ))
        .await
        .expect("curation status should be a read-only provider command");
    let result: SkillCurationStatusResult =
        serde_json::from_value(result.output).expect("curation status result should decode");

    assert!(result.available);
    assert_eq!(result.provider_id, "local-skill-governance");
    assert_eq!(result.interval_ms, 60_000);
    assert_eq!(result.idle_budget_ms, Some(10_000));
    assert!(result.last_run_id.is_none());
    assert!(result.next_eligible_run_at.is_none());
}

#[tokio::test]
async fn skill_curation_run_records_bounded_dry_run_without_file_mutation() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-run-dry");
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::Used, None),
            },
            trace.clone(),
        ))
        .await
        .expect("usage observation should seed curation candidates");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/dry-run".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("dry-run curation command should be accepted");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("run result should decode");

    assert!(result.run.dry_run);
    assert!(!result.mutated);
    assert_eq!(result.run.candidate_count, 1);
    assert_eq!(result.recommendations.len(), 1);
    assert!(result
        .report_ref
        .as_deref()
        .unwrap_or("")
        .contains("REPORT.md"));
    assert!(result
        .run_json_ref
        .as_deref()
        .unwrap_or("")
        .contains("run.json"));
    assert!(result.rollback_ref.is_none());

    let status = provider
        .call(traced_command(
            SKILL_CURATION_STATUS_COMMAND,
            SkillCurationStatusCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                interval_ms: 60_000,
                idle_budget_ms: None,
            },
            trace,
        ))
        .await
        .expect("curation status should observe the recorded run");
    let status: SkillCurationStatusResult =
        serde_json::from_value(status.output).expect("status result should decode");
    assert_eq!(status.last_run_id, Some(result.run.run_id));
}

#[tokio::test]
async fn skill_curation_dry_run_does_not_mutate_active_governance_or_alias_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-dry-run-immutability");
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::Used, None),
            },
            trace.clone(),
        ))
        .await
        .expect("usage observation should seed immutable dry-run records");
    provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            SkillAliasUpsertCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                record: alias_record(),
            },
            trace.clone(),
        ))
        .await
        .expect("alias state should exist before dry-run");

    let before_governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace.clone(),
        ))
        .await
        .expect("pre dry-run governance snapshot should decode");
    let before_governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(before_governance.output).expect("snapshot should decode");
    let before_aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("pre dry-run alias snapshot should decode");
    let before_aliases: SkillAliasSnapshotResult =
        serde_json::from_value(before_aliases.output).expect("alias snapshot should decode");

    let dry_run = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/dry-run-immutability".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("dry-run command should be admitted without approval refs");
    let dry_run: SkillCurationRunResult =
        serde_json::from_value(dry_run.output).expect("dry-run result should decode");
    assert!(!dry_run.mutated);
    assert!(dry_run.rollback_ref.is_none());

    let after_governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace.clone(),
        ))
        .await
        .expect("post dry-run governance snapshot should decode");
    let after_governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(after_governance.output).expect("snapshot should decode");
    let after_aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("post dry-run alias snapshot should decode");
    let after_aliases: SkillAliasSnapshotResult =
        serde_json::from_value(after_aliases.output).expect("alias snapshot should decode");
    assert_eq!(after_governance.records, before_governance.records);
    assert_eq!(after_aliases.aliases, before_aliases.aliases);

    let curation_snapshot = provider
        .call(traced_command(
            SKILL_CURATION_SNAPSHOT_COMMAND,
            SkillCurationSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
                include_package_mementos: true,
            },
            trace,
        ))
        .await
        .expect("curation snapshot should expose refs only after dry-run");
    let curation_snapshot: SkillCurationSnapshotResult =
        serde_json::from_value(curation_snapshot.output).expect("curation snapshot should decode");
    assert!(curation_snapshot.rollback_refs.is_empty());
    assert!(curation_snapshot.package_memento_refs.is_empty());
}
