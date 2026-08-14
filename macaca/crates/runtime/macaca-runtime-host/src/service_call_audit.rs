//! Structured audit trail for generic `service.call` routing.
//!
//! This module captures the complete service-call evidence chain in a
//! provider-neutral, application-agnostic format that can be replayed by
//! `trace_id` and `session_id`.

use std::collections::BTreeMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};

use crate::service_runtime_error::ServiceRuntimeError;

/// One immutable audit event emitted during service routing lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCallAuditEvent {
    pub stage: String,
    pub trace_id: String,
    pub session_id: Option<String>,
    pub app_id: Option<String>,
    pub service_id: String,
    /// Canonical operation identifier required to interpret a replay entry.
    pub operation: Option<String>,
    pub provider_id: Option<String>,
    pub decision: Option<String>,
    pub retry_count: Option<u32>,
    pub latency_ms: Option<u64>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    /// Explicitly allowlisted provider facts, never provider output or input.
    pub replay_metadata: BTreeMap<String, String>,
    pub emitted_at: DateTime<Utc>,
}

impl ServiceCallAuditEvent {
    /// Build a minimal event and fill optional fields incrementally.
    pub fn new(
        stage: impl Into<String>,
        trace_id: impl Into<String>,
        service_id: impl Into<String>,
    ) -> Self {
        Self {
            stage: stage.into(),
            trace_id: trace_id.into(),
            session_id: None,
            app_id: None,
            service_id: service_id.into(),
            operation: None,
            provider_id: None,
            decision: None,
            retry_count: None,
            latency_ms: None,
            input_hash: None,
            output_hash: None,
            replay_metadata: BTreeMap::new(),
            emitted_at: Utc::now(),
        }
    }
}

/// Observer contract for route-level audit events.
pub trait ServiceCallAuditSink: Send + Sync {
    /// Persist or forward one audit event.
    fn emit(&self, event: ServiceCallAuditEvent) -> Result<(), ServiceRuntimeError>;

    /// Query audit chain by trace id. Default returns unsupported.
    fn replay_by_trace_id(
        &self,
        _trace_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        Err(ServiceRuntimeError::State(
            "service call audit replay by trace id is unavailable".into(),
        ))
    }

    /// Query audit chain by session id. Default returns unsupported.
    fn replay_by_session_id(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        Err(ServiceRuntimeError::State(
            "service call audit replay by session id is unavailable".into(),
        ))
    }
}

/// In-memory sink for tests and local diagnostics.
#[derive(Default)]
pub struct InMemoryServiceCallAuditSink {
    events: RwLock<Vec<ServiceCallAuditEvent>>,
}

impl InMemoryServiceCallAuditSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all emitted events.
    pub fn events(&self) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        let events = self.events.read().map_err(|_| {
            ServiceRuntimeError::State("service call audit sink lock poisoned".into())
        })?;
        Ok(events.clone())
    }

    /// Replay one session evidence chain by `trace_id`.
    pub fn replay_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        let events = self.events()?;
        Ok(events
            .into_iter()
            .filter(|event| event.trace_id == trace_id)
            .collect())
    }

    /// Replay one session evidence chain by `session_id`.
    pub fn replay_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        let events = self.events()?;
        Ok(events
            .into_iter()
            .filter(|event| event.session_id.as_deref() == Some(session_id))
            .collect())
    }
}

impl ServiceCallAuditSink for InMemoryServiceCallAuditSink {
    fn emit(&self, event: ServiceCallAuditEvent) -> Result<(), ServiceRuntimeError> {
        let mut events = self.events.write().map_err(|_| {
            ServiceRuntimeError::State("service call audit sink lock poisoned".into())
        })?;
        events.push(event);
        Ok(())
    }

    fn replay_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        InMemoryServiceCallAuditSink::replay_by_trace_id(self, trace_id)
    }

    fn replay_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        InMemoryServiceCallAuditSink::replay_by_session_id(self, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_filters_events_by_trace_id() {
        let sink = InMemoryServiceCallAuditSink::new();
        sink.emit(ServiceCallAuditEvent::new(
            "service_call_requested",
            "trace-a",
            "service.example",
        ))
        .unwrap();
        sink.emit(ServiceCallAuditEvent::new(
            "service_call_succeeded",
            "trace-b",
            "service.example",
        ))
        .unwrap();
        let replay = sink.replay_by_trace_id("trace-a").unwrap();
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].trace_id, "trace-a");
        assert_eq!(replay[0].stage, "service_call_requested");
    }

    #[test]
    fn replay_filters_events_by_session_id() {
        let sink = InMemoryServiceCallAuditSink::new();
        let mut event_a =
            ServiceCallAuditEvent::new("service_call_requested", "trace-a", "service.example");
        event_a.session_id = Some("session-a".into());
        sink.emit(event_a).unwrap();
        let mut event_b =
            ServiceCallAuditEvent::new("service_call_succeeded", "trace-b", "service.example");
        event_b.session_id = Some("session-b".into());
        sink.emit(event_b).unwrap();
        let replay = sink.replay_by_session_id("session-a").unwrap();
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].session_id.as_deref(), Some("session-a"));
    }
}
