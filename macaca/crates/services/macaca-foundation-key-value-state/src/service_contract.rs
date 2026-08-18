//! Key-value state service contract and fail-closed unavailable Strategy.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    KeyValueStateProviderCapability, KeyValueStateProviderSnapshot, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceLifecycleState, ServiceResult, ServiceScope, ServiceType, TraceSchemaRef,
    FOUNDATION_KEY_VALUE_STATE_COMMANDS, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};
use std::sync::Arc;

/// Provider-neutral Command boundary for all key-value state Strategies.
#[async_trait]
pub trait KeyValueStateService: Send + Sync {
    /// Return registry/discovery metadata without exposing provider implementation details.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Process one trace-required, provider-neutral key-value state command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return bounded health facts for service lifecycle diagnostics.
    fn health(&self) -> ServiceHealth;
    /// Return replay-safe lifecycle state without raw namespaces, keys, or values.
    fn snapshot(&self) -> KeyValueStateProviderSnapshot;
    /// Report capabilities without exposing provider-native topology or clients.
    fn provider_capabilities(&self) -> KeyValueStateProviderCapability;
    /// Cancel one bounded watch by its trace identity; the default is a no-op for providers
    /// that do not expose streaming state.
    async fn cancel_watch(&self, _trace_id: &str) -> ServiceResult<()> {
        Ok(())
    }
    /// Stop the provider and release bounded watches, leases, and caches.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Abstract Factory boundary for optional provider adapters.
///
/// A host composition can register embedded, remote, consensus, plugin, or
/// test Strategies through this factory without exposing a provider client,
/// protocol request, endpoint, or topology detail to SDK and application code.
pub trait KeyValueStateProviderFactory: Send + Sync {
    /// Return a bounded provider-class identifier for diagnostics and selection.
    fn provider_class(&self) -> &str;
    /// Construct the provider-owned Strategy behind the generic service contract.
    fn create(&self) -> Arc<dyn KeyValueStateService>;
}

/// Null Object used when a host composition has no key-value state provider.
#[derive(Debug, Clone)]
pub struct UnavailableKeyValueStateProvider {
    reason: String,
}

impl UnavailableKeyValueStateProvider {
    /// Construct a fail-closed provider with a bounded safe diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableKeyValueStateProvider {
    fn default() -> Self {
        Self::new("key-value state provider is not installed")
    }
}

#[async_trait]
impl KeyValueStateService for UnavailableKeyValueStateProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID),
            ServiceType::new("foundation.key_value_state"),
            TraceSchemaRef::new("macaca.trace.foundation.key_value_state.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = FOUNDATION_KEY_VALUE_STATE_COMMANDS
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "key-value state command"))
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(
            service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "key-value state command rejected: provider unavailable"
        );
        Ok(ServiceCallResult {
            output: serde_json::json!({"status":"unavailable","reason":self.reason}),
            trace,
            status: "unavailable".into(),
            metadata: [
                (
                    "key_value_state.audit_event".into(),
                    "key_value_state_pack_unavailable".into(),
                ),
                (
                    "service.audit.stage".into(),
                    "key_value_state_pack_unavailable".into(),
                ),
            ]
            .into_iter()
            .collect(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        }
    }

    fn snapshot(&self) -> KeyValueStateProviderSnapshot {
        KeyValueStateProviderSnapshot {
            descriptor_hash: "foundation-key-value-state-unavailable-v1".into(),
            provider_class: "unavailable".into(),
            namespace_hashes: Default::default(),
            active_watch_count: 0,
        }
    }

    fn provider_capabilities(&self) -> KeyValueStateProviderCapability {
        KeyValueStateProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: Default::default(),
            supports_ttl: false,
            supports_watch: false,
            supports_snapshot: false,
            supports_compaction: false,
            max_value_bytes: 0,
            max_batch_entries: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
