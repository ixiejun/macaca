//! Trace/audit observer boundary for Optional Web3 Node v0.
//!
//! The event payload is intentionally bounded and redacted. It carries stable
//! identifiers needed for audit and UI display, but it never carries private
//! keys, seed phrases, credentials, raw encrypted payloads, or provider
//! secrets. Production bridges can forward this event into EventLog or an
//! external compliance sink without changing the Web3 facade.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use macaca_proto::Web3Error;

/// Trace/audit-compatible Web3 lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Web3TraceEvent {
    pub wallet_id: Option<String>,
    pub chain_id: Option<String>,
    pub capability: String,
    pub operation: String,
    pub status: String,
    pub request_id: Option<String>,
    pub transaction_id: Option<String>,
    pub receipt_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub error_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Observer interface used by the Web3 facade.
#[async_trait]
pub trait Web3TraceEventSink: Send + Sync {
    /// Record one redacted Web3 lifecycle event.
    async fn emit(&self, event: Web3TraceEvent) -> Result<(), Web3Error>;
}

/// No-op sink used when callers only need structured logs.
#[derive(Debug, Default)]
pub struct NoopWeb3TraceEventSink;

#[async_trait]
impl Web3TraceEventSink for NoopWeb3TraceEventSink {
    async fn emit(&self, _event: Web3TraceEvent) -> Result<(), Web3Error> {
        Ok(())
    }
}

/// In-memory event sink used by tests and diagnostics.
#[derive(Default)]
pub struct InMemoryWeb3TraceEventSink {
    events: RwLock<Vec<Web3TraceEvent>>,
}

impl InMemoryWeb3TraceEventSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a stable snapshot of emitted events.
    pub async fn events(&self) -> Vec<Web3TraceEvent> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl Web3TraceEventSink for InMemoryWeb3TraceEventSink {
    async fn emit(&self, event: Web3TraceEvent) -> Result<(), Web3Error> {
        self.events.write().await.push(event);
        Ok(())
    }
}
