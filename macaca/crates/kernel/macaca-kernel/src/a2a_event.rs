//! Trace/audit event observer boundary for A2A Payment v0.
//!
//! The kernel emits payment lifecycle observations through this small Observer
//! contract. The default in-memory sink is test-oriented; production systems
//! can bridge the same bounded event payload into EventLog, audit storage, SSE,
//! or external compliance pipelines without changing the coordinator.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use macaca_proto::A2AError;

/// Trace/audit-compatible payment event emitted by the coordinator.
///
/// The event intentionally stores bounded identifiers and metadata only. It
/// must never contain provider credentials, private keys, wallet secrets, raw
/// encrypted payloads, or other sensitive material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2APaymentEvent {
    pub requester_id: String,
    pub provider_id: String,
    pub capability_id: String,
    pub quote_id: Option<String>,
    pub intent_id: Option<String>,
    pub amount: Option<String>,
    pub asset: Option<String>,
    pub operation: String,
    pub status: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub error_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Observer interface for payment lifecycle events.
///
/// Implementations must treat the payload as already redacted. They may enrich
/// routing metadata, but they should not query adapters for secrets or raw
/// credentials during event emission.
#[async_trait]
pub trait A2APaymentEventSink: Send + Sync {
    /// Record one bounded payment lifecycle event.
    async fn emit(&self, event: A2APaymentEvent) -> Result<(), A2AError>;
}

/// No-op sink used when callers only need structured logs.
#[derive(Debug, Default)]
pub struct NoopA2APaymentEventSink;

#[async_trait]
impl A2APaymentEventSink for NoopA2APaymentEventSink {
    async fn emit(&self, _event: A2APaymentEvent) -> Result<(), A2AError> {
        Ok(())
    }
}

/// In-memory event sink used by tests and local diagnostics.
#[derive(Default)]
pub struct InMemoryA2APaymentEventSink {
    events: RwLock<Vec<A2APaymentEvent>>,
}

impl InMemoryA2APaymentEventSink {
    /// Create an empty event sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a stable snapshot of all events emitted so far.
    pub async fn events(&self) -> Vec<A2APaymentEvent> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl A2APaymentEventSink for InMemoryA2APaymentEventSink {
    async fn emit(&self, event: A2APaymentEvent) -> Result<(), A2AError> {
        self.events.write().await.push(event);
        Ok(())
    }
}
