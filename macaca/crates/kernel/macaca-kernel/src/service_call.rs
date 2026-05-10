//! Service call execution with trace, audit, and logging boundaries.
//!
//! The executor implements the Command + Chain of Responsibility pattern for
//! service calls.  Middleware validates cross-cutting requirements before the
//! command reaches a concrete service adapter.

use async_trait::async_trait;
use macaca_proto::{ServiceCommand, ServiceError, ServiceResult};

use crate::facade::{KernelTraceEvent, TraceEventBus};
use crate::system_service::SystemService;

/// Context passed to service call middleware.
///
/// The context intentionally carries a borrowed command and a service
/// descriptor.  Middleware can validate trace/policy/logging requirements
/// without gaining access to provider internals.
pub struct ServiceCallContext<'a> {
    pub service: &'a dyn SystemService,
    pub command: &'a ServiceCommand,
}

/// Middleware contract for service call validation and audit hooks.
#[async_trait]
pub trait ServiceCallMiddleware: Send + Sync {
    /// Validate or observe a call before service dispatch.
    async fn before_call(&self, context: &ServiceCallContext<'_>) -> ServiceResult<()>;
}

/// Middleware that enforces the Route C "no trace, no call" rule.
pub struct TraceRequiredMiddleware;

#[async_trait]
impl ServiceCallMiddleware for TraceRequiredMiddleware {
    async fn before_call(&self, context: &ServiceCallContext<'_>) -> ServiceResult<()> {
        if context.command.trace.is_none() {
            let descriptor = context.service.descriptor();
            tracing::warn!(
                service_id = %descriptor.id,
                command = %context.command.name,
                "system service call rejected before dispatch: missing trace context"
            );
            return Err(ServiceError::MissingTraceContext);
        }
        Ok(())
    }
}

/// Generic executor for traced system service calls.
///
/// The executor owns middleware sequencing but not service implementation.  It
/// logs execution boundaries and emits presentation-neutral trace events
/// through the Phase 01 `TraceEventBus` observer contract.
pub struct ServiceCallExecutor<B> {
    trace_bus: B,
    middleware: Vec<Box<dyn ServiceCallMiddleware>>,
}

impl<B> ServiceCallExecutor<B>
where
    B: TraceEventBus,
{
    /// Create an executor with trace-required middleware installed first.
    pub fn new(trace_bus: B) -> Self {
        Self {
            trace_bus,
            middleware: vec![Box::new(TraceRequiredMiddleware)],
        }
    }

    /// Append middleware after trace validation.
    pub fn with_middleware(mut self, middleware: Box<dyn ServiceCallMiddleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Execute one service command through middleware and trace emission.
    pub async fn execute(
        &self,
        service: &dyn SystemService,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let descriptor = service.descriptor();
        tracing::info!(
            service_id = %descriptor.id,
            command = %command.name,
            "system service call accepted by executor"
        );

        let context = ServiceCallContext {
            service,
            command: &command,
        };
        for middleware in &self.middleware {
            if let Err(err) = middleware.before_call(&context).await {
                self.emit_rejection_trace(&descriptor, &command, &err)?;
                return Err(err);
            }
        }

        if let Some(trace) = command.trace.clone() {
            self.trace_bus.emit_trace(KernelTraceEvent {
                context: trace,
                event_type: "system_service.call.accepted".into(),
                payload: serde_json::json!({
                    "service_id": descriptor.id.to_string(),
                    "command": command.name.to_string(),
                    "lifecycle_state": format!("{:?}", descriptor.lifecycle_state),
                }),
            })?;
        }

        let result = service.call(command.clone()).await;
        match result {
            Ok(result) => {
                tracing::info!(
                    service_id = %descriptor.id,
                    command = %command.name,
                    "system service call completed"
                );
                self.trace_bus.emit_trace(KernelTraceEvent {
                    context: result.trace.clone(),
                    event_type: "system_service.call.completed".into(),
                    payload: serde_json::json!({
                        "service_id": descriptor.id.to_string(),
                        "command": command.name.to_string(),
                        "status": result.status,
                        "lifecycle_state": format!("{:?}", descriptor.lifecycle_state),
                    }),
                })?;
                Ok(result)
            }
            Err(err) => {
                tracing::warn!(
                    service_id = %descriptor.id,
                    command = %command.name,
                    error = %err,
                    "system service call failed"
                );
                if let Some(trace) = command.trace.clone() {
                    self.trace_bus.emit_trace(KernelTraceEvent {
                        context: trace,
                        event_type: "system_service.call.failed".into(),
                        payload: serde_json::json!({
                            "service_id": descriptor.id.to_string(),
                            "command": command.name.to_string(),
                            "error": err.to_string(),
                        }),
                    })?;
                }
                Err(err)
            }
        }
    }

    /// Return a snapshot of trace events if the backing bus supports it.
    pub fn trace_bus(&self) -> &B {
        &self.trace_bus
    }

    fn emit_rejection_trace(
        &self,
        descriptor: &macaca_proto::ServiceDescriptor,
        command: &ServiceCommand,
        err: &ServiceError,
    ) -> ServiceResult<()> {
        tracing::warn!(
            service_id = %descriptor.id,
            command = %command.name,
            error = %err,
            "system service call rejected"
        );
        if let Some(trace) = command.trace.clone() {
            self.trace_bus.emit_trace(KernelTraceEvent {
                context: trace,
                event_type: "system_service.call.rejected".into(),
                payload: serde_json::json!({
                    "service_id": descriptor.id.to_string(),
                    "command": command.name.to_string(),
                    "error": err.to_string(),
                }),
            })?;
        }
        Ok(())
    }
}
