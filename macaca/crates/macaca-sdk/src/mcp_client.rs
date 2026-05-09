//! SDK MCP client facade for Route C S6.
//!
//! MCP service contracts live in `macaca-runtime-host` because host runtime owns
//! protocol sessions and cleanup.  The SDK still remains a thin client: it does
//! not construct MCP runtimes or toolkits.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, McpCleanupCommand, McpProbeCommand, McpRegisterCommand,
    McpRegisterResult, McpRuntimeStatusView, McpServiceSnapshot, McpServiceSnapshotCommand,
    McpStatusCommand, McpStatusResult, McpToolAttachCommand, McpToolAttachResult,
    McpToolCatalogCommand, McpToolCatalogResult, McpToolInvokeCommand, McpToolInvokeResult,
    MCP_CLEANUP_COMMAND, MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND, MCP_SERVICE_ID,
    MCP_SNAPSHOT_COMMAND, MCP_STATUS_COMMAND, MCP_TOOL_ATTACH_COMMAND, MCP_TOOL_CATALOG_COMMAND,
    MCP_TOOL_INVOKE_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused MCP client consumed by Web, CLI, framework, and applications.
#[async_trait]
pub trait SystemMcpClient: Send + Sync {
    async fn register(&self, command: McpRegisterCommand) -> MacacaResult<McpRegisterResult>;
    async fn probe(&self, command: McpProbeCommand) -> MacacaResult<McpStatusResult>;
    async fn tool_catalog(
        &self,
        command: McpToolCatalogCommand,
    ) -> MacacaResult<McpToolCatalogResult>;
    async fn attach_tools(
        &self,
        command: McpToolAttachCommand,
    ) -> MacacaResult<McpToolAttachResult>;
    async fn invoke_tool(&self, command: McpToolInvokeCommand)
        -> MacacaResult<McpToolInvokeResult>;
    async fn status(&self, command: McpStatusCommand) -> MacacaResult<McpStatusResult>;
    async fn snapshot(
        &self,
        command: McpServiceSnapshotCommand,
    ) -> MacacaResult<McpServiceSnapshot>;
    async fn cleanup(&self, command: McpCleanupCommand) -> MacacaResult<McpStatusResult>;
}

/// Null-object MCP client used when no runtime-backed service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemMcpClient;

#[async_trait]
impl SystemMcpClient for UnavailableSystemMcpClient {
    async fn register(&self, command: McpRegisterCommand) -> MacacaResult<McpRegisterResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk mcp client unavailable for register");
        Err(MacacaError::Config("MCP service is unavailable".into()))
    }

    async fn probe(&self, command: McpProbeCommand) -> MacacaResult<McpStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty probe");
        Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
    }

    async fn tool_catalog(
        &self,
        command: McpToolCatalogCommand,
    ) -> MacacaResult<McpToolCatalogResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty catalog");
        Ok(McpToolCatalogResult {
            tools: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn attach_tools(
        &self,
        command: McpToolAttachCommand,
    ) -> MacacaResult<McpToolAttachResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty attach result");
        Ok(McpToolAttachResult {
            statuses: Vec::new(),
            conflicts: Vec::new(),
            applied_prefixes: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn invoke_tool(
        &self,
        command: McpToolInvokeCommand,
    ) -> MacacaResult<McpToolInvokeResult> {
        warn!(
            trace_id = %command.invocation.trace.trace_id,
            tool = %command.invocation.tool_name,
            "sdk mcp client unavailable for invocation"
        );
        Err(MacacaError::Config("MCP service is unavailable".into()))
    }

    async fn status(&self, command: McpStatusCommand) -> MacacaResult<McpStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty status");
        Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
    }

    async fn snapshot(
        &self,
        command: McpServiceSnapshotCommand,
    ) -> MacacaResult<McpServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning unavailable snapshot");
        Ok(McpServiceSnapshot::unavailable(
            "runtime-backed MCP service is not installed",
        ))
    }

    async fn cleanup(&self, command: McpCleanupCommand) -> MacacaResult<McpStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client cleanup no-op");
        Ok(McpStatusResult::new(Vec::<McpRuntimeStatusView>::new()))
    }
}

/// Runtime-backed MCP client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedMcpClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedMcpClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemMcpClient for ServiceBackedMcpClient {
    async fn register(&self, command: McpRegisterCommand) -> MacacaResult<McpRegisterResult> {
        call(
            &self.service,
            MCP_REGISTER_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn probe(&self, command: McpProbeCommand) -> MacacaResult<McpStatusResult> {
        call(
            &self.service,
            MCP_PROBE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn tool_catalog(
        &self,
        command: McpToolCatalogCommand,
    ) -> MacacaResult<McpToolCatalogResult> {
        call(
            &self.service,
            MCP_TOOL_CATALOG_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn attach_tools(
        &self,
        command: McpToolAttachCommand,
    ) -> MacacaResult<McpToolAttachResult> {
        call(
            &self.service,
            MCP_TOOL_ATTACH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invoke_tool(
        &self,
        command: McpToolInvokeCommand,
    ) -> MacacaResult<McpToolInvokeResult> {
        call(
            &self.service,
            MCP_TOOL_INVOKE_COMMAND,
            command.invocation.trace.clone(),
            command,
        )
        .await
    }

    async fn status(&self, command: McpStatusCommand) -> MacacaResult<McpStatusResult> {
        call(
            &self.service,
            MCP_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: McpServiceSnapshotCommand,
    ) -> MacacaResult<McpServiceSnapshot> {
        call(
            &self.service,
            MCP_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cleanup(&self, command: McpCleanupCommand) -> MacacaResult<McpStatusResult> {
        call(
            &self.service,
            MCP_CLEANUP_COMMAND,
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
    let service_command =
        ServiceCallCommand::new(MCP_SERVICE_ID, command_name, serde_json::to_value(payload)?)?
            .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
