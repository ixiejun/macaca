//! `SystemService` lifecycle implementation for the Application provider.
//!
//! This module owns provider start/stop/cleanup/health hooks.  Command bodies
//! are delegated to `command_dispatch` so the adapter stays a thin syscall surface.

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceResult,
};

use super::ApplicationSystemServiceProvider;

#[async_trait]
impl SystemService for ApplicationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            registry_configured = self.registry.is_some(),
            runtime_configured = self.runtime.is_some(),
            "application service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "application service command accepted"
        );
        self.dispatch_command(command, trace).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "application service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.sessions.write().await.clear();
        self.wasm_sessions.write().await.clear();
        self.genui_surfaces.clear().await;
        tracing::info!(
            service_id = %self.descriptor.id,
            "application service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.registry.is_some() && self.runtime.is_some() && self.kernel.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "application provider dependencies are not fully configured".into(),
            })
        }
    }
}
