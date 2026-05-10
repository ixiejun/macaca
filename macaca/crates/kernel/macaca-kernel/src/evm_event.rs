//! Trace/audit observer boundary for Optional EVM / DApp Module v0.
//!
//! EVM lifecycle events are intentionally bounded and redacted. They preserve
//! enough identity for operational audit while excluding private keys, seed
//! phrases, provider secrets, raw encrypted payloads, unbounded ABI arguments,
//! and unredacted signatures. Production integrations can forward these events
//! into the existing trace/event log without changing the EVM facade.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use macaca_proto::EvmError;

/// Trace/audit-compatible EVM lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmTraceEvent {
    pub chain_id: Option<String>,
    pub operation: String,
    pub status: String,
    pub request_id: Option<String>,
    pub contract_address: Option<String>,
    pub transaction_id: Option<String>,
    pub receipt_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub error_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Observer interface used by the EVM facade.
#[async_trait]
pub trait EvmTraceEventSink: Send + Sync {
    /// Record one redacted EVM lifecycle event.
    async fn emit(&self, event: EvmTraceEvent) -> Result<(), EvmError>;
}

/// No-op sink used when callers only need structured logs.
#[derive(Debug, Default)]
pub struct NoopEvmTraceEventSink;

#[async_trait]
impl EvmTraceEventSink for NoopEvmTraceEventSink {
    async fn emit(&self, _event: EvmTraceEvent) -> Result<(), EvmError> {
        Ok(())
    }
}

/// In-memory sink used by tests and diagnostics.
#[derive(Default)]
pub struct InMemoryEvmTraceEventSink {
    events: RwLock<Vec<EvmTraceEvent>>,
}

impl InMemoryEvmTraceEventSink {
    /// Create an empty event sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a stable snapshot of emitted events.
    pub async fn events(&self) -> Vec<EvmTraceEvent> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl EvmTraceEventSink for InMemoryEvmTraceEventSink {
    async fn emit(&self, event: EvmTraceEvent) -> Result<(), EvmError> {
        self.events.write().await.push(event);
        Ok(())
    }
}
