//! Provider-neutral time service interface and fail-closed Null Object.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceLifecycleState,
    ServiceResult, ServiceScope, ServiceType, TimeClockHealth, TimeProviderSnapshot,
    TraceSchemaRef, FOUNDATION_TIME_COMMANDS, FOUNDATION_TIME_SERVICE_ID,
};

/// Time capability boundary implemented by host, remote, plugin, and replay providers.
#[async_trait]
pub trait TimeService: Send + Sync {
    /// Describe declared commands, health, and provider-neutral diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Execute one trace-required time command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return bounded provider health without raw host state.
    fn health(&self) -> ServiceHealth;
    /// Return a replay-safe Memento without timer ids, payloads, or host handles.
    fn snapshot(&self) -> TimeProviderSnapshot;
    /// Release active timer state during service shutdown.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Null Object for hosts where no time provider is installed.
#[derive(Debug, Clone)]
pub struct UnavailableTimeProvider {
    reason: String,
}

impl UnavailableTimeProvider {
    /// Create a structured unavailable provider with a bounded diagnostic.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableTimeProvider {
    fn default() -> Self {
        Self::new("time provider is not installed")
    }
}

#[async_trait]
impl TimeService for UnavailableTimeProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_TIME_SERVICE_ID),
            ServiceType::new("foundation.time"),
            TraceSchemaRef::new("macaca.trace.foundation.time.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::Always;
        descriptor.capabilities = FOUNDATION_TIME_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "time command"))
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(service_id = FOUNDATION_TIME_SERVICE_ID, command = %command.name,
            trace_id = %trace.trace_id, "time service unavailable");
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
    fn snapshot(&self) -> TimeProviderSnapshot {
        TimeProviderSnapshot {
            descriptor_hash: "foundation-time-unavailable-v1".into(),
            provider_class: "unavailable".into(),
            health: TimeClockHealth {
                provider_class: "unavailable".into(),
                wall_clock_available: false,
                monotonic_available: false,
                timezone_data_version: None,
                locale_data_available: false,
                max_timer_duration_ms: 0,
                unavailable_reason: Some(self.reason.clone()),
            },
            timer_state_hashes: Default::default(),
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
