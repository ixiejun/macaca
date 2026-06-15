//! Provider-neutral event-log command DTOs.
//!
//! Persistence backends own storage mechanics, but shells and services need a
//! stable Command object for append-only audit/event evidence. Keeping this DTO
//! in `macaca-proto` prevents presentation shells from importing runtime-host
//! persistence internals just to describe an append operation.

use serde::{Deserialize, Serialize};

/// Command object for appending a single sanitized event-log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEventCommand {
    /// Session scope that owns the append-only event sequence.
    pub session_id: String,
    /// Provider-neutral event type used for replay and filtering.
    pub event_type: String,
    /// Bounded source identifier for audit attribution.
    pub source: String,
    /// Sanitized event payload; callers must not include raw secrets or prompts.
    pub payload: serde_json::Value,
    /// Optional application scope for indexed queries.
    pub app_id: Option<String>,
    /// Optional agent scope for indexed queries.
    pub agent_name: Option<String>,
}

impl AppendEventCommand {
    /// Build a command with required replay identity and sanitized payload.
    pub fn new(
        session_id: impl Into<String>,
        event_type: impl Into<String>,
        source: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            event_type: event_type.into(),
            source: source.into(),
            payload,
            app_id: None,
            agent_name: None,
        }
    }

    /// Attach application scope for indexed replay queries.
    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    /// Attach agent scope for indexed replay queries.
    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }
}

/// Query object for replaying sanitized event-log entries.
///
/// The event-log backend owns index selection and storage, while shells and
/// services can use this provider-neutral query contract without importing a
/// concrete persistence crate or runtime host.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventLogQuery {
    /// Session scope to replay from.
    pub session_id: String,
    /// Exclusive lower bound for per-session sequence numbers.
    pub since_seq: u64,
    /// Maximum number of entries to return.
    pub limit: usize,
    /// Optional source filter for indexed replay.
    pub source: Option<String>,
    /// Optional agent filter for indexed replay.
    pub agent: Option<String>,
    /// Optional event type filter for indexed replay.
    pub event_type: Option<String>,
}

impl EventLogQuery {
    /// Build a query for the provided session with the default bounded page size.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            since_seq: 0,
            limit: 500,
            source: None,
            agent: None,
            event_type: None,
        }
    }

    /// Set the exclusive sequence lower bound.
    pub fn since(mut self, since_seq: u64) -> Self {
        self.since_seq = since_seq;
        self
    }

    /// Set the maximum returned event count.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Filter events by source.
    pub fn source(mut self, source: Option<String>) -> Self {
        self.source = source;
        self
    }

    /// Filter events by agent identity.
    pub fn agent(mut self, agent: Option<String>) -> Self {
        self.agent = agent;
        self
    }

    /// Filter events by event type.
    pub fn event_type(mut self, event_type: Option<String>) -> Self {
        self.event_type = event_type;
        self
    }
}
