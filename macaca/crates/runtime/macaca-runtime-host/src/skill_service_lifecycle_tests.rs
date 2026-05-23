use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasSnapshotCommand, SkillAliasSnapshotResult,
    SkillAuthorKind, SkillCurationLifecycleCommand, SkillCurationLifecycleResult,
    SkillCurationSupersedeCommand, SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult,
    SkillLifecycleState, SkillServicePolicyHints, SkillServiceScope, SKILL_ALIAS_SNAPSHOT_COMMAND,
    SKILL_CURATION_ARCHIVE_COMMAND, SKILL_CURATION_PIN_COMMAND, SKILL_CURATION_QUARANTINE_COMMAND,
    SKILL_CURATION_REJECT_COMMAND, SKILL_CURATION_RELEASE_QUARANTINE_COMMAND,
    SKILL_CURATION_RESTORE_COMMAND, SKILL_CURATION_SUPERSEDE_COMMAND, SKILL_CURATION_UNPIN_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
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

fn lifecycle_command(
    trace: TraceContext,
    evidence_ids: Vec<String>,
) -> SkillCurationLifecycleCommand {
    SkillCurationLifecycleCommand {
        trace,
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/lifecycle".into(),
        name: "agent-lifecycle".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        author_kind: SkillAuthorKind::Agent,
        reason: "operator requested a metadata-only lifecycle transition".into(),
        evidence_ids,
        task_id: Some("task-lifecycle-test".into()),
        policy_decision_refs: vec!["policy://decision/lifecycle-test".into()],
        policy: SkillServicePolicyHints::default(),
    }
}

fn supersede_alias_record() -> SkillAliasRecord {
    let now = chrono::Utc::now();
    SkillAliasRecord {
        source_skill_id: "skill://agent/lifecycle".into(),
        source_name: "agent-lifecycle".into(),
        target_skill_id: "skill://agent/lifecycle-v2".into(),
        target_name: "agent-lifecycle-v2".into(),
        kind: SkillAliasKind::SupersededBy,
        rationale: "operator approved redirect before hiding the source skill".into(),
        created_at: now,
        updated_at: now,
        evidence_ids: vec!["supersede-alias-evidence-1".into()],
    }
}

#[tokio::test]
async fn skill_curation_pin_and_unpin_mutate_only_governance_metadata() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-pin-unpin");
    let pin = lifecycle_command(trace.clone(), vec!["pin-evidence-1".into()]);

    let result = provider
        .call(traced_command(
            SKILL_CURATION_PIN_COMMAND,
            pin,
            trace.clone(),
        ))
        .await
        .expect("pin command should update governance metadata");
    let pinned: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("pin result should decode");
    assert!(pinned.mutated);
    assert!(pinned.pinned);
    assert_eq!(pinned.lifecycle, SkillLifecycleState::Active);
    assert_eq!(pinned.evidence_ids, vec!["pin-evidence-1"]);
    assert_eq!(
        pinned.policy_decision_refs,
        vec!["policy://decision/lifecycle-test"]
    );

    let unpin = lifecycle_command(trace.clone(), vec!["unpin-evidence-1".into()]);
    let result = provider
        .call(traced_command(
            SKILL_CURATION_UNPIN_COMMAND,
            unpin,
            trace.clone(),
        ))
        .await
        .expect("unpin command should update governance metadata");
    let unpinned: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("unpin result should decode");
    assert!(!unpinned.pinned);

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("governance snapshot should include lifecycle record");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("snapshot result should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert!(!snapshot.records[0].pinned);
    assert_eq!(snapshot.records[0].evidence_ids.len(), 2);
}

#[tokio::test]
async fn skill_curation_archive_and_restore_update_snapshot_visibility() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-archive-restore");
    let archive = lifecycle_command(trace.clone(), vec!["archive-evidence-1".into()]);

    let result = provider
        .call(traced_command(
            SKILL_CURATION_ARCHIVE_COMMAND,
            archive,
            trace.clone(),
        ))
        .await
        .expect("archive command should update governance lifecycle");
    let archived: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("archive result should decode");
    assert_eq!(archived.lifecycle, SkillLifecycleState::Archived);
    assert!(!archived.pinned);

    let filtered = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            filtered,
            trace.clone(),
        ))
        .await
        .expect("filtered snapshot should succeed");
    let filtered: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("filtered snapshot should decode");
    assert!(filtered.records.is_empty());

    let restore = lifecycle_command(trace.clone(), vec!["restore-evidence-1".into()]);
    let result = provider
        .call(traced_command(
            SKILL_CURATION_RESTORE_COMMAND,
            restore,
            trace.clone(),
        ))
        .await
        .expect("restore command should update governance lifecycle");
    let restored: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("restore result should decode");
    assert_eq!(restored.lifecycle, SkillLifecycleState::Active);

    let visible = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            visible,
            trace,
        ))
        .await
        .expect("restored record should be visible");
    let visible: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("visible snapshot should decode");
    assert_eq!(visible.records.len(), 1);
    assert_eq!(visible.records[0].lifecycle, SkillLifecycleState::Active);
}

#[tokio::test]
async fn skill_curation_quarantine_and_release_update_lifecycle() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-quarantine-release");
    let quarantine = lifecycle_command(trace.clone(), vec!["quarantine-evidence-1".into()]);

    let result = provider
        .call(traced_command(
            SKILL_CURATION_QUARANTINE_COMMAND,
            quarantine,
            trace.clone(),
        ))
        .await
        .expect("quarantine command should update governance lifecycle");
    let quarantined: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("quarantine result should decode");
    assert_eq!(quarantined.lifecycle, SkillLifecycleState::Quarantined);

    let release = lifecycle_command(trace.clone(), vec!["release-quarantine-evidence-1".into()]);
    let result = provider
        .call(traced_command(
            SKILL_CURATION_RELEASE_QUARANTINE_COMMAND,
            release,
            trace,
        ))
        .await
        .expect("release quarantine command should restore active lifecycle");
    let released: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("release result should decode");
    assert_eq!(released.lifecycle, SkillLifecycleState::Active);
}

#[tokio::test]
async fn skill_curation_supersede_requires_and_records_alias() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-supersede");
    let command = SkillCurationSupersedeCommand {
        lifecycle: lifecycle_command(trace.clone(), vec!["supersede-evidence-1".into()]),
        alias: supersede_alias_record(),
    };

    let result = provider
        .call(traced_command(
            SKILL_CURATION_SUPERSEDE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("supersede command should update lifecycle after alias upsert");
    let superseded: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("supersede result should decode");
    assert_eq!(superseded.lifecycle, SkillLifecycleState::Superseded);
    assert_eq!(superseded.evidence_ids, vec!["supersede-evidence-1"]);

    let alias_snapshot = SkillAliasSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            alias_snapshot,
            trace.clone(),
        ))
        .await
        .expect("alias snapshot should expose supersede redirect");
    let aliases: SkillAliasSnapshotResult =
        serde_json::from_value(result.output).expect("alias snapshot should decode");
    assert_eq!(aliases.aliases.len(), 1);
    assert_eq!(aliases.aliases[0].kind, SkillAliasKind::SupersededBy);

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: vec![SkillLifecycleState::Superseded],
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("filtered snapshot should include superseded record");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("snapshot result should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(
        snapshot.records[0].lifecycle,
        SkillLifecycleState::Superseded
    );
}

#[tokio::test]
async fn skill_curation_reject_preserves_rationale_and_evidence() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-reject");
    let command = lifecycle_command(trace.clone(), vec!["reject-evidence-1".into()]);

    let result = provider
        .call(traced_command(
            SKILL_CURATION_REJECT_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("reject command should preserve draft rationale as rejected lifecycle");
    let rejected: SkillCurationLifecycleResult =
        serde_json::from_value(result.output).expect("reject result should decode");
    assert_eq!(rejected.lifecycle, SkillLifecycleState::Rejected);
    assert_eq!(rejected.evidence_ids, vec!["reject-evidence-1"]);
    assert_eq!(
        rejected.policy_decision_refs,
        vec!["policy://decision/lifecycle-test"]
    );
    assert!(rejected
        .reason
        .contains("metadata-only lifecycle transition"));
}

#[tokio::test]
async fn skill_curation_archive_rejects_pinned_skill() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-pinned-archive");
    let pin = lifecycle_command(trace.clone(), vec!["pin-protection-1".into()]);
    provider
        .call(traced_command(
            SKILL_CURATION_PIN_COMMAND,
            pin,
            trace.clone(),
        ))
        .await
        .expect("pin command should succeed");

    let archive = lifecycle_command(trace.clone(), vec!["archive-denied-1".into()]);
    let err = provider
        .call(traced_command(
            SKILL_CURATION_ARCHIVE_COMMAND,
            archive,
            trace.clone(),
        ))
        .await
        .expect_err("archive of pinned skill should be denied");
    assert!(
        err.to_string().contains("pinned"),
        "denial should explain pinned protection"
    );

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("snapshot should remain available after denied archive");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("snapshot result should decode");
    assert!(snapshot.records[0].pinned);
    assert_eq!(snapshot.records[0].lifecycle, SkillLifecycleState::Active);
}

#[tokio::test]
async fn skill_curation_lifecycle_rejects_missing_evidence() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-missing-evidence");
    let command = lifecycle_command(trace.clone(), Vec::new());

    let err = provider
        .call(traced_command(SKILL_CURATION_PIN_COMMAND, command, trace))
        .await
        .expect_err("lifecycle mutation without evidence should be rejected");

    assert!(
        err.to_string().contains("evidence"),
        "validation error should explain the missing evidence"
    );
}

#[tokio::test]
async fn skill_curation_lifecycle_rejects_missing_policy_decision_ref() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-missing-policy");
    let mut command = lifecycle_command(trace.clone(), vec!["pin-evidence-1".into()]);
    command.policy_decision_refs.clear();

    let err = provider
        .call(traced_command(SKILL_CURATION_PIN_COMMAND, command, trace))
        .await
        .expect_err("lifecycle mutation must require policy decision refs");
    assert!(
        err.to_string()
            .contains("requires policy decision references"),
        "unexpected error: {err}"
    );
}
