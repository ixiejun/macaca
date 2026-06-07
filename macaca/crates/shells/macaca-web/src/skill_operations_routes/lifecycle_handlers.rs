//! Lifecycle mutation handler — pin, archive, quarantine, reject, etc.
//!
//! Maps URL action labels to [`SkillCurationLifecycleAction`] via the adapter
//! and forwards a single SDK command per request.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::TraceContext;
use macaca_sdk::skill::SkillCurationLifecycleCommand;

use crate::routes::{proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{
    action_label, application_skill_scope, parse_application_id, parse_lifecycle_action,
    policy_hints,
};
use crate::skill_operations_routes::types::SkillOperatorCommandRequest;
use crate::state::AppState;

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
