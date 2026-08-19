//! Service and Abstract Factory contracts for secret-reference providers.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    SecretsReferenceProviderCapability, SecretsReferenceProviderSnapshot, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceLifecycleState, ServiceResult, ServiceScope, ServiceType, TraceSchemaRef,
    FOUNDATION_SECRETS_REFERENCE_COMMANDS, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
};

/// Provider-neutral service boundary; implementations must not expose raw secret values.
#[async_trait]
pub trait SecretsReferenceService: Send + Sync {
    /// Return bounded descriptor metadata for registry and discovery.
    fn descriptor(&self) -> ServiceDescriptor;
    /// Execute one traced reference, lease, rotation, or audit command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;
    /// Return health without endpoints, credentials, or provider payloads.
    fn health(&self) -> ServiceHealth;
    /// Return replay-safe state hashes and lease counts.
    fn snapshot(&self) -> SecretsReferenceProviderSnapshot;
    /// Report provider capabilities and raw-value prohibition.
    fn provider_capabilities(&self) -> SecretsReferenceProviderCapability;
    /// Stop provider state and revoke bounded leases.
    async fn shutdown(&self) -> ServiceResult<()>;
}

/// Composition-root factory for replaceable secret adapters.
pub trait SecretsReferenceProviderFactory: Send + Sync {
    /// Return a bounded provider class label.
    fn provider_class(&self) -> &str;
    /// Construct the provider-owned service strategy.
    fn create(&self) -> Arc<dyn SecretsReferenceService>;
}

/// Null Object used when no secret-reference provider is installed.
#[derive(Debug, Clone)]
pub struct UnavailableSecretsReferenceProvider {
    reason: String,
}

impl UnavailableSecretsReferenceProvider {
    /// Construct a fail-closed unavailable strategy with a safe reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableSecretsReferenceProvider {
    fn default() -> Self {
        Self::new("secrets-reference provider is not installed")
    }
}

#[async_trait]
impl SecretsReferenceService for UnavailableSecretsReferenceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_SECRETS_REFERENCE_SERVICE_ID),
            ServiceType::new("foundation.secrets_reference"),
            TraceSchemaRef::new("macaca.trace.foundation.secrets_reference.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = FOUNDATION_SECRETS_REFERENCE_COMMANDS
            .iter()
            .map(|name| {
                ServiceCapability::new(CapabilityId::new(*name), "secret reference command")
            })
            .collect();
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        tracing::warn!(service_id = FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
            command = %command.name, trace_id = %trace.trace_id,
            "secrets reference command rejected: provider unavailable");
        Ok(ServiceCallResult {
            output: serde_json::json!({"status":"unavailable","reason":self.reason}),
            trace,
            status: "unavailable".into(),
            metadata: [
                (
                    "replay.secrets_reference_command".into(),
                    command.name.to_string(),
                ),
                (
                    "service.audit.stage".into(),
                    "secrets_reference_pack_unavailable".into(),
                ),
                (
                    "secrets_reference.redaction".into(),
                    "raw_values_and_locators_redacted".into(),
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
    fn snapshot(&self) -> SecretsReferenceProviderSnapshot {
        SecretsReferenceProviderSnapshot {
            descriptor_hash: "foundation-secrets-reference-unavailable-v1".into(),
            provider_class: "unavailable".into(),
            reference_state_hashes: Default::default(),
            lease_state_hashes: Default::default(),
            audit_tail_hash: "unavailable".into(),
        }
    }
    fn provider_capabilities(&self) -> SecretsReferenceProviderCapability {
        SecretsReferenceProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: Default::default(),
            supported_version_states: Default::default(),
            supports_leases: false,
            supports_rotation: false,
            supports_provider_injection: false,
            raw_value_app_results_forbidden: true,
            max_lease_ttl_seconds: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }
    }
    async fn shutdown(&self) -> ServiceResult<()> {
        Ok(())
    }
}
