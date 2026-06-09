//! Executor event → SSE bridge adapter.
//!
//! Forwards delegated worker executor events to the hot-swappable SSE channel
//! while synchronizing kernel agent activity. Shared by WASM and framework paths.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use macaca_sdk::runtime_host::executor::ExecutorEvent;
use tokio::sync::RwLock;

use axum::response::sse::Event;
use std::convert::Infallible;

use crate::sse::convert_executor_event_to_sse;
use crate::state::AppState;

use super::agent_activity_adapter::sync_delegated_agent_activity_from_executor_event;

/// Spawn a background task that mirrors executor events into the session SSE stream.
pub(crate) fn spawn_executor_event_forwarder(
    state: Arc<AppState>,
    mut exec_rx: tokio::sync::broadcast::Receiver<ExecutorEvent>,
    sse_tx: Arc<RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
    forwarder_stop_flag: Arc<AtomicBool>,
    log_label: &'static str,
) {
    tokio::spawn(async move {
        loop {
            match exec_rx.recv().await {
                Ok(exec_event) => {
                    if forwarder_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        tracing::debug!(label = log_label, "Executor event forwarder stopped (superseded)");
                        break;
                    }
                    sync_delegated_agent_activity_from_executor_event(&state, &exec_event).await;
                    let sse_event = convert_executor_event_to_sse(exec_event);
                    let sender = sse_tx.read().await;
                    if sender.send(sse_event).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        label = log_label,
                        lagged = n,
                        "Executor event forwarder lagged, continuing"
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Bridge producer channel events into the hot-swappable SSE sender.
pub(crate) fn spawn_sse_channel_bridge(
    mut rx: tokio::sync::mpsc::Receiver<Result<Event, Infallible>>,
    sse_tx: Arc<RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
) {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let sender = sse_tx.read().await;
            if sender.send(event).await.is_err() {
                break;
            }
        }
    });
}
