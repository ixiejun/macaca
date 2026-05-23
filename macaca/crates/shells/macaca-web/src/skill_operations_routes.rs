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
    SkillAliasSnapshotCommand, SkillCurationDryRunCommand, SkillExperienceProposalSnapshotCommand,
    SkillGovernanceSnapshotCommand, SkillServiceScope,
};

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

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

fn application_skill_scope(app_id: ApplicationId) -> SkillServiceScope {
    SkillServiceScope {
        application_id: Some(app_id),
        session_id: None,
        agent_name: None,
    }
}
