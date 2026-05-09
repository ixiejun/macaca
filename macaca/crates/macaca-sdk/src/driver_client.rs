//! SDK Driver client facade for Route C S6.
//!
//! The SDK owns a caller-friendly Facade over Driver Service commands.  It does
//! not construct driver runtimes; it only translates typed commands into the
//! generic `SystemServiceClient` boundary or returns structured unavailable.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_driver::{
    DriverCleanupCommand, DriverInventoryCommand, DriverInventoryResult, DriverLoadServiceCommand,
    DriverLoadServiceResult, DriverServiceSnapshot, DriverServiceSnapshotCommand,
    DriverStatusCommand, DriverStatusResult, DriverToolCatalogCommand, DriverToolCatalogResult,
    DriverToolInvokeCommand, DriverToolInvokeResult, DRIVER_CLEANUP_COMMAND,
    DRIVER_INVENTORY_COMMAND, DRIVER_LOAD_COMMAND, DRIVER_RELOAD_COMMAND, DRIVER_SERVICE_ID,
    DRIVER_SNAPSHOT_COMMAND, DRIVER_STATUS_COMMAND, DRIVER_TOOL_CATALOG_COMMAND,
    DRIVER_TOOL_INVOKE_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Driver client consumed by Web, CLI, framework, and applications.
#[async_trait]
pub trait SystemDriverClient: Send + Sync {
    async fn load(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult>;
    async fn reload(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult>;
    async fn inventory(
        &self,
        command: DriverInventoryCommand,
    ) -> MacacaResult<DriverInventoryResult>;
    async fn tool_catalog(
        &self,
        command: DriverToolCatalogCommand,
    ) -> MacacaResult<DriverToolCatalogResult>;
    async fn invoke_tool(
        &self,
        command: DriverToolInvokeCommand,
    ) -> MacacaResult<DriverToolInvokeResult>;
    async fn status(&self, command: DriverStatusCommand) -> MacacaResult<DriverStatusResult>;
    async fn snapshot(
        &self,
        command: DriverServiceSnapshotCommand,
    ) -> MacacaResult<DriverServiceSnapshot>;
    async fn cleanup(&self, command: DriverCleanupCommand) -> MacacaResult<serde_json::Value>;
}

/// Null-object Driver client used when no runtime-backed service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemDriverClient;

#[async_trait]
impl SystemDriverClient for UnavailableSystemDriverClient {
    async fn load(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk driver client unavailable for load");
        Err(MacacaError::Config("Driver service is unavailable".into()))
    }

    async fn reload(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk driver client unavailable for reload");
        Err(MacacaError::Config("Driver service is unavailable".into()))
    }

    async fn inventory(
        &self,
        command: DriverInventoryCommand,
    ) -> MacacaResult<DriverInventoryResult> {
        info!(trace_id = %command.trace.trace_id, "sdk driver client returning empty inventory");
        Ok(DriverInventoryResult::new(Vec::new()))
    }

    async fn tool_catalog(
        &self,
        command: DriverToolCatalogCommand,
    ) -> MacacaResult<DriverToolCatalogResult> {
        info!(trace_id = %command.trace.trace_id, "sdk driver client returning empty catalog");
        Ok(DriverToolCatalogResult::new(Vec::new()))
    }

    async fn invoke_tool(
        &self,
        command: DriverToolInvokeCommand,
    ) -> MacacaResult<DriverToolInvokeResult> {
        warn!(
            trace_id = %command.invocation.trace.trace_id,
            tool = %command.invocation.tool_name,
            "sdk driver client unavailable for tool invocation"
        );
        Err(MacacaError::Config("Driver service is unavailable".into()))
    }

    async fn status(&self, command: DriverStatusCommand) -> MacacaResult<DriverStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk driver client returning unavailable status");
        Ok(DriverStatusResult {
            service_id: DRIVER_SERVICE_ID.into(),
            healthy: false,
            loaded: 0,
            failed: 0,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn snapshot(
        &self,
        command: DriverServiceSnapshotCommand,
    ) -> MacacaResult<DriverServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk driver client returning unavailable snapshot");
        Ok(DriverServiceSnapshot::unavailable(
            "runtime-backed Driver service is not installed",
        ))
    }

    async fn cleanup(&self, command: DriverCleanupCommand) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk driver client cleanup no-op");
        Ok(serde_json::json!({"status": "unavailable"}))
    }
}

/// Runtime-backed Driver client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedDriverClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedDriverClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemDriverClient for ServiceBackedDriverClient {
    async fn load(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult> {
        call(
            &self.service,
            DRIVER_LOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn reload(
        &self,
        command: DriverLoadServiceCommand,
    ) -> MacacaResult<DriverLoadServiceResult> {
        call(
            &self.service,
            DRIVER_RELOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn inventory(
        &self,
        command: DriverInventoryCommand,
    ) -> MacacaResult<DriverInventoryResult> {
        call(
            &self.service,
            DRIVER_INVENTORY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn tool_catalog(
        &self,
        command: DriverToolCatalogCommand,
    ) -> MacacaResult<DriverToolCatalogResult> {
        call(
            &self.service,
            DRIVER_TOOL_CATALOG_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invoke_tool(
        &self,
        command: DriverToolInvokeCommand,
    ) -> MacacaResult<DriverToolInvokeResult> {
        call(
            &self.service,
            DRIVER_TOOL_INVOKE_COMMAND,
            command.invocation.trace.clone(),
            command,
        )
        .await
    }

    async fn status(&self, command: DriverStatusCommand) -> MacacaResult<DriverStatusResult> {
        call(
            &self.service,
            DRIVER_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: DriverServiceSnapshotCommand,
    ) -> MacacaResult<DriverServiceSnapshot> {
        call(
            &self.service,
            DRIVER_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cleanup(&self, command: DriverCleanupCommand) -> MacacaResult<serde_json::Value> {
        call(
            &self.service,
            DRIVER_CLEANUP_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        DRIVER_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
