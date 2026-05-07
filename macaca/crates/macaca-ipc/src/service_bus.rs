//! Service bus facade for typed system service calls.
//!
//! The facade applies the Facade + Chain of Responsibility + Observer
//! patterns.  Callers submit a `ServiceEnvelope`; middleware validates common
//! invariants such as trace, deadline, and policy; the selected transport owns
//! dispatch; and a trace sink records bus-level lifecycle events without
//! coupling the IPC crate to Web SSE, EventLog, or CLI presentation code.

use std::sync::{Arc, RwLock};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{
    ServiceBusError, ServiceBusResult, ServiceEnvelope, ServiceEnvelopeId, ServiceReply,
    ServiceTransportKind, TraceContext,
};

use crate::transport::ServiceTransport;

/// Presentation-neutral trace/audit event emitted by the service bus.
///
/// The event payload is schema-flexible JSON so later EventLog, Web UI, CLI,
/// or external audit services can render bus activity without the bus knowing
/// about those consumers.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceBusTraceEvent {
    pub envelope_id: ServiceEnvelopeId,
    pub event_type: String,
    pub target_service: String,
    pub transport_kind: ServiceTransportKind,
    pub trace: Option<TraceContext>,
    pub emitted_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

/// Observer boundary for bus-level trace and audit events.
pub trait ServiceBusTraceSink: Send + Sync {
    /// Emit one bus-level trace event.
    fn emit(&self, event: ServiceBusTraceEvent) -> ServiceBusResult<()>;
}

/// In-memory trace sink used by tests and local diagnostics.
///
/// Production code can bridge the same observer contract to EventLog, SSE,
/// OpenTelemetry, or a future audit service without changing the bus facade.
#[derive(Default)]
pub struct InMemoryServiceBusTraceSink {
    events: RwLock<Vec<ServiceBusTraceEvent>>,
}

impl InMemoryServiceBusTraceSink {
    /// Create an empty trace sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a snapshot of emitted bus events.
    pub fn events(&self) -> ServiceBusResult<Vec<ServiceBusTraceEvent>> {
        let events = self.events.read().map_err(|_| {
            ServiceBusError::DispatchFailed("service bus trace sink lock poisoned".into())
        })?;
        Ok(events.clone())
    }
}

impl ServiceBusTraceSink for InMemoryServiceBusTraceSink {
    fn emit(&self, event: ServiceBusTraceEvent) -> ServiceBusResult<()> {
        let mut events = self.events.write().map_err(|_| {
            ServiceBusError::DispatchFailed("service bus trace sink lock poisoned".into())
        })?;
        events.push(event);
        Ok(())
    }
}

/// Middleware context available before service dispatch.
///
/// The context intentionally exposes only envelope and transport metadata so
/// cross-cutting checks cannot depend on concrete service implementation
/// details or application-specific workflow state.
pub struct ServiceBusContext<'a> {
    pub envelope: &'a ServiceEnvelope,
    pub transport_kind: ServiceTransportKind,
}

/// Ordered middleware contract for service bus invariants.
#[async_trait]
pub trait ServiceBusMiddleware: Send + Sync {
    /// Validate or observe an envelope before transport dispatch.
    async fn before_dispatch(&self, context: &ServiceBusContext<'_>) -> ServiceBusResult<()>;
}

/// Middleware that enforces the Route C "no trace, no call" rule.
pub struct TraceRequiredBusMiddleware;

#[async_trait]
impl ServiceBusMiddleware for TraceRequiredBusMiddleware {
    async fn before_dispatch(&self, context: &ServiceBusContext<'_>) -> ServiceBusResult<()> {
        if context.envelope.trace_context().is_none() {
            tracing::warn!(
                envelope_id = %context.envelope.id,
                target_service = %context.envelope.target_service,
                command = %context.envelope.command.name,
                "service bus call rejected before routing: missing trace context"
            );
            return Err(ServiceBusError::MissingTraceContext);
        }
        Ok(())
    }
}

/// Middleware that rejects envelopes whose deadline has already expired.
pub struct DeadlineBusMiddleware;

#[async_trait]
impl ServiceBusMiddleware for DeadlineBusMiddleware {
    async fn before_dispatch(&self, context: &ServiceBusContext<'_>) -> ServiceBusResult<()> {
        if context.envelope.is_deadline_expired(Utc::now()) {
            tracing::warn!(
                envelope_id = %context.envelope.id,
                target_service = %context.envelope.target_service,
                command = %context.envelope.command.name,
                "service bus call rejected before dispatch: deadline exceeded"
            );
            return Err(ServiceBusError::DeadlineExceeded);
        }
        Ok(())
    }
}

/// Test and compatibility policy middleware that denies every call.
///
/// Production policy, budget, region, entitlement, and payment checks will use
/// the same middleware insertion point.  This implementation proves denial is
/// structured data and prevents dispatch.
pub struct StaticDenyBusMiddleware {
    reason: String,
}

impl StaticDenyBusMiddleware {
    /// Create a middleware that denies calls with a stable reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ServiceBusMiddleware for StaticDenyBusMiddleware {
    async fn before_dispatch(&self, context: &ServiceBusContext<'_>) -> ServiceBusResult<()> {
        tracing::warn!(
            envelope_id = %context.envelope.id,
            target_service = %context.envelope.target_service,
            command = %context.envelope.command.name,
            reason = %self.reason,
            "service bus call rejected by policy middleware"
        );
        Err(ServiceBusError::PolicyDenied(self.reason.clone()))
    }
}

/// Stable facade for routed service bus calls.
///
/// The bus owns middleware sequencing and trace/audit emission.  It does not
/// own service lifecycle or provider execution; those remain behind transport
/// and Phase 02 `SystemService` adapters.
pub struct ServiceBus {
    transport: Arc<dyn ServiceTransport>,
    middleware: Vec<Box<dyn ServiceBusMiddleware>>,
    trace_sink: Option<Arc<dyn ServiceBusTraceSink>>,
}

impl ServiceBus {
    /// Create a bus with trace and deadline middleware installed first.
    pub fn new(transport: Arc<dyn ServiceTransport>) -> Self {
        Self {
            transport,
            middleware: vec![
                Box::new(TraceRequiredBusMiddleware),
                Box::new(DeadlineBusMiddleware),
            ],
            trace_sink: None,
        }
    }

    /// Add middleware after trace and deadline validation.
    pub fn with_middleware(mut self, middleware: Box<dyn ServiceBusMiddleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Attach a presentation-neutral trace sink for bus lifecycle events.
    pub fn with_trace_sink(mut self, trace_sink: Arc<dyn ServiceBusTraceSink>) -> Self {
        self.trace_sink = Some(trace_sink);
        self
    }

    /// Dispatch one envelope through middleware and the selected transport.
    pub async fn call(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply> {
        let transport_kind = self.transport.kind();
        let started = Instant::now();
        tracing::info!(
            envelope_id = %envelope.id,
            source = %envelope.source,
            target_service = %envelope.target_service,
            command = %envelope.command.name,
            transport_kind = %transport_kind,
            "service bus call accepted"
        );
        self.emit(
            &envelope,
            transport_kind.clone(),
            "service_bus.call.accepted",
            serde_json::json!({"status": "accepted"}),
        )?;

        let context = ServiceBusContext {
            envelope: &envelope,
            transport_kind: transport_kind.clone(),
        };
        for middleware in &self.middleware {
            if let Err(err) = middleware.before_dispatch(&context).await {
                let event_type = if matches!(err, ServiceBusError::DeadlineExceeded) {
                    "service_bus.call.timed_out"
                } else {
                    "service_bus.call.rejected"
                };
                self.emit(
                    &envelope,
                    transport_kind.clone(),
                    event_type,
                    serde_json::json!({
                        "status": "rejected",
                        "error": err.to_string(),
                    }),
                )?;
                return Err(err);
            }
        }

        tracing::info!(
            envelope_id = %envelope.id,
            target_service = %envelope.target_service,
            transport_kind = %transport_kind,
            "service bus route selected"
        );
        self.emit(
            &envelope,
            transport_kind.clone(),
            "service_bus.call.routed",
            serde_json::json!({"status": "routed"}),
        )?;

        match self.transport.call(envelope.clone()).await {
            Ok(mut reply) => {
                reply.duration_ms = started.elapsed().as_millis() as u64;
                tracing::info!(
                    envelope_id = %envelope.id,
                    target_service = %envelope.target_service,
                    transport_kind = %transport_kind,
                    duration_ms = reply.duration_ms,
                    "service bus call completed"
                );
                self.emit(
                    &envelope,
                    transport_kind,
                    "service_bus.call.completed",
                    serde_json::json!({
                        "status": reply.status,
                        "duration_ms": reply.duration_ms,
                    }),
                )?;
                Ok(reply)
            }
            Err(err) => {
                tracing::warn!(
                    envelope_id = %envelope.id,
                    target_service = %envelope.target_service,
                    transport_kind = %transport_kind,
                    error = %err,
                    "service bus call failed"
                );
                self.emit(
                    &envelope,
                    transport_kind,
                    "service_bus.call.failed",
                    serde_json::json!({
                        "status": "failed",
                        "error": err.to_string(),
                    }),
                )?;
                Err(err)
            }
        }
    }

    fn emit(
        &self,
        envelope: &ServiceEnvelope,
        transport_kind: ServiceTransportKind,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> ServiceBusResult<()> {
        if let Some(trace_sink) = &self.trace_sink {
            trace_sink.emit(ServiceBusTraceEvent {
                envelope_id: envelope.id,
                event_type: event_type.into(),
                target_service: envelope.target_service.to_string(),
                transport_kind,
                trace: envelope.trace_context(),
                emitted_at: Utc::now(),
                payload,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Duration;
    use macaca_proto::{
        KernelServiceId, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceEnvelope,
        TraceContext,
    };

    use crate::local_service::{LocalServiceHandler, LocalServiceTransport};

    use super::*;

    struct EchoHandler;

    #[async_trait]
    impl LocalServiceHandler for EchoHandler {
        async fn handle(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply> {
            Ok(ServiceReply::success(
                &envelope,
                ServiceTransportKind::Local,
                envelope.command.payload.clone(),
                "ok",
            ))
        }
    }

    struct MockRemoteTransport;

    #[async_trait]
    impl ServiceTransport for MockRemoteTransport {
        fn kind(&self) -> ServiceTransportKind {
            ServiceTransportKind::Custom("mock_remote".into())
        }

        async fn call(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply> {
            Ok(ServiceReply::success(
                &envelope,
                self.kind(),
                serde_json::json!({"remote": true}),
                "ok",
            ))
        }
    }

    fn traced_envelope(service_id: KernelServiceId) -> ServiceEnvelope {
        ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            service_id,
            ServiceCommand::with_trace(
                ServiceCommandName::new("echo"),
                serde_json::json!({"hello": "bus"}),
                TraceContext::new("trace-service-bus"),
            ),
        )
    }

    #[tokio::test]
    async fn service_bus_dispatches_and_emits_trace_events() {
        let transport = Arc::new(LocalServiceTransport::new());
        let service_id = KernelServiceId::new("service.echo");
        transport
            .register_handler(service_id.clone(), Arc::new(EchoHandler))
            .unwrap();
        let trace_sink = Arc::new(InMemoryServiceBusTraceSink::new());
        let bus = ServiceBus::new(transport).with_trace_sink(trace_sink.clone());

        let reply = bus.call(traced_envelope(service_id)).await.unwrap();

        assert!(reply.success);
        assert_eq!(reply.transport_kind, ServiceTransportKind::Local);
        let events = trace_sink.events().unwrap();
        assert!(events
            .iter()
            .any(|event| event.event_type == "service_bus.call.accepted"));
        assert!(events
            .iter()
            .any(|event| event.event_type == "service_bus.call.routed"));
        assert!(events
            .iter()
            .any(|event| event.event_type == "service_bus.call.completed"));
    }

    #[tokio::test]
    async fn missing_trace_is_rejected_before_dispatch() {
        let transport = Arc::new(LocalServiceTransport::new());
        let service_id = KernelServiceId::new("service.echo");
        transport
            .register_handler(service_id.clone(), Arc::new(EchoHandler))
            .unwrap();
        let trace_sink = Arc::new(InMemoryServiceBusTraceSink::new());
        let bus = ServiceBus::new(transport).with_trace_sink(trace_sink.clone());
        let envelope = ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            service_id,
            ServiceCommand::without_trace(
                ServiceCommandName::new("echo"),
                serde_json::json!({"should_not": "dispatch"}),
            ),
        );

        let err = bus.call(envelope).await.unwrap_err();

        assert_eq!(err, ServiceBusError::MissingTraceContext);
        assert!(trace_sink
            .events()
            .unwrap()
            .iter()
            .any(|event| event.event_type == "service_bus.call.rejected"));
    }

    #[tokio::test]
    async fn policy_denial_prevents_dispatch() {
        let transport = Arc::new(LocalServiceTransport::new());
        let service_id = KernelServiceId::new("service.echo");
        transport
            .register_handler(service_id.clone(), Arc::new(EchoHandler))
            .unwrap();
        let bus = ServiceBus::new(transport)
            .with_middleware(Box::new(StaticDenyBusMiddleware::new("budget exceeded")));

        let err = bus.call(traced_envelope(service_id)).await.unwrap_err();

        assert_eq!(err, ServiceBusError::PolicyDenied("budget exceeded".into()));
    }

    #[tokio::test]
    async fn expired_deadline_fails_before_dispatch() {
        let transport = Arc::new(LocalServiceTransport::new());
        let service_id = KernelServiceId::new("service.echo");
        transport
            .register_handler(service_id.clone(), Arc::new(EchoHandler))
            .unwrap();
        let mut envelope = traced_envelope(service_id);
        envelope.deadline = Some(Utc::now() - Duration::seconds(1));
        let bus = ServiceBus::new(transport);

        let err = bus.call(envelope).await.unwrap_err();

        assert_eq!(err, ServiceBusError::DeadlineExceeded);
    }

    #[tokio::test]
    async fn mock_remote_transport_uses_same_service_bus_facade() {
        let service_id = KernelServiceId::new("service.remote");
        let bus = ServiceBus::new(Arc::new(MockRemoteTransport));

        let reply = bus.call(traced_envelope(service_id)).await.unwrap();

        assert!(reply.success);
        assert_eq!(
            reply.transport_kind,
            ServiceTransportKind::Custom("mock_remote".into())
        );
        assert_eq!(reply.output, Some(serde_json::json!({"remote": true})));
    }
}
