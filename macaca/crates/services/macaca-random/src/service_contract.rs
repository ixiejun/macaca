//! Command-oriented random service contract and unavailable Null Object.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceLifecycleState,
    ServiceResult, ServiceScope, ServiceType, TraceSchemaRef, FOUNDATION_RANDOM_COMMANDS,
    FOUNDATION_RANDOM_SERVICE_ID,
};

/// Provider-neutral random service boundary.
#[async_trait]
pub trait RandomService: Send + Sync {
    /// Return descriptor metadata used by service registration and diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Dispatch one traced random command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return a bounded health projection without generated values.
    fn health(&self) -> ServiceHealth;
    /// Stop provider-owned state and release resources.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Fail-closed Null Object for absent entropy providers.
#[derive(Debug, Clone)]
pub struct UnavailableRandomProvider {
    reason: String,
}

impl UnavailableRandomProvider {
    /// Create an unavailable provider with a safe, bounded reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableRandomProvider {
    fn default() -> Self {
        Self::new("random provider is not installed")
    }
}

#[async_trait]
impl RandomService for UnavailableRandomProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_RANDOM_SERVICE_ID),
            ServiceType::new("foundation.random"),
            TraceSchemaRef::new("macaca.trace.foundation.random.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = FOUNDATION_RANDOM_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "random command"))
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(service_id = FOUNDATION_RANDOM_SERVICE_ID, command = %command.name,
            trace_id = %trace.trace_id, "random command rejected: provider unavailable");
        Ok(ServiceCallResult {
            output: serde_json::json!({"status":"unavailable","reason":self.reason}),
            trace,
            status: "unavailable".into(),
            metadata: Default::default(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
