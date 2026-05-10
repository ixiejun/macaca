//! Protocol-level IPC service bus contracts for Route C Phase 03.
//!
//! The service bus is the typed call plane between the microkernel facade and
//! replaceable system services.  These structs are data-only contracts: they do
//! not own transport execution, service lifecycle, or provider behavior.  That
//! separation lets local in-process dispatch stay fast today while future
//! child-process, MCP, HTTP, and signed remote A2A transports can reuse the
//! same envelope and reply shapes.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{KernelServiceId, ServiceCommand, TraceContext};

/// Unique identifier for one service bus envelope.
///
/// The id is a value object so trace, audit, retry, and future idempotency
/// stores can correlate a call without depending on transport-specific ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceEnvelopeId(pub Uuid);

impl ServiceEnvelopeId {
    /// Create a new random envelope id for a caller-originated service call.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ServiceEnvelopeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ServiceEnvelopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Provider-neutral identity of the component that submitted the call.
///
/// This is intentionally string-backed.  A source may be a kernel facade,
/// system service, application runtime, plugin host, CLI shell, gateway, or
/// future remote agent.  Policy evaluates the identity as data instead of the
/// bus branching on concrete caller types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ServiceBusSource(String);

impl ServiceBusSource {
    /// Create a source identity after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw source identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceBusSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Declarative permission scope attached to a service bus call.
///
/// Phase 03 does not implement the production permission backend, but the
/// envelope carries the scope now so later policy, entitlement, budget, and
/// region checks can be inserted without changing the transport contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PermissionScope(String);

impl PermissionScope {
    /// Create a permission scope after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw permission scope string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PermissionScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Idempotency key carried across retries and future remote transports.
///
/// The local Phase 03 bus preserves this key for trace and audit correlation.
/// Durable idempotency storage belongs to a later persistence/service phase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Create an idempotency key after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw idempotency key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Extensible transport classification for service bus calls.
///
/// `Custom` keeps the contract open for third-party or store-installed
/// transports without forcing a kernel enum edit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ServiceTransportKind {
    Local,
    ChildProcess,
    Mcp,
    Http,
    SignedRemoteA2a,
    Custom(String),
}

impl fmt::Display for ServiceTransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::ChildProcess => f.write_str("child_process"),
            Self::Mcp => f.write_str("mcp"),
            Self::Http => f.write_str("http"),
            Self::SignedRemoteA2a => f.write_str("signed_remote_a2a"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Typed service call envelope routed by the IPC service bus.
///
/// The envelope duplicates trace at the bus layer so routing, middleware, and
/// audit can run before a Phase 02 `ServiceCommand` reaches service logic.
/// If the command lacks trace but the envelope has one, local bridges may copy
/// the envelope trace into the command before dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceEnvelope {
    pub id: ServiceEnvelopeId,
    pub source: ServiceBusSource,
    pub target_service: KernelServiceId,
    pub command: ServiceCommand,
    pub trace: Option<TraceContext>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub permission_scope: Option<PermissionScope>,
    pub deadline: Option<DateTime<Utc>>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub metadata: BTreeMap<String, String>,
}

impl ServiceEnvelope {
    /// Create a new envelope and copy trace from the command when present.
    pub fn new(
        source: ServiceBusSource,
        target_service: KernelServiceId,
        command: ServiceCommand,
    ) -> Self {
        Self {
            id: ServiceEnvelopeId::new(),
            source,
            target_service,
            trace: command.trace.clone(),
            command,
            session_id: None,
            task_id: None,
            permission_scope: None,
            deadline: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Return the trace context visible to bus middleware.
    ///
    /// Envelope trace takes priority because transport middleware must be able
    /// to reason about correlation before service command adaptation.
    pub fn trace_context(&self) -> Option<TraceContext> {
        self.trace.clone().or_else(|| self.command.trace.clone())
    }

    /// Report whether the deadline has already expired at `now`.
    pub fn is_deadline_expired(&self, now: DateTime<Utc>) -> bool {
        self.deadline.is_some_and(|deadline| deadline <= now)
    }
}

/// Structured reply returned from any service bus transport.
///
/// The reply records transport and duration metadata at the bus boundary while
/// keeping service output as JSON-compatible data from the Phase 02 result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceReply {
    pub envelope_id: ServiceEnvelopeId,
    pub target_service: KernelServiceId,
    pub transport_kind: ServiceTransportKind,
    pub trace: Option<TraceContext>,
    pub success: bool,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<ServiceBusError>,
    pub duration_ms: u64,
    pub metadata: BTreeMap<String, String>,
}

impl ServiceReply {
    /// Build a successful reply for one completed envelope.
    pub fn success(
        envelope: &ServiceEnvelope,
        transport_kind: ServiceTransportKind,
        output: serde_json::Value,
        status: impl Into<String>,
    ) -> Self {
        Self {
            envelope_id: envelope.id,
            target_service: envelope.target_service.clone(),
            transport_kind,
            trace: envelope.trace_context(),
            success: true,
            status: status.into(),
            output: Some(output),
            error: None,
            duration_ms: 0,
            metadata: BTreeMap::new(),
        }
    }

    /// Build a failed reply without losing the structured bus error.
    pub fn failure(
        envelope: &ServiceEnvelope,
        transport_kind: ServiceTransportKind,
        error: ServiceBusError,
    ) -> Self {
        Self {
            envelope_id: envelope.id,
            target_service: envelope.target_service.clone(),
            transport_kind,
            trace: envelope.trace_context(),
            success: false,
            status: "error".into(),
            output: None,
            error: Some(error),
            duration_ms: 0,
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured errors emitted by the service bus boundary.
///
/// Expected boundary failures are data, not panics.  This lets Web UI,
/// EventLog, CLI inspectors, and future audit services explain what happened
/// without parsing provider-specific strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ServiceBusError {
    #[error("missing trace context")]
    MissingTraceContext,

    #[error("no route for service: {0}")]
    NoRoute(String),

    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),

    #[error("transport unavailable: {0}")]
    TransportUnavailable(String),

    #[error("deadline exceeded")]
    DeadlineExceeded,

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("dispatch failed: {0}")]
    DispatchFailed(String),

    #[error("service error: {0}")]
    ServiceError(String),

    #[error("reply decode error: {0}")]
    ReplyDecode(String),
}

/// Result alias for service bus boundary operations.
pub type ServiceBusResult<T> = Result<T, ServiceBusError>;

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::{ServiceCommand, ServiceCommandName};

    #[test]
    fn service_envelope_round_trips_with_context() {
        let trace = TraceContext::new("trace-bus-round-trip");
        let command = ServiceCommand::with_trace(
            ServiceCommandName::new("invoke"),
            serde_json::json!({"input": true}),
            trace.clone(),
        );
        let mut envelope = ServiceEnvelope::new(
            ServiceBusSource::new("kernel.facade"),
            KernelServiceId::new("service.test"),
            command,
        );
        envelope.session_id = Some("session-1".into());
        envelope.task_id = Some("task-1".into());
        envelope.permission_scope = Some(PermissionScope::new("service.test.invoke"));
        envelope.deadline = Some(Utc::now() + Duration::seconds(30));
        envelope.idempotency_key = Some(IdempotencyKey::new("idem-1"));
        envelope.metadata.insert("route".into(), "local".into());

        let encoded = serde_json::to_vec(&envelope).unwrap();
        let decoded: ServiceEnvelope = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.source.as_str(), "kernel.facade");
        assert_eq!(decoded.target_service.as_str(), "service.test");
        assert_eq!(decoded.session_id.as_deref(), Some("session-1"));
        assert_eq!(decoded.task_id.as_deref(), Some("task-1"));
        assert_eq!(
            decoded
                .permission_scope
                .as_ref()
                .map(PermissionScope::as_str),
            Some("service.test.invoke")
        );
        assert_eq!(
            decoded.idempotency_key.as_ref().map(IdempotencyKey::as_str),
            Some("idem-1")
        );
        assert_eq!(
            decoded.metadata.get("route").map(String::as_str),
            Some("local")
        );
        assert_eq!(
            decoded.trace_context().map(|trace| trace.trace_id),
            Some(trace.trace_id)
        );
    }

    #[test]
    fn service_reply_round_trips_with_structured_error() {
        let envelope = ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            KernelServiceId::new("service.missing"),
            ServiceCommand::without_trace(ServiceCommandName::new("call"), serde_json::json!({})),
        );
        let reply = ServiceReply::failure(
            &envelope,
            ServiceTransportKind::Local,
            ServiceBusError::MissingTraceContext,
        );

        let encoded = serde_json::to_string(&reply).unwrap();
        let decoded: ServiceReply = serde_json::from_str(&encoded).unwrap();

        assert!(!decoded.success);
        assert_eq!(decoded.error, Some(ServiceBusError::MissingTraceContext));
        assert_eq!(decoded.transport_kind, ServiceTransportKind::Local);
    }

    #[test]
    fn expired_deadline_is_data_not_panic() {
        let mut envelope = ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            KernelServiceId::new("service.deadline"),
            ServiceCommand::without_trace(ServiceCommandName::new("call"), serde_json::json!({})),
        );
        envelope.deadline = Some(Utc::now() - Duration::seconds(1));

        assert!(envelope.is_deadline_expired(Utc::now()));
        assert_eq!(
            ServiceBusError::DeadlineExceeded.to_string(),
            "deadline exceeded"
        );
    }
}
