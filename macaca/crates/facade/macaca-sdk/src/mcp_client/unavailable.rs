//! Null-object MCP client used when no runtime-backed service is installed.
//!
//! The type follows the Null Object pattern: shells and tests can depend on
//! [`super::SystemMcpClient`] without branching on `Option`.  Read-only queries
//! return empty sanitized snapshots; mutating commands fail with a stable config
//! error so callers can surface a neutral unavailable state.

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, McpCleanupCommand, McpDiagnosticsSnapshot,
    McpDiagnosticsSnapshotCommand, McpExposureRefreshCommand, McpExposureRefreshResult,
    McpOAuthLoginCommand, McpOAuthStatusCommand, McpOAuthStatusResult, McpProbeCommand,
    McpRegisterCommand, McpRegisterResult, McpReloadCommand, McpReloadResult,
    McpResourceListCommand, McpResourceListResult, McpResourceReadCommand, McpResourceReadResult,
    McpResourceTemplateListResult, McpRuntimeStatusView, McpServerStatusListCommand,
    McpServerStatusListResult, McpServiceSnapshot, McpServiceSnapshotCommand, McpStatusCommand,
    McpStatusResult, McpToolAttachCommand, McpToolAttachResult, McpToolCatalogCommand,
    McpToolCatalogResult, McpToolInvokeCommand, McpToolInvokeResult, MCP_SERVICE_ID,
};
use tracing::{info, warn};

use super::SystemMcpClient;

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

    async fn server_status_list(
        &self,
        command: McpServerStatusListCommand,
    ) -> MacacaResult<McpServerStatusListResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty operator status");
        Ok(McpServerStatusListResult {
            statuses: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn reload(&self, command: McpReloadCommand) -> MacacaResult<McpReloadResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk mcp client unavailable for reload");
        Err(MacacaError::Config("MCP service is unavailable".into()))
    }

    async fn oauth_login(
        &self,
        command: McpOAuthLoginCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk mcp client unavailable for oauth login");
        Err(MacacaError::Config("MCP service is unavailable".into()))
    }

    async fn oauth_status(
        &self,
        command: McpOAuthStatusCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk mcp client unavailable for oauth status");
        Err(MacacaError::Config("MCP service is unavailable".into()))
    }

    async fn resource_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceListResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty resources");
        Ok(McpResourceListResult {
            resources: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn resource_template_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceTemplateListResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning empty resource templates");
        Ok(McpResourceTemplateListResult {
            templates: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn resource_read(
        &self,
        command: McpResourceReadCommand,
    ) -> MacacaResult<McpResourceReadResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning unavailable resource read");
        Ok(McpResourceReadResult::denied(
            command.server_id,
            command.uri,
            "runtime-backed MCP service is not installed",
        ))
    }

    async fn diagnostics_snapshot(
        &self,
        command: McpDiagnosticsSnapshotCommand,
    ) -> MacacaResult<McpDiagnosticsSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client returning unavailable diagnostics");
        Ok(McpDiagnosticsSnapshot {
            service_id: MCP_SERVICE_ID.into(),
            exposure_generation: 0,
            server_count: 0,
            auth_required: 0,
            ready: 0,
            failed: 0,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn exposure_refresh(
        &self,
        command: McpExposureRefreshCommand,
    ) -> MacacaResult<McpExposureRefreshResult> {
        info!(trace_id = %command.trace.trace_id, "sdk mcp client exposure refresh no-op");
        Ok(McpExposureRefreshResult {
            thread_id: command.thread_id,
            refreshed: false,
            exposure_generation: 0,
            visible_tool_count: 0,
            captured_at: chrono::Utc::now(),
        })
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
