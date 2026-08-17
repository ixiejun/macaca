//! Runtime-host Bridge for provider-neutral foundation filesystem services.
//!
//! This adapter is the only integration point between the generic service runtime
//! and a concrete filesystem Strategy. SDK, shell, application-framework, and
//! kernel layers remain unaware of whether the provider is local, remote, mock,
//! plugin-backed, or unavailable.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_foundation_filesystem::{FilesystemService, UnavailableFilesystemProvider};
use macaca_kernel::SystemService;
use macaca_proto::{
    FilesystemProviderCapability, FilesystemProviderSnapshot, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceHealth, ServiceResult,
};

/// Runtime composition Bridge that owns filesystem provider lifecycle delegation.
pub struct FoundationFilesystemSystemServiceProvider {
    provider: Arc<dyn FilesystemService>,
}

impl FoundationFilesystemSystemServiceProvider {
    /// Inject an approved provider Strategy from the runtime-host composition root.
    pub fn new(provider: Arc<dyn FilesystemService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed fallback when no filesystem provider was installed.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableFilesystemProvider::default()))
    }

    /// Return sanitized Memento data for health and replay diagnostics.
    pub fn snapshot(&self) -> FilesystemProviderSnapshot {
        self.provider.snapshot()
    }

    /// Return provider capabilities without exposing host roots or native handles.
    pub fn provider_capabilities(&self) -> FilesystemProviderCapability {
        self.provider.provider_capabilities()
    }

    /// Delegate lifecycle-owned watch cancellation to the selected Strategy.
    pub async fn cancel_watch(&self, watch_checkpoint: &str) -> ServiceResult<()> {
        self.provider.cancel_watch(watch_checkpoint).await
    }
}

#[async_trait]
impl SystemService for FoundationFilesystemSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = "service.foundation.filesystem",
            "foundation filesystem service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.provider.call(command).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.provider.shutdown().await?;
        tracing::info!(
            service_id = "service.foundation.filesystem",
            "foundation filesystem service stopped"
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
