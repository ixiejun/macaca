//! Autonomous materialization operator run handler.
//!
//! Web remains a transport Adapter: package target selection, proposal processing,
//! materialization, policy admission, and result mementos all stay behind the
//! SDK-backed Skill service command.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::TraceContext;
use macaca_sdk::skill::{
    SkillAutonomousMaterializationRunCommand, SkillPackageOwnershipClass,
};

use crate::routes::{err, proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{
    application_skill_scope, materialization_collection_root, parse_application_id,
    policy_hints, refs_or_approval_refs,
};
use crate::skill_operations_routes::types::SkillOperatorCommandRequest;
use crate::state::AppState;

/// POST /api/apps/{app_id}/skills/operations/materialization/operator/run
/// runs the service-owned autonomous materialization operator.
pub async fn post_skill_materialization_operator_run(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<SkillOperatorCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-materialization-operator-{}", app_id.0));
    let package_collection_root =
        materialization_collection_root(&state, &app_id, body.package_collection_root.as_ref())
            .await
            .map_err(|message| err(StatusCode::BAD_REQUEST, message))?;
    let policy = policy_hints(
        body.required_permissions,
        body.entitlement_ready,
        body.package_ready,
        body.metadata,
    );
    let policy_decision_refs = refs_or_approval_refs(body.policy_decision_refs, body.approval_refs);

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        command = "skill.materialization.operator.run",
        dry_run = body.dry_run.unwrap_or(true),
        batch_limit = body.batch_limit.unwrap_or(5),
        "web route forwarding autonomous materialization operator run through SDK"
    );

    let result = state
        .skill_client
        .run_autonomous_materialization(SkillAutonomousMaterializationRunCommand {
            trace: trace.clone(),
            scope: application_skill_scope(app_id),
            dry_run: body.dry_run.unwrap_or(true),
            batch_limit: body.batch_limit.unwrap_or(5),
            min_ready_score: body.min_ready_score.unwrap_or(40),
            package_collection_root,
            ownership: body
                .ownership
                .unwrap_or(SkillPackageOwnershipClass::AgentPrivate),
            reason: body
                .reason
                .or(body.rationale)
                .unwrap_or_else(|| "operator requested autonomous materialization".into()),
            evidence_ids: body.evidence_ids,
            policy_decision_refs,
            audit_event_ids: body.audit_event_ids,
            policy,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    tracing::info!(
        trace_id = %trace.trace_id,
        command = "skill.materialization.operator.run",
        run_id = %result.run_id,
        status = ?result.status,
        selected = result.selected_proposal_ids.len(),
        mutated = result.mutated,
        "web route forwarded autonomous materialization operator run"
    );

    Ok(Json(serde_json::json!({
        "result": result,
        "trace_id": trace.trace_id,
    })))
}
