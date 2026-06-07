//! Skill lifecycle mutation command handler.
//!
//! Maps CLI lifecycle verbs to SDK commands or live HTTP POSTs.  Policy and
//! state transitions are enforced by `service.skill`; the shell only forwards
//! operator evidence and trace metadata.

use macaca_proto::{MacacaResult, TraceContext};
use macaca_sdk::{
    SkillAuthorKind, SkillCurationLifecycleCommand, SkillServiceScope, SystemSkillClient,
    UnavailableSystemSkillClient,
};
use tracing::info;

use super::output::{print_json, print_sdk_result};
use super::support::{live_operator_payload, optional_vec, policy_hints, url_segment};
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
