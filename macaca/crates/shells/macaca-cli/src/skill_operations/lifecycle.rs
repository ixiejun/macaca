//! Skill lifecycle mutation command handler.
//!
//! Maps CLI lifecycle verbs to SDK commands or live HTTP POSTs.  Policy and
//! state transitions are enforced by `service.skill`; the shell only forwards
//! operator evidence and trace metadata.

use macaca_proto::{MacacaResult, TraceContext};
use tracing::info;

use super::output::{print_json, print_unavailable};
use super::support::{live_operator_payload, url_segment};
use super::types::{SkillCliEvidenceRefs, SkillCliLifecycleAction, SkillCliRuntimeTarget};

/// Forward one lifecycle mutation request through the SDK Skill facade.
pub async fn execute_skill_lifecycle(
    target: SkillCliRuntimeTarget,
    action: SkillCliLifecycleAction,
    skill_id: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let path = format!(
            "/lifecycle/{}/{}",
            action.route_segment(),
            url_segment(&skill_id)
        );
        let response = client.post(&path, live_operator_payload(refs)).await?;
        info!(
            app_id = %client.app_id,
            action = action.as_str(),
            skill_id = %skill_id,
            "CLI forwarded live Skill lifecycle command through public Web API facade"
        );
        return print_json(response);
    }

    let trace = TraceContext::new(format!("cli-skill-lifecycle-{}", action.as_str()));
    info!(
        trace_id = %trace.trace_id,
        command = "skill.curation.lifecycle",
        action = action.as_str(),
        skill_id = %skill_id,
        "CLI Skill lifecycle command has no live Skill runtime target"
    );
    print_unavailable(trace, "skill.curation.lifecycle")
}
