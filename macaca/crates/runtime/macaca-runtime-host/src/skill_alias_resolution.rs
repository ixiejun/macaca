//! Runtime-host Skill alias resolution helpers for autonomous execution.
//!
//! Scheduler, Heartbeat, and future Task dispatchers must keep their historical
//! skill references intact for audit replay.  This helper therefore treats alias
//! resolution as a just-in-time service boundary check: it calls `service.skill`,
//! records sanitized resolution metadata, and leaves the original caller fields
//! untouched.  The implementation is a small Strategy shared by autonomy lanes so
//! no shell, scheduler, or task provider owns Skill governance semantics.

use std::collections::BTreeMap;
use std::time::Duration;

use macaca_proto::{
    KernelServiceId, MacacaResult, ServiceBusSource, ServiceCommand, ServiceCommandName,
    TraceContext,
};
use macaca_skill::{
    SkillAliasKind, SkillAliasResolutionPolicy, SkillAliasResolutionStatus,
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillServiceScope,
    SKILL_ALIAS_RESOLVE_COMMAND, SKILL_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use crate::ServiceRuntime;

const REQUESTED_ID_KEY: &str = "skill.alias.requested_id";
const REQUESTED_NAME_KEY: &str = "skill.alias.requested_name";
const RESOLVED_KEY: &str = "skill.alias.resolved";
const STATUS_KEY: &str = "skill.alias.status";
const EFFECTIVE_ID_KEY: &str = "skill.alias.effective_id";
const EFFECTIVE_NAME_KEY: &str = "skill.alias.effective_name";
const KIND_KEY: &str = "skill.alias.kind";
const POLICY_KEY: &str = "skill.alias.policy";
const EVIDENCE_COUNT_KEY: &str = "skill.alias.evidence_count";

/// Resolve one metadata-declared Skill reference through `service.skill`.
///
/// Callers opt in by carrying `skill.alias.requested_id` and, optionally,
/// `skill.alias.requested_name`.  The helper writes only bounded decision facts
/// back into metadata.  It does not replace the requested id, copy rationale
/// text, or persist alias decisions outside the Skill service.
pub(crate) async fn resolve_skill_alias_metadata(
    runtime: &ServiceRuntime,
    trace: TraceContext,
    scope: SkillServiceScope,
    metadata: &mut BTreeMap<String, String>,
    dispatch_source: &'static str,
    timeout_ms: u64,
) -> MacacaResult<()> {
    let Some(requested_id) = requested_skill_id(metadata) else {
        return Ok(());
    };
    let requested_name = metadata
        .get(REQUESTED_NAME_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let command = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope,
        skill_id: requested_id.clone(),
        name: requested_name,
    };
    let service_command = ServiceCommand::with_trace(
        ServiceCommandName::new(SKILL_ALIAS_RESOLVE_COMMAND),
        serde_json::to_value(command)?,
        trace.clone(),
    );
    info!(
        service_id = SKILL_SERVICE_ID,
        requested_skill_id = requested_id.as_str(),
        dispatch_source,
        trace_id = trace.trace_id.as_str(),
        "skill alias resolution requested before autonomous execution"
    );
    match timeout(
        Duration::from_millis(timeout_ms.max(1)),
        runtime.call(
            &KernelServiceId::new(SKILL_SERVICE_ID),
            ServiceBusSource::new(dispatch_source),
            service_command,
        ),
    )
    .await
    {
        Ok(Ok(reply)) if reply.success => {
            let Some(output) = reply.output else {
                mark_unavailable(metadata, "empty_reply");
                return Ok(());
            };
            let result: SkillAliasResolveResult = serde_json::from_value(output)?;
            apply_resolution_metadata(metadata, &result);
            match &result.status {
                SkillAliasResolutionStatus::Redirected
                | SkillAliasResolutionStatus::WarnAndRedirected => {
                    info!(
                        service_id = SKILL_SERVICE_ID,
                        requested_skill_id = result.requested_skill_id.as_str(),
                        target_skill_id = result.target_skill_id.as_deref().unwrap_or("none"),
                        status = alias_status_label(&result.status),
                        dispatch_source,
                        trace_id = trace.trace_id.as_str(),
                        "skill alias hit before autonomous execution"
                    );
                }
                SkillAliasResolutionStatus::Miss => {
                    info!(
                        service_id = SKILL_SERVICE_ID,
                        requested_skill_id = result.requested_skill_id.as_str(),
                        dispatch_source,
                        trace_id = trace.trace_id.as_str(),
                        "skill alias miss before autonomous execution"
                    );
                }
                SkillAliasResolutionStatus::Denied
                | SkillAliasResolutionStatus::Expired
                | SkillAliasResolutionStatus::LoopPrevented => {
                    warn!(
                        service_id = SKILL_SERVICE_ID,
                        requested_skill_id = result.requested_skill_id.as_str(),
                        status = alias_status_label(&result.status),
                        dispatch_source,
                        trace_id = trace.trace_id.as_str(),
                        "skill alias guarded decision before autonomous execution"
                    );
                }
            }
        }
        Ok(Ok(reply)) => {
            warn!(
                service_id = SKILL_SERVICE_ID,
                status = reply.status.as_str(),
                requested_skill_id = requested_id.as_str(),
                dispatch_source,
                trace_id = trace.trace_id.as_str(),
                "skill alias resolution returned unsuccessful reply"
            );
            mark_unavailable(metadata, reply.status.as_str());
        }
        Ok(Err(error)) => {
            warn!(
                service_id = SKILL_SERVICE_ID,
                error = %error,
                requested_skill_id = requested_id.as_str(),
                dispatch_source,
                trace_id = trace.trace_id.as_str(),
                "skill alias resolution failed before autonomous execution"
            );
            mark_unavailable(metadata, "service_error");
        }
        Err(_) => {
            warn!(
                service_id = SKILL_SERVICE_ID,
                requested_skill_id = requested_id.as_str(),
                dispatch_source,
                trace_id = trace.trace_id.as_str(),
                "skill alias resolution timed out before autonomous execution"
            );
            mark_unavailable(metadata, "timeout");
        }
    }
    Ok(())
}

fn requested_skill_id(metadata: &BTreeMap<String, String>) -> Option<String> {
    metadata
        .get(REQUESTED_ID_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn apply_resolution_metadata(
    metadata: &mut BTreeMap<String, String>,
    result: &SkillAliasResolveResult,
) {
    metadata.insert(RESOLVED_KEY.into(), result.resolved.to_string());
    metadata.insert(STATUS_KEY.into(), alias_status_label(&result.status).into());
    if let Some(target_skill_id) = result.target_skill_id.as_ref() {
        metadata.insert(EFFECTIVE_ID_KEY.into(), target_skill_id.clone());
    }
    if let Some(target_name) = result.target_name.as_ref() {
        metadata.insert(EFFECTIVE_NAME_KEY.into(), target_name.clone());
    }
    if let Some(kind) = result.kind.as_ref() {
        metadata.insert(KIND_KEY.into(), alias_kind_label(kind).into());
    }
    if let Some(policy) = result.resolution_policy.as_ref() {
        metadata.insert(POLICY_KEY.into(), alias_policy_label(policy).into());
    }
    metadata.insert(
        EVIDENCE_COUNT_KEY.into(),
        result.evidence_ids.len().to_string(),
    );
}

fn mark_unavailable(metadata: &mut BTreeMap<String, String>, reason: &str) {
    metadata.insert(RESOLVED_KEY.into(), "false".into());
    metadata.insert(STATUS_KEY.into(), format!("unavailable:{reason}"));
}

fn alias_status_label(status: &SkillAliasResolutionStatus) -> &'static str {
    match status {
        SkillAliasResolutionStatus::Miss => "miss",
        SkillAliasResolutionStatus::Redirected => "redirected",
        SkillAliasResolutionStatus::WarnAndRedirected => "warn_and_redirected",
        SkillAliasResolutionStatus::Denied => "denied",
        SkillAliasResolutionStatus::Expired => "expired",
        SkillAliasResolutionStatus::LoopPrevented => "loop_prevented",
    }
}

fn alias_kind_label(kind: &SkillAliasKind) -> &'static str {
    match kind {
        SkillAliasKind::Redirect => "redirect",
        SkillAliasKind::SupersededBy => "superseded_by",
        SkillAliasKind::AbsorbedInto => "absorbed_into",
    }
}

fn alias_policy_label(policy: &SkillAliasResolutionPolicy) -> &'static str {
    match policy {
        SkillAliasResolutionPolicy::Redirect => "redirect",
        SkillAliasResolutionPolicy::WarnAndRedirect => "warn_and_redirect",
        SkillAliasResolutionPolicy::Deny => "deny",
    }
}
