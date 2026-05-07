//! Local typed service transport for the service bus.
//!
//! This module keeps Route C Phase 03 local-first and typed-first.  Local
//! dispatch uses Rust trait objects and `ServiceEnvelope` values directly
//! instead of forcing every hot-path service call through JSON or a wire
//! protocol.  Real child-process, MCP, HTTP, and signed remote A2A transports
//! can implement the same `ServiceTransport` trait later.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use macaca_proto::{
    KernelServiceId, ServiceBusError, ServiceBusResult, ServiceEnvelope, ServiceReply,
    ServiceTransportKind,
};

use crate::transport::ServiceTransport;

/// Handler contract for one local service endpoint.
///
/// The handler is intentionally generic and lives in `macaca-ipc` so this
/// crate does not need to depend on `macaca-kernel`.  Kernel adapters can wrap
/// Phase 02 `SystemService` implementations behind this trait from the kernel
/// side, preserving an acyclic dependency direction.
#[async_trait]
pub trait LocalServiceHandler: Send + Sync {
    /// Handle one already-routed envelope.
    async fn handle(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply>;
}

/// In-process service transport keyed by `KernelServiceId`.
///
/// Routing is identity-based and provider-neutral.  The transport does not
/// inspect application names, driver names, gateway names, workflow names, or
/// model names; those belong to service descriptors and policy data.
#[derive(Default)]
pub struct LocalServiceTransport {
    handlers: RwLock<BTreeMap<KernelServiceId, Arc<dyn LocalServiceHandler>>>,
}

impl LocalServiceTransport {
    /// Create an empty local service transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace the handler for a service id.
    pub fn register_handler(
        &self,
        service_id: KernelServiceId,
        handler: Arc<dyn LocalServiceHandler>,
    ) -> ServiceBusResult<()> {
        tracing::info!(
            service_id = %service_id,
            "local service transport handler registered"
        );
        let mut handlers = self.handlers.write().map_err(|_| {
            ServiceBusError::DispatchFailed("local service handler registry poisoned".into())
        })?;
        handlers.insert(service_id, handler);
        Ok(())
    }

    fn get_handler(
        &self,
        service_id: &KernelServiceId,
    ) -> ServiceBusResult<Option<Arc<dyn LocalServiceHandler>>> {
        let handlers = self.handlers.read().map_err(|_| {
            ServiceBusError::DispatchFailed("local service handler registry poisoned".into())
        })?;
        Ok(handlers.get(service_id).cloned())
    }
}

#[async_trait]
impl ServiceTransport for LocalServiceTransport {
    fn kind(&self) -> ServiceTransportKind {
        ServiceTransportKind::Local
    }

    async fn call(&self, envelope: ServiceEnvelope) -> ServiceBusResult<ServiceReply> {
        let handler = match self.get_handler(&envelope.target_service)? {
            Some(handler) => handler,
            None => {
                tracing::warn!(
                    envelope_id = %envelope.id,
                    target_service = %envelope.target_service,
                    "local service transport has no route for target service"
                );
                return Err(ServiceBusError::NoRoute(
                    envelope.target_service.to_string(),
                ));
            }
        };

        tracing::info!(
            envelope_id = %envelope.id,
            target_service = %envelope.target_service,
            "local service transport dispatching envelope"
        );
        handler.handle(envelope).await
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceEnvelope, TraceContext,
    };

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

    #[tokio::test]
    async fn local_transport_dispatches_to_registered_handler() {
        let transport = LocalServiceTransport::new();
        let service_id = KernelServiceId::new("service.echo");
        transport
            .register_handler(service_id.clone(), Arc::new(EchoHandler))
            .unwrap();
        let envelope = ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            service_id,
            ServiceCommand::with_trace(
                ServiceCommandName::new("echo"),
                serde_json::json!({"hello": "typed"}),
                TraceContext::new("trace-local-typed"),
            ),
        );

        let reply = transport.call(envelope).await.unwrap();

        assert!(reply.success);
        assert_eq!(reply.output, Some(serde_json::json!({"hello": "typed"})));
    }

    #[tokio::test]
    async fn local_transport_returns_structured_no_route() {
        let transport = LocalServiceTransport::new();
        let envelope = ServiceEnvelope::new(
            ServiceBusSource::new("test"),
            KernelServiceId::new("service.missing"),
            ServiceCommand::with_trace(
                ServiceCommandName::new("call"),
                serde_json::json!({}),
                TraceContext::new("trace-no-route"),
            ),
        );

        let err = transport.call(envelope).await.unwrap_err();

        assert_eq!(err, ServiceBusError::NoRoute("service.missing".into()));
    }
}
