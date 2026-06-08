//! Contract tests for curation run records, snapshots, and sanitized reporting.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillAuthorKind, SkillCurationAction, SkillCurationPhase, SkillCurationRunCommand,
    SkillCurationRunResult,
    SkillCurationSnapshotCommand, SkillCurationSnapshotResult, SkillGovernanceRecordUsageCommand,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillSemanticReviewStatus,
    SkillServiceScope, SkillUsageEventKind, SKILL_ALIAS_SNAPSHOT_COMMAND,
    SKILL_ALIAS_UPSERT_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{alias_record, observation, traced_command};

#[tokio::test]
async fn skill_curation_report_keeps_protected_ownership_and_absent_provider_sanitized() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-report-sanitized");
    let mut protected_system_skill = observation(SkillUsageEventKind::Created, None);
    protected_system_skill.skill_id = "skill://system/protected".into();
    protected_system_skill.name = "system-protected".into();
    protected_system_skill.author_kind = SkillAuthorKind::System;
    protected_system_skill.evidence_id = Some("evidence://system/protected".into());
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: protected_system_skill,
            },
            trace.clone(),
        ))
        .await
        .expect("protected ownership observation should seed curation report");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 0,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/report-sanitized".into()],
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect("dry-run report command should be accepted without semantic provider");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("curation run result should decode");

    assert!(result.semantic_analysis_status.contains("unavailable"));
    let recommendation = result
        .recommendations
        .iter()
        .find(|candidate| candidate.skill_id == "skill://system/protected")
        .expect("protected ownership recommendation should exist");
    assert_eq!(recommendation.action, SkillCurationAction::Protected);
    assert!(recommendation.protected);
    assert!(recommendation
        .phases
        .contains(&SkillCurationPhase::Protected));

    let report_ref = result.report_ref.as_deref().unwrap_or("");
    let run_json_ref = result.run_json_ref.as_deref().unwrap_or("");
    assert!(report_ref.starts_with("store://skill-curation/"));
    assert!(report_ref.ends_with("/REPORT.md"));
    assert!(run_json_ref.starts_with("store://skill-curation/"));
    assert!(run_json_ref.ends_with("/run.json"));
    assert!(!report_ref.contains("SKILL.md"));
    assert!(!report_ref.contains("prompt"));
    assert!(!run_json_ref.contains("provider_payload"));
    assert!(result.rollback_ref.is_none());
    assert!(!result.mutated);
}

#[tokio::test]
async fn skill_curation_run_records_structured_absent_semantic_provider() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-semantic-provider-absent");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 0,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/semantic-absent".into()],
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect("dry-run should preserve deterministic curation when semantic provider is absent");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("curation run result should decode");

    assert_eq!(
        result.semantic_review.status,
        SkillSemanticReviewStatus::Unavailable
    );
    assert!(result.semantic_review.proposals.is_empty());
    assert!(!result.semantic_review.mutated);
    assert!(result
        .semantic_review
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.contains("semantic review provider is unavailable") }));
    let sanitized_debug = format!("{:?}", result.semantic_review);
    assert!(!sanitized_debug.contains("raw_provider_payload"));
    assert!(!sanitized_debug.contains("prompt"));
    assert!(!sanitized_debug.contains("secret"));
    assert!(result.semantic_analysis_status.contains("unavailable"));
    assert!(!result.mutated);
}

#[tokio::test]
async fn skill_curation_apply_run_requires_approval_and_policy_refs() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-run-apply-denied");

    let err = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: Vec::new(),
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect_err("apply curation run must be approval-gated");

    assert!(
        err.to_string().contains("approval refs"),
        "denial should explain the missing approval gate"
    );
}

#[tokio::test]
async fn skill_curation_snapshot_returns_governance_and_memento_refs_only() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-snapshot");
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
        .expect("usage observation should seed snapshot records");
    let run = provider
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
                audit_event_ids: Vec::new(),
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("curation run should create a replayable run ref");
    let run: SkillCurationRunResult =
        serde_json::from_value(run.output).expect("run result should decode");

    let snapshot = provider
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
        .expect("curation snapshot should be a typed service command");
    let snapshot: SkillCurationSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");

    assert!(!snapshot.mutated);
    assert_eq!(snapshot.snapshot.record_count, 1);
    assert!(snapshot.curation_run_refs.contains(&run.run.run_id));
    assert!(
        !serde_json::to_string(&snapshot)
            .expect("snapshot result should serialize")
            .contains("SKILL.md body"),
        "curation snapshot must not expose full skill instruction bodies"
    );
}
