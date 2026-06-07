//! SSE handler for live session event streaming (Adapter over executor + EventLog).
//!
//! Hot-swaps the coordinator SSE channel on browser reconnect while avoiding duplicate
//! executor subscriptions when an active chat session already forwards delegated events.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;

use macaca_proto::ApplicationId;

use crate::routes::{err, ErrorResponse};
use crate::sse::convert_executor_event_to_sse;
use crate::state::AppState;

use super::persistence::SESSION_PREFIX;
use super::types::StoredSession;

pub(crate) async fn stream_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
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

    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&stored.meta.app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let maybe_executor = state.executor_registry.get(&app_id).await;

    // Check if coordinator is actively running (has an active_session entry)
    let active = state
        .sessions
        .active_sessions
        .read()
        .await
        .contains_key(&session_id);

    // If coordinator is active, hot-swap its sse_tx so events flow to this new connection
    let mut coordinator_rx = if active {
        let (new_tx, new_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
        // Replace sse_tx → bridge now forwards coordinator events to new_rx
        let sessions = state.sessions.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            *session.sse_tx.write().await = new_tx;
            tracing::info!(session_id = %session_id, "SSE tx hot-swapped for browser reconnect");
        }
        Some(new_rx)
    } else {
        None
    };

    let stream = async_stream::stream! {
        use tokio::sync::broadcast;

        // NOTE: Do NOT check stored.meta.status here.
        // Coordinator may have completed (status='completed') while executor
        // workers are still running delegated tasks. Always subscribe to
        // executor events and let the broadcast channel close naturally.

        // Subscribe to EventLog broadcast so we can notify the frontend
        // when new events are appended (e.g. driver_trace from delegated agents).
        let mut event_log_rx = state.persist.event_log.subscribe();
        let stream_session_id = session_id.clone();

        let Some(executor) = maybe_executor else {
            yield Ok(Event::default().event("session_end").data("{}"));
            return;
        };

        if let Some(ref mut coord_rx) = coordinator_rx {
            // Active session mode:
            // The chat_v2 forwarders already send both coordinator and delegated
            // executor events into this hot-swapped coordinator channel.
            // Reading executor_rx here would duplicate every delegated event.
            loop {
                tokio::select! {
                    msg = coord_rx.recv() => {
                        match msg {
                            Some(event) => yield event,
                            None => break,
                        }
                    }
                    result = event_log_rx.recv() => {
                        match result {
                            Ok((notified_sid, latest_seq)) => {
                                if notified_sid == stream_session_id {
                                    yield Ok(Event::default()
                                        .event("update")
                                        .data(serde_json::json!({
                                            "seq": latest_seq
                                        }).to_string()));
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
        } else {
            // No active coordinator — just stream executor events
            let mut executor_rx = executor.subscribe_to_events();
            loop {
                tokio::select! {
                    result = executor_rx.recv() => {
                        match result {
                            Ok(event) => yield convert_executor_event_to_sse(event),
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                    result = event_log_rx.recv() => {
                        match result {
                            Ok((notified_sid, latest_seq)) => {
                                if notified_sid == stream_session_id {
                                    yield Ok(Event::default()
                                        .event("update")
                                        .data(serde_json::json!({
                                            "seq": latest_seq
                                        }).to_string()));
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
