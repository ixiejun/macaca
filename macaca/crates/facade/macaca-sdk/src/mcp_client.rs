//! SDK MCP client facade for Route C S6.
//!
//! MCP service contracts live in `macaca-runtime-host` because host runtime owns
//! protocol sessions and cleanup.  The SDK still remains a thin client: it does
//! not construct MCP runtimes or toolkits.

use std::sync::Arc;

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
    McpToolCatalogResult, McpToolInvokeCommand, McpToolInvokeResult, MCP_CLEANUP_COMMAND,
    MCP_DIAGNOSTICS_SNAPSHOT_COMMAND, MCP_EXPOSURE_REFRESH_COMMAND, MCP_OAUTH_LOGIN_COMMAND,
    MCP_OAUTH_STATUS_COMMAND, MCP_PROBE_COMMAND, MCP_REGISTER_COMMAND, MCP_RELOAD_COMMAND,
    MCP_RESOURCE_LIST_COMMAND, MCP_RESOURCE_READ_COMMAND, MCP_RESOURCE_TEMPLATE_LIST_COMMAND,
    MCP_SERVER_STATUS_LIST_COMMAND, MCP_SERVICE_ID, MCP_SNAPSHOT_COMMAND, MCP_STATUS_COMMAND,
    MCP_TOOL_ATTACH_COMMAND, MCP_TOOL_CATALOG_COMMAND, MCP_TOOL_INVOKE_COMMAND,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use macaca_proto::{
        ApplicationId, CapabilityToolInvocation, CapabilityToolInvocationResult,
        CapabilityToolInvocationScope, CapabilityToolOriginKind, TraceContext,
    };

    #[derive(Default)]
    struct CapturingServiceClient {
        calls: Mutex<Vec<ServiceCallCommand>>,
    }

    #[async_trait]
    impl SystemServiceClient for CapturingServiceClient {
        async fn inspect_services(
            &self,
            command: &crate::service_client::ServiceInspectionCommand,
        ) -> MacacaResult<crate::service_client::ServiceInspectionResult> {
            Ok(crate::service_client::ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![MCP_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<crate::service_client::ServiceCallResult> {
            self.calls.lock().unwrap().push(command.clone());
            let trace = command.trace.clone().unwrap();
            Ok(crate::service_client::ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(CapabilityToolInvocationResult::ok(
                    MCP_SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    "mcp_lookup",
                    serde_json::json!({"ok": true}),
                    trace,
                ))?,
            })
        }
    }

    fn invocation() -> CapabilityToolInvocation {
        CapabilityToolInvocation::new(
            TraceContext::new("trace-sdk-mcp-invoke"),
            CapabilityToolInvocationScope::new(ApplicationId::new(), "session-a", "agent-a")
                .unwrap(),
            "mcp_lookup",
            serde_json::json!({"query": "value"}),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn invoke_tool_dispatches_through_generic_service_client() {
        let service = Arc::new(CapturingServiceClient::default());
        let client = ServiceBackedMcpClient::new(service.clone());
        let result = client
            .invoke_tool(
                McpToolInvokeCommand::routed(
                    invocation(),
                    "server-a",
                    "lookup",
                    macaca_proto::McpServiceLifecycleScope::AgentSession,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(result.status, "ok");
        let calls = service.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].service_id, MCP_SERVICE_ID);
        assert_eq!(calls[0].command_name, MCP_TOOL_INVOKE_COMMAND);
        assert_eq!(
            calls[0].trace.as_ref().unwrap().trace_id,
            "trace-sdk-mcp-invoke"
        );
    }
}
