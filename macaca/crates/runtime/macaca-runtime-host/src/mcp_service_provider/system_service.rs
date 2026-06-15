//! `SystemService` lifecycle implementation for the MCP provider.
//!
//! This module owns provider start/stop/cleanup/health hooks.  Command bodies are
//! delegated to `command_dispatch` so the adapter stays a thin syscall surface.

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceResult,
};

use super::McpSystemServiceProvider;

#[async_trait]
impl SystemService for McpSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.facade.is_some(),
            "mcp service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "mcp service command accepted"
        );
        self.dispatch_command(command, trace).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "mcp service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "mcp service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.facade.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "MCP runtime is not configured".into(),
            })
        }
    }
}
