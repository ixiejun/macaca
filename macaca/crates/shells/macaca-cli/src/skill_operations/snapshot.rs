//! Skill operations snapshot command handler.
//!
//! Dual-path execution:
//! 1. Live path — GET the public Web facade when `SkillCliRuntimeTarget` carries an app id.
//! 2. Diagnostic path — aggregate governance + proposal snapshots via SDK Null Object.

use macaca_proto::{MacacaResult, TraceContext};
use macaca_sdk::{
    SkillExperienceProposalSnapshotCommand, SkillGovernanceSnapshotCommand,
    SkillServiceScope, SystemSkillClient, UnavailableSystemSkillClient,
};
use tracing::info;

use super::output::print_json;
use super::types::SkillCliRuntimeTarget;

/// Print a sanitized Skill operations snapshot through the SDK Skill facade.
pub async fn execute_skill_operations_snapshot(target: SkillCliRuntimeTarget) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client.get("").await?;
        info!(
            app_id = %client.app_id,
            "CLI emitted live Skill operations snapshot from public Web API facade"
        );
        return print_json(response);
    }

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
