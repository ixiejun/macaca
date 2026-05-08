//! Error and configuration types for ServiceRuntime v1.
//!
//! Keeping boundary errors and configuration separate keeps the facade file
//! small and makes structured runtime failures easy to audit from tests and
//! future shell tools.

use std::sync::Arc;

use macaca_ipc::InMemoryServiceBusTraceSink;
use macaca_proto::{ServiceBusError, ServiceBusSource};
use thiserror::Error;

use crate::service_decorator::{AllowAllServiceRuntimePolicy, ServiceRuntimePolicy};
use crate::service_runtime_event::ServiceRuntimeEventSink;

/// Structured errors emitted at the ServiceRuntime boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServiceRuntimeError {
    #[error("service already registered: {0}")]
    DuplicateService(String),

    #[error("unknown service: {0}")]
    UnknownService(String),

    #[error("missing trace context")]
    MissingTraceContext,

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("service bus error: {0}")]
    Bus(String),

    #[error("service operation failed: {0}")]
    Service(String),

    #[error("runtime state unavailable: {0}")]
    State(String),
}

impl From<ServiceBusError> for ServiceRuntimeError {
    fn from(value: ServiceBusError) -> Self {
        match value {
            ServiceBusError::MissingTraceContext => Self::MissingTraceContext,
            ServiceBusError::PolicyDenied(reason) => Self::PolicyDenied(reason),
            other => Self::Bus(other.to_string()),
        }
    }
}

/// ServiceRuntime configuration.
#[derive(Clone)]
pub struct ServiceRuntimeConfig {
    pub source: ServiceBusSource,
    pub policy: Arc<dyn ServiceRuntimePolicy>,
    pub event_sink: Option<Arc<dyn ServiceRuntimeEventSink>>,
    pub bus_trace_sink: Option<Arc<InMemoryServiceBusTraceSink>>,
}

impl Default for ServiceRuntimeConfig {
    fn default() -> Self {
        Self {
            source: ServiceBusSource::new("runtime.host"),
            policy: Arc::new(AllowAllServiceRuntimePolicy),
            event_sink: None,
            bus_trace_sink: None,
        }
    }
}
