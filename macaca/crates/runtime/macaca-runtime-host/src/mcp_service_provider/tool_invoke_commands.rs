//! Guarded MCP tool invocation command handler.
//!
//! **Pattern:** Chain of Responsibility — admission checks (trace, scope, oauth,
//! provider availability) run before any facade side effects occur.

use macaca_proto::{
    McpToolInvokeCommand, ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
    TraceContext,
};

use super::support::{
    decode, failed_invocation_result, runtime_policy_from_capability_hints, to_value,
    validate_invocation_command,
};
use super::McpSystemServiceProvider;
use crate::mcp_runtime::McpRuntimeContext;

impl McpSystemServiceProvider {
    pub(super) async fn handle_tool_invoke(
        &self,
        command: ServiceCommand,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: McpToolInvokeCommand = super::support::decode(command.payload)?;
        if let Some(reason) = super::support::validate_invocation_command(&typed) {
            tracing::warn!(
                trace_id = %typed.invocation.trace.trace_id,
                visible_tool = %typed.invocation.tool_name,
                reason,
                "mcp service tool invocation rejected before side effects"
            );
            let result = super::support::failed_invocation_result(
                &typed.invocation.tool_name,
                reason,
                typed.invocation.trace,
            );
            return Ok(McpSystemServiceProvider::service_result(
                super::support::to_value(result)?,
                trace,
            ));
        }
        if self.facade.is_none() {
            tracing::warn!(
                trace_id = %typed.invocation.trace.trace_id,
                visible_tool = %typed.invocation.tool_name,
                "mcp service tool invocation rejected because provider is unavailable"
            );
            let result = super::support::failed_invocation_result(
                &typed.invocation.tool_name,
                "mcp_provider_unavailable",
                typed.invocation.trace,
            );
            return Ok(McpSystemServiceProvider::service_result(
                super::support::to_value(result)?,
                trace,
            ));
        }
        if typed
            .invocation
            .policy
            .metadata
            .get("mcp.oauth_required")
            .is_some_and(|value| value == "true")
        {
            tracing::warn!(
                trace_id = %typed.invocation.trace.trace_id,
                visible_tool = %typed.invocation.tool_name,
                "mcp service tool invocation rejected because oauth is required"
            );
            let result = super::support::failed_invocation_result(
                &typed.invocation.tool_name,
                "mcp_oauth_required",
                typed.invocation.trace,
            );
            return Ok(McpSystemServiceProvider::service_result(
                super::support::to_value(result)?,
                trace,
            ));
        }
        let facade = self.facade()?;
        let server_id = typed.server_id.as_deref().ok_or_else(|| {
            ServiceError::UnsupportedCommand(
                "mcp.tool.invoke requires descriptor server_id metadata".into(),
            )
        })?;
        let backend_tool_name = typed.backend_tool_name.as_deref().ok_or_else(|| {
            ServiceError::UnsupportedCommand(
                "mcp.tool.invoke requires descriptor backend_tool_name metadata".into(),
            )
        })?;
        let context = McpRuntimeContext {
            app_id: Some(typed.invocation.scope.application_id),
            session_id: Some(typed.invocation.scope.session_id.clone()),
            agent_name: Some(typed.invocation.scope.agent_name.clone()),
        };
        tracing::info!(
            trace_id = %typed.invocation.trace.trace_id,
            server_id,
            backend_tool = backend_tool_name,
            visible_tool = %typed.invocation.tool_name,
            lifecycle = ?typed.lifecycle,
            "mcp service tool invocation accepted"
        );
        let result = facade
            .invoke_tool(
                server_id,
                backend_tool_name,
                &typed.invocation.tool_name,
                typed.invocation.input,
                typed.invocation.trace,
                &context,
                &super::support::runtime_policy_from_capability_hints(&typed.invocation.policy),
            )
            .await;
        Ok(McpSystemServiceProvider::service_result(
            super::support::to_value(result)?,
            trace,
        ))
    }
}
