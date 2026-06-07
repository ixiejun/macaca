//! Self-evolution evaluation report handler.
//!
//! The route is intentionally a thin Adapter: it does not inspect checkpoint
//! semantics, evaluate metric thresholds, or branch on application-specific
//! benchmark logic. Scoring (when omitted) and report construction are service-owned.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::TraceContext;
use macaca_sdk::skill::{SkillEvaluationReportCommand, SkillEvaluationScoreCommand};

use crate::routes::{proto_err, ErrorResponse};
use crate::skill_operations_routes::adapter::{application_skill_scope, parse_application_id};
use crate::skill_operations_routes::types::SkillEvaluationReportRequest;
use crate::state::AppState;

/// POST /api/apps/{app_id}/skills/operations/evaluation/report.
///
/// Builds a sanitized self-evolution report through the Skill service facade.
pub async fn post_skill_evaluation_report(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<SkillEvaluationReportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-skill-evaluation-report-{}", app_id.0));
    let scope = application_skill_scope(app_id);
    let include_markdown = body.include_markdown;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        command = "skill.evaluation.report",
        include_markdown,
        "web route building Skill self-evolution evaluation report through SDK"
    );

    let score = match body.score {
        Some(score) => score,
        None => {
            state
                .skill_client
                .evaluate_self_evolution(SkillEvaluationScoreCommand {
                    trace: trace.clone(),
                    scope: scope.clone(),
                    record: body.record.clone(),
                })
                .await
                .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?
                .score
        }
    };
    let result = state
        .skill_client
        .self_evolution_evaluation_report(SkillEvaluationReportCommand {
            trace: trace.clone(),
            scope,
            record: body.record,
            score,
            include_markdown,
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        command = "skill.evaluation.report",
        passed = result.score.passed,
        reason_count = result.score.reason_codes.len(),
        include_markdown,
        "web route emitted Skill self-evolution evaluation report through SDK"
    );
    Ok(Json(
        serde_json::json!({ "result": result, "trace_id": trace.trace_id }),
    ))
}
