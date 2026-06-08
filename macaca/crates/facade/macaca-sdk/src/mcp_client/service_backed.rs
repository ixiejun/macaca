//! Runtime-backed MCP client implemented over the generic SDK service client.
//!
//! This module is the Adapter half of the SDK MCP facade: it maps typed MCP
//! command DTOs onto `SystemServiceClient::call_service` without importing MCP
//! runtime internals.  Runtime-host remains the sole owner of protocol sessions.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaResult, McpCleanupCommand, McpDiagnosticsSnapshot, McpDiagnosticsSnapshotCommand,
    McpExposureRefreshCommand, McpExposureRefreshResult, McpOAuthLoginCommand,
    McpOAuthStatusCommand, McpOAuthStatusResult, McpProbeCommand, McpRegisterCommand,
    McpRegisterResult, McpReloadCommand, McpReloadResult, McpResourceListCommand,
    McpResourceListResult, McpResourceReadCommand, McpResourceReadResult,
    McpResourceTemplateListResult, McpServerStatusListCommand, McpServerStatusListResult,
    McpServiceSnapshot, McpServiceSnapshotCommand, McpStatusCommand, McpStatusResult,
    McpToolAttachCommand, McpToolAttachResult, McpToolCatalogCommand, McpToolCatalogResult,
    McpToolInvokeCommand, McpToolInvokeResult, MCP_CLEANUP_COMMAND,
    MCP_DIAGNOSTICS_SNAPSHOT_COMMAND, MCP_EXPOSURE_REFRESH_COMMAND, MCP_OAUTH_LOGIN_COMMAND,
    MCP_OAUTH_STATUS_COMMAND, MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND, MCP_RELOAD_COMMAND,
    MCP_RESOURCE_LIST_COMMAND, MCP_RESOURCE_READ_COMMAND, MCP_RESOURCE_TEMPLATE_LIST_COMMAND,
    MCP_SERVER_STATUS_LIST_COMMAND, MCP_SNAPSHOT_COMMAND, MCP_STATUS_COMMAND,
    MCP_TOOL_ATTACH_COMMAND, MCP_TOOL_CATALOG_COMMAND, MCP_TOOL_INVOKE_COMMAND,
};

use crate::service_client::SystemServiceClient;

use super::support::call;
use super::SystemMcpClient;

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

    async fn server_status_list(
        &self,
        command: McpServerStatusListCommand,
    ) -> MacacaResult<McpServerStatusListResult> {
        call(
            &self.service,
            MCP_SERVER_STATUS_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn reload(&self, command: McpReloadCommand) -> MacacaResult<McpReloadResult> {
        call(
            &self.service,
            MCP_RELOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn oauth_login(
        &self,
        command: McpOAuthLoginCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        call(
            &self.service,
            MCP_OAUTH_LOGIN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn oauth_status(
        &self,
        command: McpOAuthStatusCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        call(
            &self.service,
            MCP_OAUTH_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resource_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceListResult> {
        call(
            &self.service,
            MCP_RESOURCE_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resource_template_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceTemplateListResult> {
        call(
            &self.service,
            MCP_RESOURCE_TEMPLATE_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resource_read(
        &self,
        command: McpResourceReadCommand,
    ) -> MacacaResult<McpResourceReadResult> {
        call(
            &self.service,
            MCP_RESOURCE_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn diagnostics_snapshot(
        &self,
        command: McpDiagnosticsSnapshotCommand,
    ) -> MacacaResult<McpDiagnosticsSnapshot> {
        call(
            &self.service,
            MCP_DIAGNOSTICS_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn exposure_refresh(
        &self,
        command: McpExposureRefreshCommand,
    ) -> MacacaResult<McpExposureRefreshResult> {
        call(
            &self.service,
            MCP_EXPOSURE_REFRESH_COMMAND,
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
