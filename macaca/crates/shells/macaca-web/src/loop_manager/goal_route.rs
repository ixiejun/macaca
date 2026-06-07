//! HTTP route adapter for goal creation (`POST /api/apps/{app_id}/goals`).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::ApplicationId;

use super::loop_orchestrator::ensure_plan_and_worker_loops;
use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

pub(crate) async fn create_goal(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let description = body["description"].as_str().ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "Missing 'description' field".into(),
        )
    })?;
    let store = Arc::clone(&state.persist.todo_store);
    let session_id = body["session_id"].as_str().map(|s| s.to_string());
    let space =
        macaca_task::TaskSpace::for_session(app_id.clone(), session_id.clone(), Arc::clone(&store));
    let goal = space.push_goal(description).await;

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        goal.session_id.as_deref(),
        &app_id,
        crate::run_trace::phase::GOAL_CREATE_HTTP,
        "api.create_goal",
        crate::run_trace::status::OK,
        Some(description.chars().take(160).collect::<String>()),
        None,
        Some(goal.id.to_string()),
        None,
    )
    .await;

    // Start PlanLoop + WorkerLoops if not already running
    ensure_plan_and_worker_loops(&state, &app_id, session_id).await;

    Ok(Json(
        serde_json::json!({ "goal_id": goal.id.to_string(), "status": "pending" }),
    ))
}
