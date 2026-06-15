//! SDK MCP client facade (**Facade** module root).
//!
//! MCP protocol sessions and cleanup are host-owned runtime concerns.  The SDK
//! remains a thin client: it does not construct MCP runtimes or toolkits.
//!
//! # Module tree (P5 iteration 94)
//! - `support` — generic service-call bridge (`call`)
//! - `unavailable` — Null Object client for composition roots without MCP runtime
//! - `service_backed` — Adapter over `SystemServiceClient`
//! - `tests` — contract tests for service dispatch boundary

mod service_backed;
mod support;
mod unavailable;

#[cfg(test)]
mod tests;

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
    McpToolInvokeCommand, McpToolInvokeResult,
};

pub use service_backed::ServiceBackedMcpClient;
pub use unavailable::UnavailableSystemMcpClient;

/// Focused MCP client consumed by Web, CLI, framework, and applications.
///
/// The trait is the stable SDK boundary.  Implementations use either the Null
/// Object (`UnavailableSystemMcpClient`) or the service Adapter
/// (`ServiceBackedMcpClient`) depending on composition-root wiring.
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
    async fn server_status_list(
        &self,
        command: McpServerStatusListCommand,
    ) -> MacacaResult<McpServerStatusListResult>;
    async fn reload(&self, command: McpReloadCommand) -> MacacaResult<McpReloadResult>;
    async fn oauth_login(
        &self,
        command: McpOAuthLoginCommand,
    ) -> MacacaResult<McpOAuthStatusResult>;
    async fn oauth_status(
        &self,
        command: McpOAuthStatusCommand,
    ) -> MacacaResult<McpOAuthStatusResult>;
    async fn resource_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceListResult>;
    async fn resource_template_list(
        &self,
        command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceTemplateListResult>;
    async fn resource_read(
        &self,
        command: McpResourceReadCommand,
    ) -> MacacaResult<McpResourceReadResult>;
    async fn diagnostics_snapshot(
        &self,
        command: McpDiagnosticsSnapshotCommand,
    ) -> MacacaResult<McpDiagnosticsSnapshot>;
    async fn exposure_refresh(
        &self,
        command: McpExposureRefreshCommand,
    ) -> MacacaResult<McpExposureRefreshResult>;
    async fn snapshot(
        &self,
        command: McpServiceSnapshotCommand,
    ) -> MacacaResult<McpServiceSnapshot>;
    async fn cleanup(&self, command: McpCleanupCommand) -> MacacaResult<McpStatusResult>;
}
