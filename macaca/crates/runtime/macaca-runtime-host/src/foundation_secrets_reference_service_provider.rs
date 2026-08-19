//! Runtime-host Bridge for provider-neutral secret-reference services.
//!
//! This is the sole composition point that adapts a reference provider to the
//! generic SystemService boundary. It returns handles and metadata only; raw
//! secret values remain inside provider-owned injection paths.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_foundation_secrets_reference::{
    SecretsReferenceService, UnavailableSecretsReferenceProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    SecretsReferenceProviderCapability, SecretsReferenceProviderSnapshot, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceResult,
};

/// Runtime composition adapter for replaceable secret-reference providers.
pub struct FoundationSecretsReferenceSystemServiceProvider {
    provider: Arc<dyn SecretsReferenceService>,
}

impl FoundationSecretsReferenceSystemServiceProvider {
    /// Inject a host-selected provider Strategy.
    pub fn new(provider: Arc<dyn SecretsReferenceService>) -> Self {
        Self { provider }
    }
    /// Build a fail-closed unavailable composition.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableSecretsReferenceProvider::default()))
    }
    /// Return replay-safe provider state.
    pub fn snapshot(&self) -> SecretsReferenceProviderSnapshot {
        self.provider.snapshot()
    }
    /// Return provider-neutral capability facts.
    pub fn provider_capabilities(&self) -> SecretsReferenceProviderCapability {
        self.provider.provider_capabilities()
    }
}

#[async_trait]
impl SystemService for FoundationSecretsReferenceSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }
    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = "service.foundation.secrets.reference",
            "secrets reference service started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.provider.call(command).await
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        tracing::info!(
            service_id = "service.foundation.secrets.reference",
            "secrets reference service stopped"
        );
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.provider.shutdown().await
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.health())
    }
}
