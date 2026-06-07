//! Skill curation run/apply and rollback command handlers.
//!
//! Forwards threshold knobs and operator evidence to `service.skill` without
//! classifying stale or narrow-use skills in the CLI layer.

use macaca_proto::{MacacaResult, TraceContext};
use macaca_sdk::{
    SkillCurationRollbackCommand, SkillCurationRunCommand, SkillServiceScope,
    SystemSkillClient, UnavailableSystemSkillClient,
};
use tracing::info;

use super::output::{print_json, print_sdk_result};
use super::support::{live_operator_payload, optional_vec, policy_hints};
use super::types::{SkillCliEvidenceRefs, SkillCliRuntimeTarget};

/// Run deterministic curation analysis or approval-gated apply through SDK.
pub async fn execute_skill_curation_run(
    target: SkillCliRuntimeTarget,
    dry_run: bool,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let trace_label = if dry_run {
            "cli-skill-curation-run"
        } else {
            "cli-skill-curation-apply"
        };
        let response = client
            .post(
                if dry_run {
                    "/curation/run"
                } else {
                    "/curation/apply"
                },
                live_operator_payload(refs)
                    .with_curation_thresholds(stale_after_days, narrow_use_threshold),
            )
            .await?;
        info!(
            app_id = %client.app_id,
            command = trace_label,
            dry_run,
            "CLI forwarded live Skill curation command through public Web API facade"
        );
        return print_json(response);
    }

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

/// Restore governance state from a curation rollback memento ref through SDK.
pub async fn execute_skill_rollback(
    target: SkillCliRuntimeTarget,
    rollback_ref: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client
            .post(
                "/curation/rollback",
                live_operator_payload(refs).with_rollback_ref(rollback_ref.clone()),
            )
            .await?;
        info!(
            app_id = %client.app_id,
            rollback_ref = %rollback_ref,
            "CLI forwarded live Skill rollback command through public Web API facade"
        );
        return print_json(response);
    }

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
