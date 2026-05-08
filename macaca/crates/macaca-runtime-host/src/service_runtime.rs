//! Host-owned ServiceRuntime facade for Route C ServiceRuntime v1.
//!
//! `ServiceRuntime` is intentionally hosted in `macaca-runtime-host`, not in the
//! kernel.  The kernel owns stable primitives and service-call adapters; this
//! facade owns runtime orchestration: provider-neutral registration, lifecycle,
//! bus routing, admission decorators, audit events, and deterministic snapshots.
//! Concrete provider migration remains a later Route C phase.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use macaca_ipc::{LocalServiceTransport, ServiceBus};
use macaca_kernel::{InMemoryTraceEventBus, SystemServiceBusHandler};
use macaca_proto::{
    KernelServiceId, ServiceBusSource, ServiceCommand, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, ServiceReply, TraceContext,
};

use crate::service_decorator::{
    PolicyRuntimeDecorator, ServiceRuntimeCallContext, ServiceRuntimeDecorator,
    TraceRequiredRuntimeDecorator,
};
use crate::service_provider::{ServiceProviderFactory, ServiceProviderFactoryContext};
use crate::service_runtime_error::{ServiceRuntimeConfig, ServiceRuntimeError};
use crate::service_runtime_event::{
    ServiceRuntimeEvent, ServiceRuntimeEventSink, ServiceRuntimeServiceSnapshot,
    ServiceRuntimeSnapshot,
};

struct RuntimeServiceRecord {
    descriptor: ServiceDescriptor,
    service: Arc<dyn macaca_kernel::SystemService>,
    lifecycle_state: ServiceLifecycleState,
    health: ServiceHealth,
    failure_reason: Option<String>,
}

/// Facade that hosts provider-neutral services and dispatches calls by bus.
pub struct ServiceRuntime {
    services: RwLock<BTreeMap<KernelServiceId, RuntimeServiceRecord>>,
    transport: Arc<LocalServiceTransport>,
    bus: ServiceBus,
    decorators: Vec<Box<dyn ServiceRuntimeDecorator>>,
    event_sink: Option<Arc<dyn ServiceRuntimeEventSink>>,
}

impl ServiceRuntime {
    /// Build a runtime with trace and policy decorators installed first.
    pub fn new(config: ServiceRuntimeConfig) -> Self {
        let transport = Arc::new(LocalServiceTransport::new());
        let mut bus = ServiceBus::new(transport.clone());
        if let Some(bus_trace_sink) = config.bus_trace_sink {
            bus = bus.with_trace_sink(bus_trace_sink);
        }
        Self {
            services: RwLock::new(BTreeMap::new()),
            transport,
            bus,
            decorators: vec![
                Box::new(TraceRequiredRuntimeDecorator),
                Box::new(PolicyRuntimeDecorator::new(config.policy)),
            ],
            event_sink: config.event_sink,
        }
    }

    /// Append one custom decorator after built-in trace and policy checks.
    pub fn with_decorator(mut self, decorator: Box<dyn ServiceRuntimeDecorator>) -> Self {
        self.decorators.push(decorator);
        self
    }

    /// Register one provider-neutral service from a factory.
    pub async fn register_provider(
        &self,
        factory: &dyn ServiceProviderFactory,
        context: ServiceProviderFactoryContext,
    ) -> Result<KernelServiceId, ServiceRuntimeError> {
        let instance = factory.create(context).await?;
        let service_id = instance.descriptor.id.clone();
        tracing::info!(service_id = %service_id, "service runtime provider registration requested");

        {
            let services = self.services.read().map_err(lock_error)?;
            if services.contains_key(&service_id) {
                self.emit(
                    &service_id,
                    "service_runtime.register.failed",
                    ServiceLifecycleState::Failed {
                        reason: "duplicate service".into(),
                    },
                    None,
                    None,
                    serde_json::json!({"error": "duplicate service"}),
                )?;
                return Err(ServiceRuntimeError::DuplicateService(
                    service_id.to_string(),
                ));
            }
        }

        let handler = Arc::new(SystemServiceBusHandler::new(
            instance.service.clone(),
            InMemoryTraceEventBus::new(),
        ));
        self.transport
            .register_handler(service_id.clone(), handler)
            .map_err(ServiceRuntimeError::from)?;

        let record = RuntimeServiceRecord {
            descriptor: instance.descriptor,
            service: instance.service,
            lifecycle_state: ServiceLifecycleState::Registered,
            health: ServiceHealth::Unknown,
            failure_reason: None,
        };
        let mut services = self.services.write().map_err(lock_error)?;
        services.insert(service_id.clone(), record);
        drop(services);

        self.emit(
            &service_id,
            "service_runtime.register.completed",
            ServiceLifecycleState::Registered,
            Some(ServiceHealth::Unknown),
            None,
            serde_json::json!({"status": "registered"}),
        )?;
        Ok(service_id)
    }

    /// Start a registered service and record runtime-owned lifecycle state.
    pub async fn start(
        &self,
        service_id: &KernelServiceId,
        trace: TraceContext,
    ) -> Result<(), ServiceRuntimeError> {
        let service = self.transition(service_id, ServiceLifecycleState::Starting, Some(&trace))?;
        match service.start().await {
            Ok(()) => {
                self.set_state(
                    service_id,
                    ServiceLifecycleState::Running,
                    ServiceHealth::Healthy,
                    None,
                )?;
                self.emit(
                    service_id,
                    "service_runtime.start.completed",
                    ServiceLifecycleState::Running,
                    Some(ServiceHealth::Healthy),
                    Some(&trace),
                    serde_json::json!({"status": "running"}),
                )
            }
            Err(err) => self.fail(
                service_id,
                "service_runtime.start.failed",
                Some(&trace),
                err.to_string(),
            ),
        }
    }

    /// Dispatch a traced command through decorators and the service bus.
    pub async fn call(
        &self,
        service_id: &KernelServiceId,
        source: ServiceBusSource,
        command: ServiceCommand,
    ) -> Result<ServiceReply, ServiceRuntimeError> {
        let (descriptor, trace) = self.descriptor_and_trace(service_id, &command)?;
        let context = ServiceRuntimeCallContext {
            service_id,
            source: &source,
            command: &command,
            descriptor: &descriptor,
        };
        for decorator in &self.decorators {
            if let Err(err) = decorator.before_dispatch(&context).await {
                self.emit_rejection(service_id, &command, &err)?;
                return Err(err);
            }
        }

        self.set_lifecycle(service_id, ServiceLifecycleState::Calling)?;
        self.emit(
            service_id,
            "service_runtime.call.dispatched",
            ServiceLifecycleState::Calling,
            None,
            trace.as_ref(),
            serde_json::json!({"command": command.name.to_string()}),
        )?;
        let envelope = macaca_proto::ServiceEnvelope::new(source, service_id.clone(), command);
        match self.bus.call(envelope).await {
            Ok(reply) => {
                self.set_lifecycle(service_id, ServiceLifecycleState::Running)?;
                self.emit(
                    service_id,
                    "service_runtime.call.completed",
                    ServiceLifecycleState::Running,
                    Some(ServiceHealth::Healthy),
                    reply.trace.as_ref(),
                    serde_json::json!({"status": reply.status}),
                )?;
                Ok(reply)
            }
            Err(err) => self.fail_with_result(
                service_id,
                "service_runtime.call.failed",
                trace.as_ref(),
                err.to_string(),
            ),
        }
    }

    /// Stop a service gracefully.
    pub async fn stop(
        &self,
        service_id: &KernelServiceId,
        trace: TraceContext,
    ) -> Result<(), ServiceRuntimeError> {
        let service = self.transition(service_id, ServiceLifecycleState::Stopping, Some(&trace))?;
        match service.stop().await {
            Ok(()) => {
                self.set_lifecycle(service_id, ServiceLifecycleState::Stopped)?;
                self.emit(
                    service_id,
                    "service_runtime.stop.completed",
                    ServiceLifecycleState::Stopped,
                    None,
                    Some(&trace),
                    serde_json::json!({"status": "stopped"}),
                )
            }
            Err(err) => self.fail(
                service_id,
                "service_runtime.stop.failed",
                Some(&trace),
                err.to_string(),
            ),
        }
    }

    /// Clean up resources owned by a service adapter.
    pub async fn cleanup(
        &self,
        service_id: &KernelServiceId,
        trace: TraceContext,
    ) -> Result<(), ServiceRuntimeError> {
        let service =
            self.transition(service_id, ServiceLifecycleState::CleaningUp, Some(&trace))?;
        match service.cleanup().await {
            Ok(()) => {
                self.set_lifecycle(service_id, ServiceLifecycleState::CleanedUp)?;
                self.emit(
                    service_id,
                    "service_runtime.cleanup.completed",
                    ServiceLifecycleState::CleanedUp,
                    None,
                    Some(&trace),
                    serde_json::json!({"status": "cleaned_up"}),
                )
            }
            Err(err) => self.fail(
                service_id,
                "service_runtime.cleanup.failed",
                Some(&trace),
                err.to_string(),
            ),
        }
    }

    /// Return a deterministic runtime memento sorted by service id.
    pub fn snapshot(&self) -> Result<ServiceRuntimeSnapshot, ServiceRuntimeError> {
        tracing::info!("service runtime snapshot requested");
        let services = self.services.read().map_err(lock_error)?;
        Ok(ServiceRuntimeSnapshot {
            services: services
                .values()
                .map(|record| ServiceRuntimeServiceSnapshot {
                    descriptor: record.descriptor.clone(),
                    lifecycle_state: record.lifecycle_state.clone(),
                    health: record.health.clone(),
                    failure_reason: record.failure_reason.clone(),
                })
                .collect(),
        })
    }

    fn descriptor_and_trace(
        &self,
        service_id: &KernelServiceId,
        command: &ServiceCommand,
    ) -> Result<(ServiceDescriptor, Option<TraceContext>), ServiceRuntimeError> {
        let services = self.services.read().map_err(lock_error)?;
        let record = services
            .get(service_id)
            .ok_or_else(|| ServiceRuntimeError::UnknownService(service_id.to_string()))?;
        Ok((record.descriptor.clone(), command.trace.clone()))
    }

    fn transition(
        &self,
        service_id: &KernelServiceId,
        next: ServiceLifecycleState,
        trace: Option<&TraceContext>,
    ) -> Result<Arc<dyn macaca_kernel::SystemService>, ServiceRuntimeError> {
        let mut services = self.services.write().map_err(lock_error)?;
        let record = services
            .get_mut(service_id)
            .ok_or_else(|| ServiceRuntimeError::UnknownService(service_id.to_string()))?;
        record.lifecycle_state = next.clone();
        let service = record.service.clone();
        drop(services);
        self.emit(
            service_id,
            "service_runtime.lifecycle.transition",
            next,
            None,
            trace,
            serde_json::json!({}),
        )?;
        Ok(service)
    }

    fn set_lifecycle(
        &self,
        service_id: &KernelServiceId,
        state: ServiceLifecycleState,
    ) -> Result<(), ServiceRuntimeError> {
        let mut services = self.services.write().map_err(lock_error)?;
        let record = services
            .get_mut(service_id)
            .ok_or_else(|| ServiceRuntimeError::UnknownService(service_id.to_string()))?;
        record.lifecycle_state = state;
        Ok(())
    }

    fn set_state(
        &self,
        service_id: &KernelServiceId,
        state: ServiceLifecycleState,
        health: ServiceHealth,
        failure_reason: Option<String>,
    ) -> Result<(), ServiceRuntimeError> {
        let mut services = self.services.write().map_err(lock_error)?;
        let record = services
            .get_mut(service_id)
            .ok_or_else(|| ServiceRuntimeError::UnknownService(service_id.to_string()))?;
        record.lifecycle_state = state;
        record.health = health;
        record.failure_reason = failure_reason;
        Ok(())
    }

    fn emit_rejection(
        &self,
        service_id: &KernelServiceId,
        command: &ServiceCommand,
        err: &ServiceRuntimeError,
    ) -> Result<(), ServiceRuntimeError> {
        self.emit(
            service_id,
            "service_runtime.call.rejected",
            ServiceLifecycleState::Registered,
            None,
            command.trace.as_ref(),
            serde_json::json!({"command": command.name.to_string(), "error": err.to_string()}),
        )
    }

    fn fail(
        &self,
        service_id: &KernelServiceId,
        operation: &str,
        trace: Option<&TraceContext>,
        reason: String,
    ) -> Result<(), ServiceRuntimeError> {
        self.set_state(
            service_id,
            ServiceLifecycleState::Failed {
                reason: reason.clone(),
            },
            ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            Some(reason.clone()),
        )?;
        self.emit(
            service_id,
            operation,
            ServiceLifecycleState::Failed {
                reason: reason.clone(),
            },
            Some(ServiceHealth::Unavailable {
                reason: reason.clone(),
            }),
            trace,
            serde_json::json!({"error": reason}),
        )?;
        Err(ServiceRuntimeError::Service(reason))
    }

    fn fail_with_result<T>(
        &self,
        service_id: &KernelServiceId,
        operation: &str,
        trace: Option<&TraceContext>,
        reason: String,
    ) -> Result<T, ServiceRuntimeError> {
        self.fail(service_id, operation, trace, reason.clone())?;
        Err(ServiceRuntimeError::Service(reason))
    }

    fn emit(
        &self,
        service_id: &KernelServiceId,
        operation: impl Into<String>,
        lifecycle_state: ServiceLifecycleState,
        health: Option<ServiceHealth>,
        trace: Option<&TraceContext>,
        payload: serde_json::Value,
    ) -> Result<(), ServiceRuntimeError> {
        let operation = operation.into();
        tracing::info!(
            service_id = %service_id,
            operation = %operation,
            lifecycle_state = ?lifecycle_state,
            trace_id = trace.map(|value| value.trace_id.as_str()),
            "service runtime event emitted"
        );
        if let Some(sink) = &self.event_sink {
            sink.emit(ServiceRuntimeEvent {
                service_id: service_id.clone(),
                operation,
                lifecycle_state,
                health,
                trace_id: trace.map(|value| value.trace_id.clone()),
                emitted_at: Utc::now(),
                payload,
            })?;
        }
        Ok(())
    }
}

fn lock_error<T>(_: T) -> ServiceRuntimeError {
    ServiceRuntimeError::State("service runtime lock poisoned".into())
}
