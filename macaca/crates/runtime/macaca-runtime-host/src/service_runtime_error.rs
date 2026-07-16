//! Error and configuration types for ServiceRuntime v1.
//!
//! Keeping boundary errors and configuration separate keeps the facade file
//! small and makes structured runtime failures easy to audit from tests and
//! future shell tools.

use std::{sync::Arc, time::Duration};

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

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("service bus error: {0}")]
    Bus(String),

    #[error("service call timed out after {timeout_ms} ms")]
    CallTimedOut { timeout_ms: u64 },

    #[error("service call cancelled by runtime token: {cancellation_token}")]
    CallCancelled { cancellation_token: String },

    #[error("service reply too large: {actual_bytes} bytes exceeds {max_bytes} bytes")]
    ReplyTooLarge {
        actual_bytes: usize,
        max_bytes: usize,
    },

    #[error("service stream frame limit exceeded: {actual_frames} frames exceeds {max_frames}")]
    StreamFrameLimitExceeded {
        actual_frames: usize,
        max_frames: usize,
    },

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
            ServiceBusError::InvalidArgument(reason) => Self::InvalidArgument(reason),
            ServiceBusError::DeadlineExceeded => Self::CallTimedOut { timeout_ms: 0 },
            other => Self::Bus(other.to_string()),
        }
    }
}

impl ServiceRuntimeError {
    /// Return whether the error came from runtime-owned call controls rather
    /// than provider logic.  The caller uses this to preserve service health
    /// while still emitting terminal audit evidence for the call.
    pub(crate) fn is_runtime_control_failure(&self) -> bool {
        matches!(self, Self::CallTimedOut { .. } | Self::CallCancelled { .. })
    }
}

/// ServiceRuntime configuration.
#[derive(Clone)]
pub struct ServiceRuntimeConfig {
    pub source: ServiceBusSource,
    pub policy: Arc<dyn ServiceRuntimePolicy>,
    pub event_sink: Option<Arc<dyn ServiceRuntimeEventSink>>,
    pub bus_trace_sink: Option<Arc<InMemoryServiceBusTraceSink>>,
    pub call_timeout: Option<Duration>,
    pub max_reply_output_bytes: usize,
    pub max_stream_frames: usize,
}

impl Default for ServiceRuntimeConfig {
    fn default() -> Self {
        Self {
            source: ServiceBusSource::new("runtime.host"),
            policy: Arc::new(AllowAllServiceRuntimePolicy),
            event_sink: None,
            bus_trace_sink: None,
            call_timeout: Some(Duration::from_secs(120)),
            max_reply_output_bytes: 4 * 1024 * 1024,
            max_stream_frames: 2048,
        }
    }
}
