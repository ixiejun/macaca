//! Shared **Adapter** helpers for skill-operations HTTP routes.
//!
//! Converts transport-layer identifiers into SDK scope objects, assembles policy
//! hints from operator payloads, and resolves materialization package roots from
//! workspace configuration (never from projection output directories).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::http::StatusCode;
use axum::Json;
use macaca_host_composition::runtime_host::{
    SkillCurationLifecycleAction, SkillCurationRunCommand, SkillServicePolicyHints,
    SkillServiceScope,
};
use macaca_proto::{ApplicationId, TraceContext};

use crate::routes::{err, ErrorResponse};
use crate::skill_operations_routes::types::SkillOperatorCommandRequest;
use crate::state::AppState;

/// Parse a UUID application id from the URL path segment.
pub(crate) fn parse_application_id(
    app_id: &str,
) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

/// Build the Skill service scope for a single application (no session/agent binding).
pub(crate) fn application_skill_scope(app_id: ApplicationId) -> SkillServiceScope {
    SkillServiceScope {
        application_id: Some(app_id),
        session_id: None,
        tenant_id: None,
        agent_name: None,
    }
}

/// Resolve the package collection root used by autonomous materialization.
///
/// `available_skills/` is a projection output produced by the Skill runtime for
/// model prompt locations. It is not a discovery source. When Web invokes the
/// service-owned operator without an explicit package root, this adapter selects
/// the workspace `skills/` source so a materialized package can be discovered by
/// later `SkillRuntime` snapshots and then projected into `available_skills/`.
pub(crate) async fn materialization_collection_root(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    explicit_root: Option<&PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(root) = explicit_root {
        return Ok(root.clone());
    }
    let workspaces = state.config.app_workspaces.read().await;
    let workspace = workspaces.get(app_id).ok_or_else(|| {
        "package_collection_root is required when the app workspace is unavailable".to_string()
    })?;
    Ok(workspace.root.join("skills"))
}

/// Test helper: derive default materialization root from a workspace path.
#[cfg(test)]
pub(crate) fn materialization_collection_root_from_workspace_path(
    workspace_root: &Path,
) -> PathBuf {
    workspace_root.join("skills")
}

/// Forward a curation run or apply command through the Skill SDK client.
pub(crate) async fn run_curation(
    state: Arc<AppState>,
    app_id: String,
    body: SkillOperatorCommandRequest,
    dry_run: bool,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!(
        "web-skill-curation-{}-{}",
        if dry_run { "run" } else { "apply" },
        app_id.0
    ));
    let command = SkillCurationRunCommand {
        trace: trace.clone(),
        scope: application_skill_scope(app_id),
        dry_run,
        stale_after_days: body.stale_after_days.unwrap_or(30),
        narrow_use_threshold: body.narrow_use_threshold.unwrap_or(1),
        approval_refs: body.approval_refs,
        policy_decision_refs: body.policy_decision_refs,
        audit_event_ids: body.audit_event_ids,
        policy: policy_hints(
            body.required_permissions,
            body.entitlement_ready,
            body.package_ready,
            body.metadata,
        ),
    };
    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        command = "skill.curation.run",
        dry_run = command.dry_run,
        stale_after_days = command.stale_after_days,
        narrow_use_threshold = command.narrow_use_threshold,
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "web route forwarding Skill curation run through SDK"
    );
    let result = state
        .skill_client
        .curation_run(command)
        .await
        .map_err(|error| crate::routes::proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(axum::Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

/// Parse a lifecycle action label from the URL path segment.
pub(crate) fn parse_lifecycle_action(
    action: &str,
) -> Result<SkillCurationLifecycleAction, (StatusCode, Json<ErrorResponse>)> {
    match action {
        "pin" => Ok(SkillCurationLifecycleAction::Pin),
        "unpin" => Ok(SkillCurationLifecycleAction::Unpin),
        "archive" => Ok(SkillCurationLifecycleAction::Archive),
        "restore" => Ok(SkillCurationLifecycleAction::Restore),
        "quarantine" => Ok(SkillCurationLifecycleAction::Quarantine),
        "release-quarantine" => Ok(SkillCurationLifecycleAction::ReleaseQuarantine),
        "reject" => Ok(SkillCurationLifecycleAction::Reject),
        _ => Err(err(
            StatusCode::BAD_REQUEST,
            "unsupported skill lifecycle action".into(),
        )),
    }
}

/// Stable string label for lifecycle actions used in trace ids and logs.
pub(crate) fn action_label(action: &SkillCurationLifecycleAction) -> &'static str {
    match action {
        SkillCurationLifecycleAction::Pin => "pin",
        SkillCurationLifecycleAction::Unpin => "unpin",
        SkillCurationLifecycleAction::Archive => "archive",
        SkillCurationLifecycleAction::Restore => "restore",
        SkillCurationLifecycleAction::Quarantine => "quarantine",
        SkillCurationLifecycleAction::ReleaseQuarantine => "release_quarantine",
        SkillCurationLifecycleAction::Supersede => "supersede",
        SkillCurationLifecycleAction::Reject => "reject",
    }
}

/// Assemble provider-neutral policy hints from operator transport fields.
pub(crate) fn policy_hints(
    required_permissions: Vec<String>,
    entitlement_ready: Option<bool>,
    package_ready: Option<bool>,
    metadata: std::collections::BTreeMap<String, String>,
) -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions,
        entitlement_ready,
        package_ready,
        metadata,
    }
}

/// Prefer explicit policy decision refs; fall back to approval refs when empty.
pub(crate) fn refs_or_approval_refs(
    policy_decision_refs: Vec<String>,
    approval_refs: Vec<String>,
) -> Vec<String> {
    if policy_decision_refs.is_empty() {
        approval_refs
    } else {
        policy_decision_refs
    }
}
