//! Trace event forwarding boundary for future web event-path migration.

use std::sync::Arc;

use axum::response::sse::Event;
use macaca_persist::EventLog;
use serde_json::Value;
use tokio::sync::mpsc;

/// Normalizes trace payloads before persistence or SSE forwarding.
#[derive(Debug, Default, Clone, Copy)]
pub struct TraceEventNormalizer;

impl TraceEventNormalizer {
    /// Preserve the existing payload shape.
    pub fn normalize(&self, payload: Value) -> Value {
        payload
    }
}

/// Facade for durable trace writes and best-effort SSE fanout.
pub struct TraceEventForwarder {
    event_log: Arc<EventLog>,
    normalizer: TraceEventNormalizer,
}

impl TraceEventForwarder {
    /// Create a forwarder around the existing append-only event log.
    pub fn new(event_log: Arc<EventLog>) -> Self {
        Self {
            event_log,
            normalizer: TraceEventNormalizer,
        }
    }

    /// Normalize a payload for callers that have not migrated to the full facade.
    pub fn normalize(&self, payload: Value) -> Value {
        self.normalizer.normalize(payload)
    }

    /// Append a JSON trace event to the durable log.
    pub async fn append_json(
        &self,
        session_id: &str,
        event_type: &str,
        source: &str,
        payload: Value,
    ) -> u64 {
        let payload = self.normalize(payload);
        self.event_log
            .append(session_id, event_type, source, payload)
            .await
    }

    /// Forward a prebuilt SSE event without changing existing channel semantics.
    pub async fn send_sse(
        &self,
        tx: &mpsc::Sender<Result<Event, std::convert::Infallible>>,
        event: Event,
    ) -> bool {
        tx.send(Ok(event)).await.is_ok()
    }
}
