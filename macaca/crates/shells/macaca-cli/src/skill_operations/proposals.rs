//! Skill draft proposal promote/reject command handler.
//!
//! Branches on the `promote` flag to select the correct SDK command or live
//! HTTP route without embedding proposal classification logic in the CLI.

use macaca_proto::{MacacaResult, TraceContext};
use macaca_sdk::{
    SkillEvolutionPromoteDraftCommand, SkillEvolutionRejectDraftCommand, SkillServiceScope,
    SystemSkillClient, UnavailableSystemSkillClient,
};
use tracing::info;

use super::output::{print_json, print_sdk_result};
use super::support::{live_operator_payload, optional_vec, policy_hints, url_segment};
use super::types::{SkillCliEvidenceRefs, SkillCliRuntimeTarget};

/// Promote or reject a draft proposal through the SDK Skill facade.
pub async fn execute_skill_proposal_decision(
    target: SkillCliRuntimeTarget,
    proposal_id: String,
    promote: bool,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let decision = if promote { "promote" } else { "reject" };
        let path = format!("/proposals/{}/{}", url_segment(&proposal_id), decision);
        let response = client.post(&path, live_operator_payload(refs)).await?;
        info!(
            app_id = %client.app_id,
            proposal_id = %proposal_id,
            decision,
            "CLI forwarded live Skill proposal decision through public Web API facade"
        );
        return print_json(response);
    }

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
