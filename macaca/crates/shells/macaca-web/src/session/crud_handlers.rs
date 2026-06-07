//! HTTP handlers for session read/delete and in-memory cache hydration.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

use super::persistence::{APP_SESSIONS_PREFIX, SESSION_PREFIX};
use super::turn_model::build_turns_from_messages;
use super::types::{SessionDetail, SessionMessage, SessionResponse, StoredSession};

pub(crate) async fn get_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Json<SessionResponse> {
    let sessions = state.sessions.conversations.read().await;
    let messages = sessions
        .get(&app_id)
        .map(|hist| {
            hist.iter()
                .filter_map(|msg| {
                    // Only include user and assistant messages (not system/tool)
                    match msg.role {
                        macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => {
                            Some(SessionMessage {
                                role: format!("{:?}", msg.role).to_lowercase(),
                                content: msg.content.clone(),
                            })
                        }
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Json(SessionResponse { app_id, messages })
}
pub(crate) async fn get_session_by_id(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<SessionDetail>, (StatusCode, Json<ErrorResponse>)> {
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state
        .persist
        .session_store
        .get(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data).map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse session: {e}"),
        )
    })?;

    // Also populate in-memory cache so subsequent messages continue the conversation
    {
        let mut sessions = state.sessions.conversations.write().await;
        sessions.insert(session_id.clone(), stored.messages.clone());
    }

    let messages = stored
        .messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => {
                Some(SessionMessage {
                    role: format!("{:?}", msg.role).to_lowercase(),
                    content: msg.content.clone(),
                })
            }
            _ => None,
        })
        .collect();

    let turns = if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    };

    // EventLog metadata for frontend migration.
    let events_count = state.persist.event_log.count(&session_id).await;
    let events_url = format!("/api/sessions/{}/events", session_id);

    Ok(Json(SessionDetail {
        session_id: stored.meta.session_id,
        app_id: stored.meta.app_id,
        created_at: stored.meta.created_at,
        updated_at: stored.meta.updated_at,
        messages,
        model: turns
            .iter()
            .rev()
            .find_map(|turn| turn.meta.as_ref().and_then(|meta| meta.model.clone())),
        turns,
        status: stored.meta.status,
        events_url,
        events_count,
    }))
}

// ---------------------------------------------------------------------------
// DELETE /api/sessions/:session_id — Delete a session
// ---------------------------------------------------------------------------

pub(crate) async fn delete_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the session to find the app_id
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state
        .persist
        .session_store
        .get(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data).map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse session: {e}"),
        )
    })?;

    // Delete the session data
    state
        .persist
        .session_store
        .delete(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete session: {e}"),
            )
        })?;

    // Delete the app session index entry
    let app_index_key = format!(
        "{}{}/{}",
        APP_SESSIONS_PREFIX, stored.meta.app_id, session_id
    );
    state
        .persist
        .session_store
        .delete(&app_index_key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete session index: {e}"),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}
