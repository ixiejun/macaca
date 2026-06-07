//! Skill evolution proposal promote / reject handlers.
//!
//! Both routes map operator transport fields into SDK draft commands and return
//! structured results with trace ids for audit correlation.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::TraceContext;
use macaca_sdk::skill::{SkillEvolutionPromoteDraftCommand, SkillEvolutionRejectDraftCommand};

use crate::routes::{proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{application_skill_scope, parse_application_id, policy_hints};
use crate::skill_operations_routes::types::SkillOperatorCommandRequest;
use crate::state::AppState;

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
