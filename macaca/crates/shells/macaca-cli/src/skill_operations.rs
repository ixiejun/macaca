//! CLI adapters for Skill governance and curation operations.
//!
//! The CLI is a presentation shell, so these commands build provider-neutral
//! SDK DTOs, call the focused Skill client, and print bounded JSON summaries.
//! They do not read skill packages, classify curation actions, or implement
//! lifecycle policy locally.

use std::collections::BTreeMap;

use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use macaca_sdk::{
    SkillAuthorKind, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationRollbackCommand, SkillCurationRunCommand, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionRejectDraftCommand, SkillExperienceProposalSnapshotCommand,
    SkillGovernanceSnapshotCommand, SkillServicePolicyHints, SkillServiceScope, SystemSkillClient,
    UnavailableSystemSkillClient,
};
use tracing::{info, warn};

/// Shared operator evidence refs accepted by mutating CLI commands.
#[derive(Debug, Clone, Default)]
pub struct SkillCliEvidenceRefs {
    pub reason: Option<String>,
    pub evidence_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub approval_ref: Option<String>,
}

/// Lifecycle actions exposed by the CLI transport.
#[derive(Debug, Clone, Copy)]
pub enum SkillCliLifecycleAction {
    Pin,
    Unpin,
    Archive,
    Restore,
    Quarantine,
    ReleaseQuarantine,
    Reject,
}

impl SkillCliLifecycleAction {
    fn into_service_action(self) -> SkillCurationLifecycleAction {
        match self {
            Self::Pin => SkillCurationLifecycleAction::Pin,
            Self::Unpin => SkillCurationLifecycleAction::Unpin,
            Self::Archive => SkillCurationLifecycleAction::Archive,
            Self::Restore => SkillCurationLifecycleAction::Restore,
            Self::Quarantine => SkillCurationLifecycleAction::Quarantine,
            Self::ReleaseQuarantine => SkillCurationLifecycleAction::ReleaseQuarantine,
            Self::Reject => SkillCurationLifecycleAction::Reject,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Archive => "archive",
            Self::Restore => "restore",
            Self::Quarantine => "quarantine",
            Self::ReleaseQuarantine => "release_quarantine",
            Self::Reject => "reject",
        }
    }
}

/// Print a sanitized Skill operations snapshot through the SDK Skill facade.
pub async fn execute_skill_operations_snapshot() -> MacacaResult<()> {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("cli-skill-operations-snapshot");
    let scope = SkillServiceScope::default();
    info!(
        trace_id = %trace.trace_id,
        command = "skill.operations.snapshot",
        "CLI forwarding Skill operations snapshot through SDK Skill facade"
    );
    let governance = client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            include_archived: true,
            lifecycle_filters: Vec::new(),
        })
        .await?;
    let proposals = client
        .skill_experience_snapshot(SkillExperienceProposalSnapshotCommand {
            trace: trace.clone(),
            scope,
            include_discarded: false,
        })
        .await?;
    info!(
        trace_id = %trace.trace_id,
        governance_records = governance.records.len(),
        proposal_count = proposals.proposals.len(),
        "CLI emitted bounded Skill operations snapshot"
    );
    print_json(serde_json::json!({
        "trace_id": trace.trace_id,
        "governance_records": governance.records.len(),
        "proposal_count": proposals.proposals.len(),
        "governance": governance,
        "proposals": proposals,
    }))
}

/// Run deterministic curation analysis or approval-gated apply through SDK.
pub async fn execute_skill_curation_run(
    dry_run: bool,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new(if dry_run {
        "cli-skill-curation-run"
    } else {
        "cli-skill-curation-apply"
    });
    let command = SkillCurationRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        dry_run,
        stale_after_days,
        narrow_use_threshold,
        approval_refs: optional_vec(refs.approval_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = if dry_run { "skill.curation.run" } else { "skill.curation.apply" },
        dry_run,
        stale_after_days,
        narrow_use_threshold,
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill curation command through SDK Skill facade"
    );
    print_sdk_result(trace, client.curation_run(command).await)
}

/// Forward one lifecycle mutation request through the SDK Skill facade.
pub async fn execute_skill_lifecycle(
    action: SkillCliLifecycleAction,
    skill_id: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new(format!("cli-skill-lifecycle-{}", action.as_str()));
    let command = SkillCurationLifecycleCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: skill_id.clone(),
        name: skill_id,
        source: "cli-skill-operations".into(),
        source_scope: "operator".into(),
        author_kind: SkillAuthorKind::Unknown,
        reason: refs
            .reason
            .unwrap_or_else(|| "cli_skill_lifecycle_request".into()),
        evidence_ids: optional_vec(refs.evidence_ref),
        task_id: None,
        policy_decision_refs: optional_vec(refs.policy_ref),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.curation.lifecycle",
        action = action.as_str(),
        skill_id = %command.skill_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill lifecycle command through SDK Skill facade"
    );
    print_sdk_result(
        trace,
        client
            .curation_lifecycle(action.into_service_action(), command)
            .await,
    )
}

/// Restore governance state from a curation rollback memento ref through SDK.
pub async fn execute_skill_rollback(
    rollback_ref: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("cli-skill-curation-rollback");
    let command = SkillCurationRollbackCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        rollback_ref,
        approval_refs: optional_vec(refs.approval_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.curation.rollback",
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill rollback command through SDK Skill facade"
    );
    print_sdk_result(trace, client.curation_rollback(command).await)
}

/// Promote or reject a draft proposal through the SDK Skill facade.
pub async fn execute_skill_proposal_decision(
    proposal_id: String,
    promote: bool,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    let client = UnavailableSystemSkillClient;
    if promote {
        let trace = TraceContext::new("cli-skill-proposal-promote");
        let command = SkillEvolutionPromoteDraftCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            proposal_id,
            reason: refs
                .reason
                .unwrap_or_else(|| "cli_skill_proposal_promote".into()),
            evidence_ids: optional_vec(refs.evidence_ref),
            policy_decision_refs: optional_vec(refs.policy_ref),
            policy: policy_hints(),
        };
        info!(
            trace_id = %trace.trace_id,
            command = "skill.evolution.promote_draft",
            proposal_id = %command.proposal_id,
            evidence_count = command.evidence_ids.len(),
            policy_decision_count = command.policy_decision_refs.len(),
            "CLI forwarding Skill proposal promotion through SDK Skill facade"
        );
        return print_sdk_result(trace, client.promote_skill_draft(command).await);
    }

    let trace = TraceContext::new("cli-skill-proposal-reject");
    let command = SkillEvolutionRejectDraftCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        proposal_id,
        rationale: refs
            .reason
            .unwrap_or_else(|| "cli_skill_proposal_reject".into()),
        evidence_ids: optional_vec(refs.evidence_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.evolution.reject_draft",
        proposal_id = %command.proposal_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill proposal rejection through SDK Skill facade"
    );
    print_sdk_result(trace, client.reject_skill_draft(command).await)
}

fn optional_vec(value: Option<String>) -> Vec<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .into_iter()
        .collect()
}

fn policy_hints() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: Vec::new(),
        entitlement_ready: None,
        package_ready: None,
        metadata: BTreeMap::new(),
    }
}

fn print_sdk_result<T: serde::Serialize>(
    trace: TraceContext,
    result: MacacaResult<T>,
) -> MacacaResult<()> {
    match result {
        Ok(result) => print_json(serde_json::json!({
            "trace_id": trace.trace_id,
            "status": "ok",
            "result": result,
        })),
        Err(error) => {
            warn!(
                trace_id = %trace.trace_id,
                error_class = "unavailable_or_denied",
                "CLI Skill command returned structured service error"
            );
            print_json(serde_json::json!({
                "trace_id": trace.trace_id,
                "status": "unavailable_or_denied",
                "error": error.to_string(),
            }))
        }
    }
}

fn print_json(value: serde_json::Value) -> MacacaResult<()> {
    let rendered = serde_json::to_string_pretty(&value).map_err(MacacaError::from)?;
    println!("{rendered}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn cli_skill_snapshot_uses_sdk_null_object() {
        super::execute_skill_operations_snapshot().await.unwrap();
    }

    #[test]
    fn cli_skill_operations_do_not_import_runtime_or_web() {
        let source = include_str!("skill_operations.rs");
        let runtime_host_import = ["macaca", "_runtime_host::"].concat();
        let web_import = ["macaca", "_web::"].concat();
        let provider_state_symbol = ["SkillProvider", "GovernanceState"].concat();
        assert!(source.contains("UnavailableSystemSkillClient"));
        assert!(!source.contains(&runtime_host_import));
        assert!(!source.contains(&web_import));
        assert!(!source.contains(&provider_state_symbol));
    }
}
