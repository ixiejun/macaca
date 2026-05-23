//! Skill governance and curation operations route adapters.
//!
//! The Web shell exposes a bounded operator snapshot, but all governance,
//! curation, alias, and proposal semantics remain owned by `service.skill`.
//! This module is therefore an Adapter: it builds typed SDK commands, attaches
//! trace/scope metadata, logs sanitized counts, and serializes the service
//! DTOs without reading skill files or classifying lifecycle state locally.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_skill::{
    SkillAliasSnapshotCommand, SkillAuthorKind, SkillCurationDryRunCommand,
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillCurationRollbackCommand,
    SkillCurationRunCommand, SkillEvolutionPromoteDraftCommand, SkillEvolutionRejectDraftCommand,
    SkillExperienceProposalSnapshotCommand, SkillGovernanceSnapshotCommand,
    SkillServicePolicyHints, SkillServiceScope,
};
use serde::Deserialize;

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

/// GET /api/apps/{app_id}/skills/operations
/// returns a sanitized Skill operations snapshot for one application.
pub async fn get_skill_operations(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-operations-{}", app_id.0));
    let scope = application_skill_scope(app_id);

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route aggregating skill governance operations through Skill service"
    );

    let governance = state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            include_archived: true,
            lifecycle_filters: Vec::new(),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let curation = state
        .skill_client
        .curation_dry_run(SkillCurationDryRunCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            stale_after_days: 30,
            narrow_use_threshold: 1,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let aliases = state
        .skill_client
        .alias_snapshot(SkillAliasSnapshotCommand {
            trace: trace.clone(),
            scope: scope.clone(),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let proposals = state
        .skill_client
        .skill_experience_snapshot(SkillExperienceProposalSnapshotCommand {
            trace: trace.clone(),
            scope,
            include_discarded: false,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        governance_records = governance.records.len(),
        recommendations = curation.recommendations.len(),
        aliases = aliases.aliases.len(),
        proposals = proposals.proposals.len(),
        "web route emitted sanitized skill operations snapshot"
    );

    Ok(Json(serde_json::json!({
        "governance": governance,
        "curation": curation,
        "aliases": aliases,
        "proposals": proposals,
        "count": {
            "governance_records": governance.records.len(),
            "curation_recommendations": curation.recommendations.len(),
            "aliases": aliases.aliases.len(),
            "proposals": proposals.proposals.len(),
        },
        "trace_id": trace.trace_id,
    })))
}

/// Request body shared by approval-gated Skill operation routes.
///
/// The Web shell only normalizes operator transport fields into service DTOs.
/// Lifecycle validation, policy admission, rollback ownership, and proposal
/// semantics remain inside the SDK-backed Skill service provider.
#[derive(Debug, Default, Deserialize)]
pub struct SkillOperatorCommandRequest {
    pub reason: Option<String>,
    pub rationale: Option<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    #[serde(default)]
    pub approval_refs: Vec<String>,
    #[serde(default)]
    pub audit_event_ids: Vec<String>,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    pub entitlement_ready: Option<bool>,
    pub package_ready: Option<bool>,
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, String>,
    pub name: Option<String>,
    pub source: Option<String>,
    pub source_scope: Option<String>,
    pub author_kind: Option<SkillAuthorKind>,
    pub stale_after_days: Option<i64>,
    pub narrow_use_threshold: Option<u64>,
    pub rollback_ref: Option<String>,
}

/// POST /api/apps/{app_id}/skills/operations/lifecycle/{action}/{skill_id}
/// forwards pin, unpin, archive, restore, quarantine, and reject commands.
pub async fn post_skill_lifecycle_operation(
    State(state): State<Arc<AppState>>,
    Path((app_id, action, skill_id)): Path<(String, String, String)>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let action = parse_lifecycle_action(&action)?;
    let trace = TraceContext::new(format!(
        "web-skill-lifecycle-{}-{}",
        action_label(&action),
        app_id.0
    ));
    let command = SkillCurationLifecycleCommand {
        trace: trace.clone(),
        scope: application_skill_scope(app_id),
        skill_id: skill_id.clone(),
        name: body.name.unwrap_or_else(|| skill_id.clone()),
        source: body.source.unwrap_or_else(|| "web-skill-operations".into()),
        source_scope: body.source_scope.unwrap_or_else(|| "application".into()),
        author_kind: body.author_kind.unwrap_or_default(),
        reason: body
            .reason
            .unwrap_or_else(|| "operator_skill_lifecycle_request".into()),
        evidence_ids: body.evidence_ids,
        task_id: None,
        policy_decision_refs: body.policy_decision_refs,
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
        command = "skill.curation.lifecycle",
        action = %action_label(&action),
        skill_id = %command.skill_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "web route forwarding Skill lifecycle command through SDK"
    );
    let result = state
        .skill_client
        .curation_lifecycle(action, command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

/// POST /api/apps/{app_id}/skills/operations/curation/run starts dry-run analysis.
pub async fn post_skill_curation_run(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    run_curation(state, app_id, body, true).await
}

/// POST /api/apps/{app_id}/skills/operations/curation/apply starts approved apply.
pub async fn post_skill_curation_apply(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    run_curation(state, app_id, body, false).await
}

/// POST /api/apps/{app_id}/skills/operations/curation/rollback restores a memento ref.
pub async fn post_skill_curation_rollback(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-curation-rollback-{}", app_id.0));
    let command = SkillCurationRollbackCommand {
        trace: trace.clone(),
        scope: application_skill_scope(app_id),
        rollback_ref: body
            .rollback_ref
            .ok_or_else(|| err(StatusCode::BAD_REQUEST, "rollback_ref is required".into()))?,
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
        command = "skill.curation.rollback",
        rollback_ref = %command.rollback_ref,
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "web route forwarding Skill rollback command through SDK"
    );
    let result = state
        .skill_client
        .curation_rollback(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

/// POST /api/apps/{app_id}/skills/operations/proposals/{proposal_id}/promote.
pub async fn post_skill_proposal_promote(
    State(state): State<Arc<AppState>>,
    Path((app_id, proposal_id)): Path<(String, String)>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-proposal-promote-{}", app_id.0));
    let command = SkillEvolutionPromoteDraftCommand {
        trace: trace.clone(),
        scope: application_skill_scope(app_id),
        proposal_id,
        reason: body
            .reason
            .unwrap_or_else(|| "operator_skill_proposal_promote".into()),
        evidence_ids: body.evidence_ids,
        policy_decision_refs: body.policy_decision_refs,
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
        command = "skill.evolution.promote_draft",
        proposal_id = %command.proposal_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "web route forwarding Skill proposal promotion through SDK"
    );
    let result = state
        .skill_client
        .promote_skill_draft(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

/// POST /api/apps/{app_id}/skills/operations/proposals/{proposal_id}/reject.
pub async fn post_skill_proposal_reject(
    State(state): State<Arc<AppState>>,
    Path((app_id, proposal_id)): Path<(String, String)>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-proposal-reject-{}", app_id.0));
    let command = SkillEvolutionRejectDraftCommand {
        trace: trace.clone(),
        scope: application_skill_scope(app_id),
        proposal_id,
        rationale: body
            .rationale
            .or(body.reason)
            .unwrap_or_else(|| "operator_skill_proposal_reject".into()),
        evidence_ids: body.evidence_ids,
        policy_decision_refs: body.policy_decision_refs,
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
        command = "skill.evolution.reject_draft",
        proposal_id = %command.proposal_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "web route forwarding Skill proposal rejection through SDK"
    );
    let result = state
        .skill_client
        .reject_skill_draft(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

fn application_skill_scope(app_id: ApplicationId) -> SkillServiceScope {
    SkillServiceScope {
        application_id: Some(app_id),
        session_id: None,
        tenant_id: None,
        agent_name: None,
    }
}

async fn run_curation(
    state: Arc<AppState>,
    app_id: String,
    body: SkillOperatorCommandRequest,
    dry_run: bool,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
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
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}

fn parse_lifecycle_action(
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

fn action_label(action: &SkillCurationLifecycleAction) -> &'static str {
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

fn policy_hints(
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

#[cfg(test)]
mod tests {
    #[test]
    fn skill_operations_routes_remain_thin_sdk_adapters() {
        let source = include_str!("skill_operations_routes.rs");
        assert!(source.contains(".skill_client"));
        assert!(source.contains("curation_lifecycle"));
        assert!(source.contains("promote_skill_draft"));
        assert!(source.contains("reject_skill_draft"));
        assert!(!source.contains(&format!("{}{}", "macaca_runtime", "_host::")));
        assert!(!source.contains(&format!("{}{}", "SkillGovernance", "StoreStrategy")));
        assert!(!source.contains(&format!("{}{}", "SkillLifecycleState::", "Archived")));
    }
}
