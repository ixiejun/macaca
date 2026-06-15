//! GET snapshot handler — aggregates multiple Skill service read commands.
//!
//! The handler fans out to governance, curation dry-run, alias, proposal, processing,
//! and materialization snapshot commands, then returns a single sanitized JSON view
//! with count metadata for operator dashboards.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_host_composition::runtime_host::{
    SkillAliasSnapshotCommand, SkillAutonomousMaterializationSnapshotCommand,
    SkillCurationDryRunCommand, SkillExperienceProposalSnapshotCommand,
    SkillGovernanceSnapshotCommand, SkillProposalProcessingSnapshotCommand,
};
use macaca_proto::TraceContext;

use crate::routes::{proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{application_skill_scope, parse_application_id};
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
        command = "skill.operations.snapshot",
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
            scope: scope.clone(),
            include_discarded: false,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let processing = state
        .skill_client
        .skill_proposal_processing_snapshot(SkillProposalProcessingSnapshotCommand {
            trace: trace.clone(),
            scope: scope.clone(),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let materialization_operator = state
        .skill_client
        .autonomous_materialization_snapshot(SkillAutonomousMaterializationSnapshotCommand {
            trace: trace.clone(),
            scope,
            limit: 10,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        command = "skill.operations.snapshot",
        governance_records = governance.records.len(),
        recommendations = curation.recommendations.len(),
        aliases = aliases.aliases.len(),
        proposals = proposals.proposals.len(),
        processing_records = processing.records.len(),
        processing_ready = processing.state_counts.ready_for_materialization,
        processing_duplicates = processing.state_counts.suppressed_duplicate,
        processing_rejected = processing.state_counts.rejected,
        materialization_operator_runs = materialization_operator.recent_runs.len(),
        materialization_operator_applied = materialization_operator.status_counts.applied,
        "web route emitted sanitized skill operations snapshot"
    );

    Ok(Json(serde_json::json!({
        "governance": governance,
        "curation": curation,
        "aliases": aliases,
        "proposals": proposals,
        "processing": processing,
        "materialization_operator": materialization_operator,
        "count": {
            "governance_records": governance.records.len(),
            "curation_recommendations": curation.recommendations.len(),
            "aliases": aliases.aliases.len(),
            "proposals": proposals.proposals.len(),
            "processing_records": processing.records.len(),
            "processing_waiting_proposals": processing.waiting_proposal_count,
            "processing_duplicate_groups": processing.duplicate_group_count,
            "processing_ready": processing.state_counts.ready_for_materialization,
            "processing_suppressed_duplicates": processing.state_counts.suppressed_duplicate,
            "processing_rejected": processing.state_counts.rejected,
            "materialization_operator_runs": materialization_operator.recent_runs.len(),
            "materialization_operator_applied": materialization_operator.status_counts.applied,
            "materialization_operator_previewed": materialization_operator.status_counts.previewed,
            "materialization_operator_denied": materialization_operator.status_counts.denied,
        },
        "trace_id": trace.trace_id,
    })))
}
