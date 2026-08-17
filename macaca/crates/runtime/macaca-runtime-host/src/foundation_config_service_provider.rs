//! Bridge from the generic runtime service protocol to foundation config providers.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_foundation_config::{ConfigService, UnavailableConfigProvider};
use macaca_kernel::SystemService;
use macaca_proto::{
    ConfigProviderCapability, ConfigProviderSnapshot, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceHealth, ServiceResult,
};

/// Runtime composition adapter; callers remain unaware of concrete config sources.
pub struct FoundationConfigSystemServiceProvider {
    provider: Arc<dyn ConfigService>,
}

impl FoundationConfigSystemServiceProvider {
    /// Wrap a host, workspace, remote, mock, or unavailable configuration provider.
    pub fn new(provider: Arc<dyn ConfigService>) -> Self {
        Self { provider }
    }
    /// Build the fail-closed provider used when no source has been installed.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableConfigProvider::default()))
    }
    /// Return the provider's sanitized Memento for replay diagnostics.
    pub fn snapshot(&self) -> ConfigProviderSnapshot {
        self.provider.snapshot()
    }
    /// Return sanitized provider capability facts for health and discovery surfaces.
    pub fn provider_capabilities(&self) -> ConfigProviderCapability {
        self.provider.provider_capabilities()
    }
    /// Forward watch cancellation through the provider lifecycle boundary.
    pub async fn cancel_watch(&self, watch_checkpoint: &str) -> ServiceResult<()> {
        self.provider.cancel_watch(watch_checkpoint).await
    }
}

#[async_trait]
impl SystemService for FoundationConfigSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }
    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = "service.foundation.config",
            "foundation config service started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.provider.call(command).await
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        tracing::info!(
            service_id = "service.foundation.config",
            "foundation config service stopped"
        );
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.health())
    }
}
