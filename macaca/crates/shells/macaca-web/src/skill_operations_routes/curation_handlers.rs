//! Curation run / apply / rollback handlers.
//!
//! Dry-run analysis and approved apply share [`adapter::run_curation`]; rollback
//! is a separate SDK command with explicit memento reference validation.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_host_composition::runtime_host::SkillCurationRollbackCommand;
use macaca_proto::TraceContext;

use crate::routes::{err, proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{
    application_skill_scope, parse_application_id, policy_hints, run_curation,
};
use crate::skill_operations_routes::types::SkillOperatorCommandRequest;
use crate::state::AppState;

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
