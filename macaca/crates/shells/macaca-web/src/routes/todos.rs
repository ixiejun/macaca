//! Todo board and goal listing routes (session-scoped Task Board adapter).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use macaca_proto::{
    ApplicationId, QueryAgentTodosCommand, QueryTaskClaimDiagnosticsCommand, QueryTaskGoalsCommand,
    QueryTaskProgressCommand, TraceContext,
};
use macaca_sdk::ServiceBackedTaskBoardDataSource;

use crate::shell::WebShellFacade;
use crate::state::AppState;

use super::shared::{err, ErrorResponse};

// ---------------------------------------------------------------------------
// Todo Board API
// ---------------------------------------------------------------------------

/// Optional session_id filter for todo/goal queries.
#[derive(Deserialize, Default)]
pub struct SessionQuery {
    #[serde(default)]
    pub(crate) session_id: Option<String>,
}

pub(crate) fn required_session_id<'a>(
    query: &'a SessionQuery,
    route_name: &str,
) -> Result<&'a str, (StatusCode, Json<ErrorResponse>)> {
    let Some(session_id) = query.session_id.as_deref().map(str::trim) else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} requires session_id"),
        ));
    };
    if session_id.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} requires non-empty session_id"),
        ));
    }
    Ok(session_id)
}

/// GET /api/apps/{app_id}/todos — list todos for the current session only.
///
/// The Web UI Task Board is intentionally session-scoped. Requiring `session_id`
/// here prevents accidental application-wide scans when a caller forgets to pass
/// the current chat session id.
pub async fn list_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let session_id = required_session_id(&query, "list_todos")?;
    let shell = WebShellFacade::for_task_board(state.system_facade.service_client());
    let response = shell
        .list_todos_json(app_id, session_id)
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(response))
}

/// GET /api/apps/{app_id}/todos/claim-diagnostics — why workers may not claim `Pending` tasks (session-scoped).
///
/// Requires `session_id`: claim order is evaluated by the Task Service.
pub async fn get_todo_claim_diagnostics(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let sid = required_session_id(&query, "claim diagnostics")?;
    let client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let diag = client
        .query_claim_diagnostics(QueryTaskClaimDiagnosticsCommand::new(
            app_id,
            sid,
            Some(TraceContext::new("web-task-claim-diagnostics")),
        ))
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(
        serde_json::to_value(&diag).unwrap_or(serde_json::json!({})),
    ))
}

/// GET /api/apps/{app_id}/todos/progress — overall progress (optionally filtered by session_id)
pub async fn get_todo_progress(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let p = client
        .query_progress(QueryTaskProgressCommand::new(
            app_id,
            query.session_id,
            Some(TraceContext::new("web-task-progress")),
        ))
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(serde_json::json!({
        "total": p.total, "pending": p.pending, "assigned": p.assigned,
        "in_progress": p.in_progress, "pending_review": p.pending_review,
        "needs_optimization": p.needs_optimization, "completed": p.completed,
        "blocked": p.blocked, "failed": p.failed, "cancelled": p.cancelled,
        "all_done": p.all_done,
    })))
}

/// GET /api/apps/{app_id}/todos/{agent_name} — list agent's board
pub async fn list_agent_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, agent_name)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let board = client
        .query_agent_todos(QueryAgentTodosCommand::new(
            app_id,
            None,
            agent_name,
            Some(TraceContext::new("web-task-agent-board")),
        ))
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(
        serde_json::json!({ "agent": board.agent, "todos": board.todos, "count": board.count }),
    ))
}

/// GET /api/apps/{app_id}/goals — list goals (optionally filtered by session_id)
pub async fn list_goals(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let goals = client
        .query_goals(QueryTaskGoalsCommand::new(
            app_id,
            query.session_id,
            Some(TraceContext::new("web-task-goals")),
        ))
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(
        serde_json::json!({ "goals": goals.goals, "count": goals.count }),
    ))
}
