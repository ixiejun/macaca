//! Contract tests for the SDK MCP client service-dispatch boundary.
//!
//! Tests are flattened at the module root (no nested `mod tests`) so escape-hatch
//! gates can scan production modules without false positives from test-only literals.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    ApplicationId, CapabilityToolInvocation, CapabilityToolInvocationResult,
    CapabilityToolInvocationScope, CapabilityToolOriginKind, MacacaResult, TraceContext,
    MCP_SERVICE_ID, MCP_TOOL_INVOKE_COMMAND,
};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

use super::service_backed::ServiceBackedMcpClient;
use super::SystemMcpClient;

/// Capturing test double for the generic SDK service client.
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
        CapabilityToolInvocationScope::new(
            ApplicationId::new(),
            "session-fixture",
            "fixture-sdk-agent",
        )
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
            macaca_proto::McpToolInvokeCommand::routed(
                invocation(),
                "server-fixture",
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
