//! SDK trace client boundary for shell-facing replay and tail operations.
//!
//! Trace remains an Observer/Memento concern. S3 models the command and client
//! boundary without requiring Web or CLI to own trace semantics.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use macaca_proto::{MacacaError, MacacaResult};

/// Replay command for shell-facing session event reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventQueryCommand {
    pub session_id: String,
    pub since: Option<u64>,
    pub limit: Option<usize>,
}

impl SessionEventQueryCommand {
    /// Create a bounded session event query.
    pub fn new(
        session_id: impl Into<String>,
        since: Option<u64>,
        limit: Option<usize>,
    ) -> MacacaResult<Self> {
        let session_id = session_id.into().trim().to_string();
        if session_id.is_empty() {
            return Err(MacacaError::Config(
                "session event query requires non-empty session_id".into(),
            ));
        }
        Ok(Self {
            session_id,
            since,
            limit,
        })
    }
}

/// Cursor command for shell-facing trace tail operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceTailCommand {
    pub session_id: String,
    pub since: Option<u64>,
}

impl TraceTailCommand {
    /// Create a session-scoped trace tail command.
    pub fn new(session_id: impl Into<String>, since: Option<u64>) -> MacacaResult<Self> {
        let session_id = session_id.into().trim().to_string();
        if session_id.is_empty() {
            return Err(MacacaError::Config(
                "trace tail command requires non-empty session_id".into(),
            ));
        }
        Ok(Self { session_id, since })
    }
}

/// Trace replay result that can be backed by EventLog or service runtime later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceQueryResult {
    pub events: Vec<Value>,
    pub next_cursor: Option<u64>,
}

/// Replaceable trace client.
#[async_trait]
pub trait SystemTraceClient: Send + Sync {
    /// Replay historical events for one session.
    async fn replay_events(
        &self,
        command: &SessionEventQueryCommand,
    ) -> MacacaResult<TraceQueryResult>;

    /// Tail trace events for one session.
    async fn tail_trace(&self, command: &TraceTailCommand) -> MacacaResult<TraceQueryResult>;
}

/// Empty local trace client for the S3 protocol slice.
#[derive(Debug, Default, Clone)]
pub struct EmptySystemTraceClient;

#[async_trait]
impl SystemTraceClient for EmptySystemTraceClient {
    async fn replay_events(
        &self,
        command: &SessionEventQueryCommand,
    ) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            limit = ?command.limit,
            "sdk trace client returning empty replay result"
        );
        Ok(TraceQueryResult {
            events: Vec::new(),
            next_cursor: command.since,
        })
    }

    async fn tail_trace(&self, command: &TraceTailCommand) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            "sdk trace client returning empty tail result"
        );
        Ok(TraceQueryResult {
            events: Vec::new(),
            next_cursor: command.since,
        })
    }
}
