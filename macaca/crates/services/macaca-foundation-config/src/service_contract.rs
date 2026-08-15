//! Service contract and unavailable Null Object for foundation configuration.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, ConfigProviderSnapshot, KernelServiceId, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceLifecycleState, ServiceResult, ServiceScope, ServiceType, TraceSchemaRef,
    FOUNDATION_CONFIG_COMMANDS, FOUNDATION_CONFIG_SERVICE_ID,
};

/// Provider-neutral boundary for layered configuration providers.
#[async_trait]
pub trait ConfigService: Send + Sync {
    /// Return descriptor metadata used for registration and capability discovery.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Execute one trace-required, canonical configuration command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return only bounded health facts, never source locations or values.
    fn health(&self) -> ServiceHealth;
    /// Return a replay-safe Memento containing hashes and redaction counts only.
    fn snapshot(&self) -> ConfigProviderSnapshot;
    /// Release watches and provider-owned caches during runtime shutdown.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Fail-closed provider used when no configuration source is composed.
#[derive(Debug, Clone)]
pub struct UnavailableConfigProvider {
    reason: String,
}

impl UnavailableConfigProvider {
    /// Create an unavailable provider with a bounded diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableConfigProvider {
    fn default() -> Self {
        Self::new("foundation config provider is not installed")
    }
}

#[async_trait]
impl ConfigService for UnavailableConfigProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_CONFIG_SERVICE_ID),
            ServiceType::new("foundation.config"),
            TraceSchemaRef::new("macaca.trace.foundation.config.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = FOUNDATION_CONFIG_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "config command"))
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(service_id = FOUNDATION_CONFIG_SERVICE_ID, command = %command.name,
            trace_id = %trace.trace_id, "foundation config command rejected: provider unavailable");
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
    fn snapshot(&self) -> ConfigProviderSnapshot {
        ConfigProviderSnapshot {
            descriptor_hash: "foundation-config-unavailable-v1".into(),
            provider_class: "unavailable".into(),
            source_hashes: Default::default(),
            schema_hashes: Default::default(),
            redaction_summary: macaca_proto::ConfigRedactionSummary {
                redacted_value_count: 0,
                redacted_source_count: 0,
                contains_secret_references: false,
            },
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
