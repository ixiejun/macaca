//! System service contract for replaceable OS capabilities.
//!
//! A `SystemService` describes lifecycle, health, and command execution for
//! one replaceable capability surface.  The trait does not define concrete LLM,
//! driver, gateway, memory, skill, or task behavior; those remain in service
//! crates and adapter implementations.

use async_trait::async_trait;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceLifecycleState, ServiceResult, TraceContext,
};

/// Provider-neutral system service boundary.
#[async_trait]
pub trait SystemService: Send + Sync {
    /// Return a descriptor for discovery, policy, and diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;

    /// Start the service after registration and authorization.
    async fn start(&self) -> ServiceResult<()>;

    /// Execute one command.  Callers should normally use `ServiceCallExecutor`
    /// so trace, logging, and middleware rules run before this method.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;

    /// Stop the service gracefully.
    async fn stop(&self) -> ServiceResult<()>;

    /// Clean up resources owned by this service adapter.
    async fn cleanup(&self) -> ServiceResult<()>;

    /// Report current health without performing provider-specific work.
    async fn health(&self) -> ServiceResult<ServiceHealth>;
}

/// Provider-neutral mock service used by contract tests.
///
/// The mock service proves lifecycle and call semantics without depending on
/// a real provider.  Production services can preserve the same contract while
/// delegating execution to LLM, task, driver, skill, gateway, or memory code.
pub struct MockSystemService {
    descriptor: ServiceDescriptor,
    fail_calls: bool,
}

impl MockSystemService {
    /// Create a healthy mock service.
    pub fn new(descriptor: ServiceDescriptor) -> Self {
        Self {
            descriptor,
            fail_calls: false,
        }
    }

    /// Create a mock service that fails every call with a structured error.
    pub fn failing(descriptor: ServiceDescriptor) -> Self {
        Self {
            descriptor,
            fail_calls: true,
        }
    }

    fn service_id(&self) -> KernelServiceId {
        self.descriptor.id.clone()
    }
}

#[async_trait]
impl SystemService for MockSystemService {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.service_id(), "system service start requested");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        tracing::info!(
            service_id = %self.service_id(),
            command = %command.name,
            "system service mock call dispatched"
        );
        if self.fail_calls {
            return Err(ServiceError::AdapterFailure(
                "mock service configured to fail".into(),
            ));
        }
        let trace: TraceContext = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        Ok(ServiceCallResult {
            output: command.payload,
            trace,
            status: "ok".into(),
            metadata: Default::default(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.service_id(), "system service stop requested");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.service_id(), "system service cleanup requested");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(match self.descriptor.lifecycle_state {
            ServiceLifecycleState::Failed { ref reason } => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            _ => ServiceHealth::Healthy,
        })
    }
}
