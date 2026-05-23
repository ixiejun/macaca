use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasSnapshotCommand, SkillAliasSnapshotResult,
    SkillAuthorKind, SkillCurationRollbackCommand, SkillCurationRollbackResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillLifecycleState, SkillMergeAliasEffect,
    SkillMergeApplyResult, SkillMergeRiskScore, SkillServiceScope, SkillUmbrellaMergeApplyCommand,
    SkillUmbrellaMergeProposal, SkillUsageEventKind, SkillUsageObservation,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_CURATION_MERGE_APPLY_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
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

fn source_observation(skill_id: &str, name: &str, pinned: Option<bool>) -> SkillUsageObservation {
    SkillUsageObservation {
        skill_id: skill_id.into(),
        name: name.into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        event: SkillUsageEventKind::Created,
        author_kind: SkillAuthorKind::Agent,
        created_by: Some("agent".into()),
        pinned,
        evidence_id: Some("evidence://merge/source-created".into()),
        metadata: BTreeMap::new(),
    }
}

fn approved_merge_apply(trace: TraceContext) -> SkillUmbrellaMergeApplyCommand {
    let proposal = SkillUmbrellaMergeProposal {
        proposal_id: "merge://proposal/debugging-umbrella".into(),
        source_skill_ids: vec!["skill://agent/debug-heartbeat".into()],
        target_umbrella_skill_id: "skill://agent/debugging".into(),
        support_file_movements: Vec::new(),
        alias_effects: vec![SkillMergeAliasEffect {
            source_skill_id: "skill://agent/debug-heartbeat".into(),
            source_name: "debug-heartbeat".into(),
            target_skill_id: "skill://agent/debugging".into(),
            target_name: "debugging".into(),
            kind: SkillAliasKind::AbsorbedInto,
            rationale: "source skill was absorbed into the generic debugging umbrella".into(),
            evidence_ids: vec!["evidence://merge/alias".into()],
        }],
        risk: SkillMergeRiskScore {
            score: 0.25,
            reasons: vec!["compatible generic debugging flow".into()],
        },
        policy_decision_refs: vec!["policy://merge/proposal-compatible".into()],
        evidence_ids: vec!["evidence://merge/proposal".into()],
        rationale: "merge keeps the reusable debugging flow under the umbrella skill".into(),
    };

    SkillUmbrellaMergeApplyCommand {
        trace,
        scope: SkillServiceScope::default(),
        proposal,
        approval_refs: vec!["approval://merge/apply".into()],
        policy_decision_refs: vec!["policy://merge/apply".into()],
        lifecycle_transition_refs: vec!["skill.curation.supersede://debug-heartbeat".into()],
        safe_mutation_refs: vec!["skill.content.mutate://support-file-plan".into()],
        audit_event_ids: vec!["audit://merge/apply".into()],
    }
}

#[tokio::test]
async fn skill_curation_merge_apply_supersedes_absorbed_sources_and_records_aliases() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-merge-apply");

    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: source_observation(
                    "skill://agent/debug-heartbeat",
                    "debug-heartbeat",
                    None,
                ),
            },
            trace.clone(),
        ))
        .await
        .expect("source skill should exist before merge apply");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_MERGE_APPLY_COMMAND,
            approved_merge_apply(trace.clone()),
            trace.clone(),
        ))
        .await
        .expect("approved merge apply should be admitted by the Skill service");
    let result: SkillMergeApplyResult =
        serde_json::from_value(result.output).expect("merge apply result should decode");

    assert!(result.mutated);
    assert_eq!(
        result.superseded_skill_ids,
        vec!["skill://agent/debug-heartbeat"]
    );
    assert_eq!(result.alias_records.len(), 1);
    assert!(result
        .report_ref
        .as_deref()
        .unwrap_or("")
        .contains("REPORT.md"));
    assert!(result
        .bounded_report_summary
        .contains("merge keeps the reusable debugging flow"));
    assert_eq!(result.evidence_ids, vec!["evidence://merge/proposal"]);
    assert!(result
        .rollback_ref
        .as_deref()
        .unwrap_or("")
        .contains("rollback"));

    let governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: vec![SkillLifecycleState::Superseded],
            },
            trace.clone(),
        ))
        .await
        .expect("governance snapshot should expose superseded source");
    let governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(governance.output).expect("governance snapshot should decode");
    assert_eq!(governance.records.len(), 1);
    assert_eq!(
        governance.records[0].lifecycle,
        SkillLifecycleState::Superseded
    );

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
        .expect("alias snapshot should expose absorbed source redirect");
    let aliases: SkillAliasSnapshotResult =
        serde_json::from_value(aliases.output).expect("alias snapshot should decode");
    assert_eq!(aliases.aliases.len(), 1);
    assert_eq!(
        aliases.aliases[0],
        SkillAliasRecord {
            source_skill_id: "skill://agent/debug-heartbeat".into(),
            source_name: "debug-heartbeat".into(),
            target_skill_id: "skill://agent/debugging".into(),
            target_name: "debugging".into(),
            kind: SkillAliasKind::AbsorbedInto,
            rationale: "source skill was absorbed into the generic debugging umbrella".into(),
            created_at: aliases.aliases[0].created_at,
            updated_at: aliases.aliases[0].updated_at,
            evidence_ids: vec!["evidence://merge/alias".into()],
        }
    );
}

#[tokio::test]
async fn skill_curation_merge_apply_rejects_pinned_absorbed_source() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-merge-pinned");

    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: source_observation(
                    "skill://agent/debug-heartbeat",
                    "debug-heartbeat",
                    Some(true),
                ),
            },
            trace.clone(),
        ))
        .await
        .expect("pinned source skill should exist before merge apply");

    let err = provider
        .call(traced_command(
            SKILL_CURATION_MERGE_APPLY_COMMAND,
            approved_merge_apply(trace.clone()),
            trace,
        ))
        .await
        .expect_err("merge apply must preserve pinned source protection");

    assert!(
        err.to_string().contains("pinned skill cannot be supersede"),
        "pinned merge denial should come from the shared lifecycle guard"
    );
}

#[tokio::test]
async fn skill_curation_merge_apply_rollback_restores_lifecycle_and_alias_map() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-merge-rollback");

    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: source_observation(
                    "skill://agent/debug-heartbeat",
                    "debug-heartbeat",
                    None,
                ),
            },
            trace.clone(),
        ))
        .await
        .expect("source skill should exist before merge apply");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_MERGE_APPLY_COMMAND,
            approved_merge_apply(trace.clone()),
            trace.clone(),
        ))
        .await
        .expect("approved merge apply should create a rollback memento");
    let result: SkillMergeApplyResult =
        serde_json::from_value(result.output).expect("merge apply result should decode");
    let rollback_ref = result
        .rollback_ref
        .clone()
        .expect("merge apply should expose rollback ref");

    let rollback = provider
        .call(traced_command(
            SKILL_CURATION_ROLLBACK_COMMAND,
            SkillCurationRollbackCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                rollback_ref,
                approval_refs: vec!["approval://merge/rollback".into()],
                policy_decision_refs: vec!["policy://merge/rollback".into()],
                audit_event_ids: vec!["audit://merge/rollback".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("rollback command should restore pre-merge governance");
    let rollback: SkillCurationRollbackResult =
        serde_json::from_value(rollback.output).expect("rollback result should decode");
    assert!(rollback.mutated);
    assert_eq!(rollback.restored_alias_count, 0);

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
        .expect("governance snapshot should show restored active source");
    let governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(governance.output).expect("governance snapshot should decode");
    assert_eq!(governance.records[0].lifecycle, SkillLifecycleState::Active);

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
        .expect("alias snapshot should show restored pre-merge alias map");
    let aliases: SkillAliasSnapshotResult =
        serde_json::from_value(aliases.output).expect("alias snapshot should decode");
    assert!(aliases.aliases.is_empty());
}
