//! Contract tests for curation rollback memento restore and telemetry dry-run counters.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillCurationAction, SkillCurationDryRunCommand, SkillCurationDryRunResult,
    SkillCurationRollbackCommand, SkillCurationRollbackResult, SkillCurationRunCommand,
    SkillCurationRunResult, SkillCurationSnapshotCommand, SkillCurationSnapshotResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillServiceScope, SkillUsageEventKind,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{alias_record, observation, traced_command};

#[tokio::test]
async fn skill_curation_rollback_restores_governance_alias_and_refs_from_memento() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-rollback");
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
        .expect("usage observation should seed rollback memento records");

    let run = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: vec!["approval://skill-curation/apply".into()],
                policy_decision_refs: vec!["policy://skill-curation/apply".into()],
                audit_event_ids: vec!["audit://skill-curation/apply".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("approved apply run should create rollback memento refs");
    let run: SkillCurationRunResult =
        serde_json::from_value(run.output).expect("apply run result should decode");
    let rollback_ref = run
        .rollback_ref
        .clone()
        .expect("apply run should return a rollback ref");

    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::FailedTask, None),
            },
            trace.clone(),
        ))
        .await
        .expect("post-memento telemetry mutation should be observable");
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
        .expect("post-memento alias mutation should be observable");

    let rollback = provider
        .call(traced_command(
            SKILL_CURATION_ROLLBACK_COMMAND,
            SkillCurationRollbackCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                rollback_ref: rollback_ref.clone(),
                approval_refs: vec!["approval://skill-curation/rollback".into()],
                policy_decision_refs: vec!["policy://skill-curation/rollback".into()],
                audit_event_ids: vec!["audit://skill-curation/rollback".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("rollback command should restore the pre-apply memento");
    let rollback: SkillCurationRollbackResult =
        serde_json::from_value(rollback.output).expect("rollback result should decode");
    assert_eq!(rollback.rollback_ref, rollback_ref);
    assert!(rollback.mutated);
    assert_eq!(rollback.restored_record_count, 1);
    assert_eq!(rollback.restored_alias_count, 0);
    assert!(rollback
        .restored_report_refs
        .iter()
        .any(|report| report.contains("REPORT.md")));
    assert!(rollback
        .restored_report_refs
        .iter()
        .any(|report| report.contains("run.json")));
    assert!(rollback.package_memento_refs.contains(&rollback_ref));

    let governance = provider
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
        .expect("governance snapshot should show restored telemetry");
    let governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(governance.output).expect("governance snapshot should decode");
    assert_eq!(governance.records.len(), 1);
    assert_eq!(governance.records[0].telemetry.use_count, 1);
    assert_eq!(governance.records[0].telemetry.failed_task_count, 0);

    let aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace,
        ))
        .await
        .expect("alias snapshot should show restored alias map");
    let aliases: SkillAliasSnapshotResult =
        serde_json::from_value(aliases.output).expect("alias snapshot should decode");
    assert!(aliases.aliases.is_empty());
}

#[tokio::test]
async fn skill_governance_dry_run_uses_success_failure_counters_generically() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-telemetry-rationale");

    for event in [
        SkillUsageEventKind::Activated,
        SkillUsageEventKind::SuccessfulTask,
        SkillUsageEventKind::FailedTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event, None),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("effectiveness telemetry should record without application semantics");
    }

    let dry_run = SkillCurationDryRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        stale_after_days: 30,
        narrow_use_threshold: 1,
    };
    let result = provider
        .call(traced_command(
            SKILL_CURATION_DRY_RUN_COMMAND,
            dry_run,
            trace.clone(),
        ))
        .await
        .expect("dry-run should use generic effectiveness counters");
    let result: SkillCurationDryRunResult =
        serde_json::from_value(result.output).expect("dry-run result should decode");

    assert_eq!(result.recommendations.len(), 1);
    assert_eq!(result.recommendations[0].action, SkillCurationAction::Keep);
    assert!(result.recommendations[0]
        .rationale
        .contains("sufficient usage"));

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("snapshot should expose aggregate counters used by dry-run");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    assert_eq!(snapshot.telemetry_aggregate.activation_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.successful_task_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.failed_task_count, 1);
}
