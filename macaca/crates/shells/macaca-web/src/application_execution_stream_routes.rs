//! Realtime Web Shell stream for persisted application execution events.
//!
//! This module is intentionally a thin Observer/Adapter around the shared
//! append-only EventLog.  The Web shell owns HTTP/SSE framing only: it validates
//! the route scope, replays already persisted rows, subscribes to durable append
//! notifications, and emits EventLog references back to the browser.  It never
//! constructs providers, starts execution loops, sends control commands, or
//! treats browser connection lifetime as execution authority.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Response;
use axum::Json;
use futures::Stream;
use macaca_persist::EventLogQuery;
use macaca_proto::{ApplicationExecutionEventType, ApplicationId, EventEntry, TraceContext};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, info, warn};

use crate::application_execution_routes::{parse_application_id, required_text, required_trace};
use crate::routes::ErrorResponse;
use crate::state::AppState;

/// Source value written by `service.application_execution` when it appends
/// protocol events to the shared EventLog.  Keeping the string in this adapter
/// lets the Web layer query a service-owned index without knowing provider or
/// application-specific semantics.
pub(crate) const APPLICATION_EXECUTION_EVENT_SOURCE: &str = "service.application_execution";

const DEFAULT_EVENT_STREAM_LIMIT: usize = 100;
const MAX_EVENT_STREAM_LIMIT: usize = 500;
const SSE_EVENT_NAME: &str = "application_execution_event";

/// Query accepted by the persisted EventLog SSE route.
///
/// `trace_id` is required even though this route is read-only.  It gives every
/// subscriber an explicit audit handle in the Web logs without letting the
/// browser become the owner of execution state.
#[derive(Debug, Deserialize)]
pub struct ExecutionEventStreamQuery {
    pub session_id: String,
    pub trace_id: String,
    #[serde(default)]
    pub since: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub event_type: Option<ApplicationExecutionEventType>,
}

/// GET /api/apps/{app_id}/execution/events
pub async fn stream_execution_events(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<ExecutionEventStreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    let (application_id, trace, event_query) = build_stream_query(&app_id, query)?;
    let event_log = state.persist.event_log.clone();
    let session_id = event_query.session_id.clone();
    let limit = event_query.limit;

    info!(
        app_id = %application_id,
        session_id = %session_id,
        trace_id = %trace.trace_id,
        since_seq = event_query.since_seq,
        limit,
        "web route opened application execution EventLog stream"
    );

    let stream = async_stream::stream! {
        let mut cursor = event_query.since_seq;
        let mut receiver = event_log.subscribe();

        let initial_batch = query_application_events(&event_log, application_id, event_query.clone()).await;
        cursor = cursor.max(initial_batch.scanned_cursor);
        for entry in initial_batch.entries {
            cursor = cursor.max(entry.seq);
            yield Ok(event_entry_to_sse(&entry));
        }

        loop {
            match receiver.recv().await {
                Ok((changed_session_id, latest_seq)) => {
                    if changed_session_id != session_id {
                        continue;
                    }

                    let follow_up_query = event_query.clone().since(cursor).limit(limit);
                    let batch = query_application_events(
                        &event_log,
                        application_id,
                        follow_up_query,
                    )
                    .await;
                    cursor = cursor.max(batch.scanned_cursor);

                    if batch.entries.is_empty() {
                        cursor = cursor.max(latest_seq);
                        debug!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            latest_seq,
                            "application execution EventLog notification had no scoped rows"
                        );
                        continue;
                    }

                    for entry in batch.entries {
                        cursor = cursor.max(entry.seq);
                        yield Ok(event_entry_to_sse(&entry));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        app_id = %application_id,
                        session_id = %session_id,
                        trace_id = %trace.trace_id,
                        skipped,
                        "application execution EventLog stream lagged and will resume from cursor"
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!(
                        app_id = %application_id,
                        session_id = %session_id,
                        trace_id = %trace.trace_id,
                        "application execution EventLog stream notification channel closed"
                    );
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// GET /api/apps/{app_id}/execution/events/ws
pub async fn websocket_execution_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<ExecutionEventStreamQuery>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let (application_id, trace, event_query) = build_stream_query(&app_id, query)?;
    let event_log = state.persist.event_log.clone();
    let session_id = event_query.session_id.clone();
    let limit = event_query.limit;

    info!(
        app_id = %application_id,
        session_id = %session_id,
        trace_id = %trace.trace_id,
        since_seq = event_query.since_seq,
        limit,
        "web route opened application execution EventLog WebSocket"
    );

    Ok(ws.on_upgrade(move |socket| async move {
        run_websocket_event_observer(
            socket,
            event_log,
            application_id,
            trace,
            event_query,
            session_id,
            limit,
        )
        .await;
    }))
}

/// Build the durable EventLog query used by the SSE adapter.
///
/// This helper is deliberately pure and testable.  It applies the route's
/// application UUID validation, trace requirement, bounded page size, and
/// service-owned source filter before any async stream is created.
pub(crate) fn build_stream_query(
    app_id: &str,
    query: ExecutionEventStreamQuery,
) -> Result<(ApplicationId, TraceContext, EventLogQuery), (StatusCode, Json<ErrorResponse>)> {
    let application_id = parse_application_id(app_id)?;
    let trace = required_trace(&query.trace_id, "events")?;
    let session_id = required_text("session_id", &query.session_id)?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_EVENT_STREAM_LIMIT)
        .clamp(1, MAX_EVENT_STREAM_LIMIT);
    let event_type = query.event_type.map(|event_type| format!("{event_type:?}"));
    let event_query = EventLogQuery::new(session_id)
        .since(query.since.unwrap_or(0))
        .limit(limit)
        .source(Some(APPLICATION_EXECUTION_EVENT_SOURCE.to_string()))
        .event_type(event_type);
    Ok((application_id, trace, event_query))
}

/// Drive a WebSocket observer from durable EventLog rows.
///
/// The WebSocket never receives provider callbacks directly.  It first sends
/// already persisted rows after the requested cursor, then waits for EventLog
/// append notifications and queries the same persisted store again.  This
/// preserves the invariant that browser connectivity can affect rendering but
/// cannot affect backend execution, EventLog persistence, or current-state
/// projection.
async fn run_websocket_event_observer(
    mut socket: WebSocket,
    event_log: Arc<macaca_persist::EventLog>,
    application_id: ApplicationId,
    trace: TraceContext,
    event_query: EventLogQuery,
    session_id: String,
    limit: usize,
) {
    let mut cursor = event_query.since_seq;
    let mut receiver = event_log.subscribe();

    let initial_batch =
        query_application_events(&event_log, application_id, event_query.clone()).await;
    cursor = cursor.max(initial_batch.scanned_cursor);
    if send_websocket_batch(&mut socket, &initial_batch.entries)
        .await
        .is_err()
    {
        info!(
            app_id = %application_id,
            session_id = %session_id,
            trace_id = %trace.trace_id,
            "application execution WebSocket closed during initial replay"
        );
        return;
    }
    if let Some(last) = initial_batch.entries.last() {
        cursor = cursor.max(last.seq);
    }

    loop {
        tokio::select! {
            client_message = socket.recv() => {
                match client_message {
                    Some(Ok(Message::Close(_))) | None => {
                        info!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            cursor,
                            "application execution WebSocket subscriber disconnected"
                        );
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        debug!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            "application execution WebSocket ignored client data frame"
                        );
                    }
                    Some(Err(error)) => {
                        warn!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            error = %error,
                            "application execution WebSocket receive failed"
                        );
                        break;
                    }
                }
            }
            notification = receiver.recv() => {
                match notification {
                    Ok((changed_session_id, latest_seq)) => {
                        if changed_session_id != session_id {
                            continue;
                        }
                        let follow_up_query = event_query.clone().since(cursor).limit(limit);
                        let batch = query_application_events(
                            &event_log,
                            application_id,
                            follow_up_query,
                        )
                        .await;
                        cursor = cursor.max(batch.scanned_cursor);
                        if batch.entries.is_empty() {
                            cursor = cursor.max(latest_seq);
                            debug!(
                                app_id = %application_id,
                                session_id = %session_id,
                                trace_id = %trace.trace_id,
                                latest_seq,
                                "application execution WebSocket notification had no scoped rows"
                            );
                            continue;
                        }
                        if send_websocket_batch(&mut socket, &batch.entries).await.is_err() {
                            break;
                        }
                        if let Some(last) = batch.entries.last() {
                            cursor = cursor.max(last.seq);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            skipped,
                            "application execution WebSocket lagged and will resume from cursor"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        warn!(
                            app_id = %application_id,
                            session_id = %session_id,
                            trace_id = %trace.trace_id,
                            "application execution WebSocket notification channel closed"
                        );
                        break;
                    }
                }
            }
        }
    }
}

/// Send one persisted EventLog batch to the subscriber.
///
/// Each message is an independent durable row payload.  The client can render
/// it immediately, but replay remains the recovery mechanism because every
/// message carries the same EventLog coordinate returned by HTTP replay.
async fn send_websocket_batch(
    socket: &mut WebSocket,
    entries: &[EventEntry],
) -> Result<(), axum::Error> {
    for entry in entries {
        socket
            .send(Message::Text(
                event_entry_to_stream_payload(entry).to_string().into(),
            ))
            .await?;
    }
    Ok(())
}

struct ApplicationEventBatch {
    entries: Vec<EventEntry>,
    scanned_cursor: u64,
}

async fn query_application_events(
    event_log: &macaca_persist::EventLog,
    application_id: ApplicationId,
    query: EventLogQuery,
) -> ApplicationEventBatch {
    let requested_limit = query.limit;
    let mut entries = Vec::with_capacity(requested_limit.min(MAX_EVENT_STREAM_LIMIT));
    let mut scanned_cursor = query.since_seq;

    loop {
        let rows = event_log
            .query_indexed(
                query
                    .clone()
                    .since(scanned_cursor)
                    .limit(MAX_EVENT_STREAM_LIMIT),
            )
            .await;
        if rows.is_empty() {
            break;
        }

        let row_count = rows.len();
        for entry in rows {
            scanned_cursor = scanned_cursor.max(entry.seq);
            if event_matches_application(&entry, application_id) {
                entries.push(entry);
                if entries.len() >= requested_limit {
                    return ApplicationEventBatch {
                        entries,
                        scanned_cursor,
                    };
                }
            }
        }

        if row_count < MAX_EVENT_STREAM_LIMIT {
            break;
        }
    }

    ApplicationEventBatch {
        entries,
        scanned_cursor,
    }
}

fn event_matches_application(entry: &EventEntry, application_id: ApplicationId) -> bool {
    entry
        .payload
        .get("application_id")
        .and_then(Value::as_str)
        .is_some_and(|value| value == application_id.to_string())
}

fn event_entry_to_sse(entry: &EventEntry) -> Event {
    Event::default()
        .event(SSE_EVENT_NAME)
        .id(entry.seq.to_string())
        .data(event_entry_to_stream_payload(entry).to_string())
}

/// Convert an EventLog row into the browser-facing SSE payload.
///
/// The payload carries both the sanitized service payload and a durable
/// `event_ref`.  Browsers can render the event immediately, while auditors and
/// replay clients can later dereference the immutable EventLog coordinate
/// without trusting transient connection state.
pub(crate) fn event_entry_to_stream_payload(entry: &EventEntry) -> Value {
    json!({
        "seq": entry.seq,
        "session_id": entry.session_id,
        "event_type": entry.event_type,
        "source": entry.source,
        "event_ref": {
            "session_id": entry.session_id,
            "seq": entry.seq,
        },
        "payload": entry.payload,
    })
}
