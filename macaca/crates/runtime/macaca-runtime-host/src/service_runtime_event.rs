//! Event and snapshot types for the host-owned ServiceRuntime.
//!
//! These types implement the Observer and Memento sides of ServiceRuntime v1.
//! The runtime emits structured events for auditability and returns
//! deterministic snapshots for diagnostics without exposing concrete provider
//! internals.

use std::sync::RwLock;

use chrono::{DateTime, Utc};
use macaca_proto::{KernelServiceId, ServiceDescriptor, ServiceHealth, ServiceLifecycleState};

use crate::service_runtime_error::ServiceRuntimeError;

/// Audit-friendly event emitted by ServiceRuntime.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceRuntimeEvent {
    pub service_id: KernelServiceId,
    pub operation: String,
    pub lifecycle_state: ServiceLifecycleState,
    pub health: Option<ServiceHealth>,
    pub trace_id: Option<String>,
    pub emitted_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

/// Observer contract for runtime lifecycle and call events.
pub trait ServiceRuntimeEventSink: Send + Sync {
    /// Persist or forward one runtime event.
    fn emit(&self, event: ServiceRuntimeEvent) -> Result<(), ServiceRuntimeError>;
}

/// In-memory event sink for tests and local diagnostics.
#[derive(Default)]
pub struct InMemoryServiceRuntimeEventSink {
    events: RwLock<Vec<ServiceRuntimeEvent>>,
}

impl InMemoryServiceRuntimeEventSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a stable snapshot of emitted events.
    pub fn events(&self) -> Result<Vec<ServiceRuntimeEvent>, ServiceRuntimeError> {
        let events = self
            .events
            .read()
            .map_err(|_| ServiceRuntimeError::State("runtime event sink lock poisoned".into()))?;
        Ok(events.clone())
    }
}

impl ServiceRuntimeEventSink for InMemoryServiceRuntimeEventSink {
    fn emit(&self, event: ServiceRuntimeEvent) -> Result<(), ServiceRuntimeError> {
        let mut events = self
            .events
            .write()
            .map_err(|_| ServiceRuntimeError::State("runtime event sink lock poisoned".into()))?;
        events.push(event);
        Ok(())
    }
}

/// Deterministic snapshot for one registered service.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceRuntimeServiceSnapshot {
    pub descriptor: ServiceDescriptor,
    pub lifecycle_state: ServiceLifecycleState,
    pub health: ServiceHealth,
    pub failure_reason: Option<String>,
}

/// Deterministic runtime memento for diagnostics and future inspectors.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceRuntimeSnapshot {
    pub services: Vec<ServiceRuntimeServiceSnapshot>,
}
