//! Trace event forwarding boundary for future web event-path migration.

use std::sync::Arc;

use async_trait::async_trait;
use axum::response::sse::Event;
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

/// Provider-neutral port for durable trace-event writes.
///
/// The Web shell owns normalization and SSE fanout, but durable persistence is
/// supplied by the host composition root. This Command-style port keeps the
/// trace forwarding facade independent from concrete persistence backends.
#[async_trait]
pub trait TraceEventSink: Send + Sync {
    /// Append a normalized JSON trace event and return the durable sequence id.
    async fn append_json(
        &self,
        session_id: &str,
        event_type: &str,
        source: &str,
        payload: Value,
    ) -> u64;
}

/// Facade for durable trace writes and best-effort SSE fanout.
pub struct TraceEventForwarder {
    sink: Arc<dyn TraceEventSink>,
    normalizer: TraceEventNormalizer,
}

impl TraceEventForwarder {
    /// Create a forwarder around a host-installed trace sink.
    pub fn new(sink: Arc<dyn TraceEventSink>) -> Self {
        Self {
            sink,
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
        self.sink
            .append_json(session_id, event_type, source, payload)
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
