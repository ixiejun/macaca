//! Session event log, lineage, compaction, and source-artifact routes.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use macaca_proto::{EventLogQuery, SessionLineage};

use crate::source_artifact::{
    ContextSourceArtifactRepository, SourceArtifactQuery, SourceArtifactResponse,
};
use crate::state::AppState;

use super::shared::{err, ErrorResponse};

// ---------------------------------------------------------------------------
// Event Log API
// ---------------------------------------------------------------------------

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub since: Option<u64>,
    pub limit: Option<usize>,
    pub source: Option<String>,
    pub agent: Option<String>,
    pub event_type: Option<String>,
}

/// Response payload for the events endpoint.
#[derive(Serialize)]
pub struct EventsResponse {
    pub events: Vec<macaca_proto::EventEntry>,
    pub latest_seq: u64,
}

/// GET /api/sessions/:id/events?since={seq}&limit={n}
///
/// Returns persisted events for a session. Clients can poll by passing the
/// last received `latest_seq` as `since` to get only new events.
pub async fn get_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit = params.limit.unwrap_or(500).clamp(1, 2000);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit)
        .source(params.source)
        .agent(params.agent)
        .event_type(params.event_type);
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/run-trace?since={seq}&limit={n}
///
/// Returns only `event_type == "run_trace"` rows (pipeline checkpoints). Use this
/// to see where execution is without loading full SSE-style event streams.
pub async fn get_session_run_trace(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit_out = params.limit.unwrap_or(500).clamp(1, 2000);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit_out)
        .event_type(Some("run_trace".to_string()));
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/context-reports?since={seq}&limit={n}&agent={agent}
///
/// Returns request-scoped context report summaries. Full prompt content is not
/// persisted by default; payloads contain source counts, token estimates, and hashes.
pub async fn get_session_context_reports(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit_out = params.limit.unwrap_or(100).clamp(1, 500);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit_out)
        .agent(params.agent)
        .event_type(Some("context_report".to_string()));
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/source-artifact?ref={source_ref}
///
/// Follows a context-report `source_ref` back to its canonical payload when the
/// reference is EventLog-backed. Unsupported refs return an explicit reason so
/// diagnostics UI can explain why retrieval is unavailable.
pub async fn get_session_source_artifact(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<SourceArtifactQuery>,
) -> Result<Json<SourceArtifactResponse>, (StatusCode, Json<ErrorResponse>)> {
    if params.source_ref.trim().is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "source ref query parameter is required".to_string(),
        ));
    }

    let repository = ContextSourceArtifactRepository::new(Arc::clone(&state.persist.event_log));
    Ok(Json(
        repository.resolve(&session_id, &params.source_ref).await,
    ))
}

#[derive(Debug, Deserialize)]
pub struct ManualCompactRequest {
    #[serde(default)]
    pub focus_topic: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ManualCompactResponse {
    pub root_session_id: String,
    pub source_session_id: String,
    pub successor_session_id: String,
    pub source_segment_id: String,
    pub successor_segment_id: String,
    pub summary: String,
}

pub async fn manual_compact_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<ManualCompactRequest>,
) -> Result<Json<ManualCompactResponse>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = state
        .manual_session_compaction_snapshot(session_id, request.focus_topic)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(ManualCompactResponse {
        root_session_id: snapshot.root_session_id,
        source_session_id: snapshot.source_session_id,
        successor_session_id: snapshot.successor_session_id,
        source_segment_id: snapshot.source_segment_id,
        successor_segment_id: snapshot.successor_segment_id,
        summary: snapshot.summary,
    }))
}

#[derive(Debug, Serialize)]
pub struct SessionLineageResponse {
    pub requested_session_id: String,
    pub lineage_tip_session_id: String,
    pub lineage: Vec<SessionLineage>,
}

pub async fn get_session_lineage(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionLineageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = state
        .session_lineage_snapshot(session_id)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(SessionLineageResponse {
        requested_session_id: snapshot.requested_session_id,
        lineage_tip_session_id: snapshot.lineage_tip_session_id,
        lineage: snapshot.lineage,
    }))
}
