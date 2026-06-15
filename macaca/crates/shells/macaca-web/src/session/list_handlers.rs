//! HTTP handlers for listing persistent sessions (global and per-application).

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

use super::persistence::APP_SESSIONS_PREFIX;
use super::persistence::SESSION_PREFIX;
use super::types::{SessionListItem, SessionListQuery, StoredSession};

fn paged_sessions(
    mut sessions: Vec<SessionListItem>,
    default_limit: Option<usize>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Vec<SessionListItem> {
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let offset = offset.unwrap_or(0);
    let limit = limit.or(default_limit).map(|value| value.clamp(1, 100));
    let iter = sessions.into_iter().skip(offset);
    match limit {
        Some(limit) => iter.take(limit).collect(),
        None => iter.collect(),
    }
}

pub(crate) async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    let keys = state
        .persist
        .session_store
        .list_keys(SESSION_PREFIX)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list sessions: {e}"),
            )
        })?;

    let mut sessions = Vec::new();
    for key in keys {
        if let Some(data) = state.persist.session_store.get(&key).await.map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })? {
            if let Ok(stored) = serde_json::from_slice::<StoredSession>(&data) {
                sessions.push(SessionListItem {
                    session_id: stored.meta.session_id,
                    app_id: stored.meta.app_id,
                    created_at: stored.meta.created_at,
                    updated_at: stored.meta.updated_at,
                    message_count: stored.meta.message_count,
                    title: stored.meta.title,
                    status: stored.meta.status,
                });
            }
        }
    }

    Ok(Json(paged_sessions(
        sessions,
        None,
        query.limit,
        query.offset,
    )))
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/sessions — List sessions for a specific app
// ---------------------------------------------------------------------------

pub(crate) async fn list_app_sessions(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    // Collect session IDs from per-session index keys
    // Format: app_sessions/{app_id}/{session_id}
    let prefix = format!("{}{}/", APP_SESSIONS_PREFIX, app_id);
    let keys = state
        .persist
        .session_store
        .list_keys(&prefix)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list app sessions: {e}"),
            )
        })?;

    let session_ids: Vec<String> = keys
        .iter()
        .filter_map(|key| key.strip_prefix(&prefix).map(|s| s.to_string()))
        .collect();

    let mut sessions = Vec::new();
    for session_id in session_ids {
        let session_key = format!("{}{}", SESSION_PREFIX, session_id);
        if let Some(data) = state
            .persist
            .session_store
            .get(&session_key)
            .await
            .map_err(|e| {
                err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get session: {e}"),
                )
            })?
        {
            if let Ok(stored) = serde_json::from_slice::<StoredSession>(&data) {
                sessions.push(SessionListItem {
                    session_id: stored.meta.session_id,
                    app_id: stored.meta.app_id,
                    created_at: stored.meta.created_at,
                    updated_at: stored.meta.updated_at,
                    message_count: stored.meta.message_count,
                    title: stored.meta.title,
                    status: stored.meta.status,
                });
            }
        }
    }

    Ok(Json(paged_sessions(
        sessions,
        Some(20),
        query.limit,
        query.offset,
    )))
}
