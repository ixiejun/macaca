//! Skill draft proposal promote/reject command handler.
//!
//! Branches on the `promote` flag to select the correct SDK command or live
//! HTTP route without embedding proposal classification logic in the CLI.

use macaca_proto::{MacacaResult, TraceContext};
use tracing::info;

use super::output::{print_json, print_unavailable};
use super::support::{live_operator_payload, url_segment};
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

    if promote {
        let trace = TraceContext::new("cli-skill-proposal-promote");
        info!(
            trace_id = %trace.trace_id,
            command = "skill.evolution.promote_draft",
            proposal_id = %proposal_id,
            "CLI Skill proposal promotion has no live Skill runtime target"
        );
        return print_unavailable(trace, "skill.evolution.promote_draft");
    }

    let trace = TraceContext::new("cli-skill-proposal-reject");
    info!(
        trace_id = %trace.trace_id,
        command = "skill.evolution.reject_draft",
        proposal_id = %proposal_id,
        "CLI Skill proposal rejection has no live Skill runtime target"
    );
    print_unavailable(trace, "skill.evolution.reject_draft")
}
