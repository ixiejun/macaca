//! Bridge from the IPC service bus to Phase 02 `SystemService` execution.
//!
//! The bridge implements the Adapter pattern.  `macaca-ipc` owns generic local
//! service routing but deliberately does not depend on the kernel crate.  This
//! adapter lives in `macaca-kernel`, depends on both sides, and converts a bus
//! `ServiceEnvelope` into the existing traced `ServiceCallExecutor` path.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_ipc::LocalServiceHandler;
use macaca_proto::{
    ServiceBusError, ServiceBusResult, ServiceEnvelope, ServiceReply, ServiceTransportKind,
};

use crate::facade::{InMemoryTraceEventBus, KernelTraceEvent, TraceEventBus};
use crate::service_call::ServiceCallExecutor;
use crate::system_service::SystemService;

/// Local bus handler that invokes one Phase 02 `SystemService`.
///
/// The handler is intentionally small: it adapts envelope trace into the
/// command when necessary, calls `ServiceCallExecutor` so trace middleware and
/// service-level audit events still run, and converts service failures into
/// structured service bus errors.
pub struct SystemServiceBusHandler<B>
where
    B: TraceEventBus,
{
    service: Arc<dyn SystemService>,
    executor: ServiceCallExecutor<B>,
}

impl<B> SystemServiceBusHandler<B>
where
    B: TraceEventBus,
{
    /// Create a handler for one service and one trace bus.
    pub fn new(service: Arc<dyn SystemService>, trace_bus: B) -> Self {
        Self {
            service,
            executor: ServiceCallExecutor::new(trace_bus),
        }
    }

    /// Expose the underlying executor for tests and diagnostics.
    pub fn executor(&self) -> &ServiceCallExecutor<B> {
        &self.executor
    }
}

impl SystemServiceBusHandler<InMemoryTraceEventBus> {
    /// Return service-level trace events when using the in-memory trace bus.
    pub fn trace_events(&self) -> macaca_proto::KernelPrimitiveResult<Vec<KernelTraceEvent>> {
        self.executor.trace_bus().events()
    }
}

#[async_trait]
impl<B> LocalServiceHandler for SystemServiceBusHandler<B>
where
    B: TraceEventBus + Send + Sync,
{
    async fn handle(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply> {
        tracing::info!(
            envelope_id = %envelope.id,
            target_service = %envelope.target_service,
            command = %envelope.command.name,
            "system service bus handler adapting envelope to service command"
        );

        let mut command = envelope.command.clone();
        if command.trace.is_none() {
            command.trace = envelope.trace_context();
        }

        let result = self.executor.execute(self.service.as_ref(), command).await;
        match result {
            Ok(result) => {
                let mut reply = ServiceReply::success(
                    &envelope,
                    ServiceTransportKind::Local,
                    result.output,
                    result.status,
                );
                reply.trace = Some(result.trace);
                reply.metadata = result.metadata;
                Ok(reply)
            }
            Err(err) => {
                tracing::warn!(
                    envelope_id = %envelope.id,
                    target_service = %envelope.target_service,
                    command = %envelope.command.name,
                    error = %err,
                    "system service bus handler received service error"
                );
                Err(match err {
                    macaca_proto::ServiceError::MissingTraceContext => {
                        ServiceBusError::MissingTraceContext
                    }
                    macaca_proto::ServiceError::InvalidArgument(reason) => {
                        ServiceBusError::InvalidArgument(reason)
                    }
                    other => ServiceBusError::ServiceError(other.to_string()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use macaca_ipc::{LocalServiceTransport, ServiceBus};
    use macaca_proto::{
        KernelServiceId, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceDescriptor,
        ServiceEnvelope, ServiceTransportKind, ServiceType, TraceContext, TraceSchemaRef,
    };

    use crate::system_service::MockSystemService;

    use super::*;

    #[tokio::test]
    async fn service_bus_bridge_invokes_mock_system_service() {
        let service_id = KernelServiceId::new("service.kernel.mock");
        let descriptor = ServiceDescriptor::new(
            service_id.clone(),
            ServiceType::new("mock"),
            TraceSchemaRef::new("trace.mock.v1"),
        );
        let handler = Arc::new(SystemServiceBusHandler::new(
            Arc::new(MockSystemService::new(descriptor)),
            InMemoryTraceEventBus::new(),
        ));
        let transport = Arc::new(LocalServiceTransport::new());
        transport
            .register_handler(service_id.clone(), handler.clone())
            .unwrap();
        let bus = ServiceBus::new(transport);
        let trace = TraceContext::new("trace-kernel-bridge");
        let envelope = ServiceEnvelope::new(
            ServiceBusSource::new("kernel.test"),
            service_id,
            ServiceCommand::with_trace(
                ServiceCommandName::new("call"),
                serde_json::json!({"bridge": true}),
                trace.clone(),
            ),
        );

        let reply = bus.call(envelope).await.unwrap();

        assert!(reply.success);
        assert_eq!(reply.transport_kind, ServiceTransportKind::Local);
        assert_eq!(
            reply.trace.map(|trace| trace.trace_id),
            Some(trace.trace_id)
        );
        let service_events = handler.trace_events().unwrap();
        assert!(service_events
            .iter()
            .any(|event| event.event_type == "system_service.call.accepted"));
        assert!(service_events
            .iter()
            .any(|event| event.event_type == "system_service.call.completed"));
    }
}
