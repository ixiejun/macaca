//! Service-owned MCP tool invocation contract tests (Template Method path).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use macaca_framework::mcp::McpTimeouts;
use macaca_proto::{
    ApplicationId, CapabilityToolOriginKind, MCP_SERVICE_ID, TraceContext,
};

use crate::mcp_runtime::{
    McpRuntimeContext, McpRuntimeFacade, McpRuntimeManager, McpToolPolicy,
};

use super::fixtures::{
    manager_with_fixture_client, seed_descriptor_route, stdio_definition, TestMcpClientBehavior,
};

#[tokio::test]
async fn service_owned_invocation_returns_structured_failure_for_unknown_server() {
    let facade = McpRuntimeFacade::new();
    let result = facade
        .invoke_tool(
            "missing",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-test-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.service_id, MCP_SERVICE_ID);
    assert_eq!(result.origin_kind, CapabilityToolOriginKind::Mcp);
    assert_eq!(result.status, "failed");
    assert_eq!(result.error_summary.as_deref(), Some("unknown_mcp_server"));
}

#[tokio::test]
async fn service_owned_invocation_rejects_unknown_descriptor_route() {
    let manager = Arc::new(McpRuntimeManager::new());
    manager
        .upsert_definition(stdio_definition("server-a", "server-bin"))
        .await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "mcp_lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-test-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(
        result.error_summary.as_deref(),
        Some("mcp_descriptor_route_unknown")
    );
}

#[tokio::test]
async fn service_owned_invocation_denies_tool_policy_before_client_creation() {
    let manager = Arc::new(McpRuntimeManager::new());
    let definition = stdio_definition("server-a", "sh");
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);
    let mut deny_tools = HashSet::new();
    deny_tools.insert("lookup".to_string());

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-test-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy {
                deny_tools,
                ..Default::default()
            },
        )
        .await;

    assert_eq!(result.error_summary.as_deref(), Some("mcp_tool_denied"));
}

#[tokio::test]
async fn service_owned_invocation_reports_missing_binary_before_dispatch() {
    let manager = Arc::new(McpRuntimeManager::new());
    let definition = stdio_definition("server-a", "definitely-missing-mcp-bin");
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-test-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(
        result.error_summary.as_deref(),
        Some("missing dependency: definitely-missing-mcp-bin")
    );
}

#[tokio::test]
async fn service_owned_invocation_returns_success_with_sanitized_hash_metadata() {
    let manager =
        manager_with_fixture_client(TestMcpClientBehavior::Success, McpTimeouts::default());
    let mut definition = stdio_definition("server-a", "sh");
    definition.required_bins.clear();
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({"query": "alpha"}),
            TraceContext::new("mcp-success-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.status, "ok");
    assert_eq!(result.metadata.get("mcp.reason_code"), Some(&"ok".into()));
    assert!(result.metadata.contains_key("mcp.input_hash"));
    assert!(result.metadata.contains_key("mcp.output_hash"));
    assert!(
        !serde_json::to_string(&result.metadata)
            .unwrap()
            .contains("alpha"),
        "audit metadata must carry stable hashes instead of raw tool input"
    );
}

#[tokio::test]
async fn service_owned_invocation_reports_protocol_client_failures() {
    let manager = manager_with_fixture_client(
        TestMcpClientBehavior::ConnectFailure,
        McpTimeouts::default(),
    );
    let mut definition = stdio_definition("server-a", "sh");
    definition.required_bins.clear();
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-client-failure-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.status, "failed");
    assert_eq!(
        result.error_summary.as_deref(),
        Some("Connection error: fixture connect failed")
    );
    assert_eq!(
        result.metadata.get("mcp.reason_code"),
        Some(&"Connection error: fixture connect failed".into())
    );
}

#[tokio::test]
async fn service_owned_invocation_reports_protocol_call_failures() {
    let manager =
        manager_with_fixture_client(TestMcpClientBehavior::CallFailure, McpTimeouts::default());
    let mut definition = stdio_definition("server-a", "sh");
    definition.required_bins.clear();
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-call-failure-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.status, "failed");
    assert_eq!(
        result.error_summary.as_deref(),
        Some("Execution error: fixture call failed")
    );
    assert_eq!(
        result.metadata.get("mcp.reason_code"),
        Some(&"Execution error: fixture call failed".into())
    );
}

#[tokio::test]
async fn service_owned_invocation_reports_mcp_tool_error_results() {
    let manager =
        manager_with_fixture_client(TestMcpClientBehavior::ToolError, McpTimeouts::default());
    let mut definition = stdio_definition("server-a", "sh");
    definition.required_bins.clear();
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-tool-error-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.status, "failed");
    assert_eq!(
        result.error_summary.as_deref(),
        Some(r#"[{"type":"text","text":"fixture tool rejected input"}]"#)
    );
}

#[tokio::test]
async fn service_owned_invocation_reports_call_timeout() {
    let manager = manager_with_fixture_client(
        TestMcpClientBehavior::CallTimeout,
        McpTimeouts {
            connect: Duration::from_millis(50),
            list_tools: Duration::from_millis(50),
            call_tool: Duration::from_millis(1),
        },
    );
    let mut definition = stdio_definition("server-a", "sh");
    definition.required_bins.clear();
    manager.upsert_definition(definition.clone()).await;
    seed_descriptor_route(&manager, &definition, "lookup", "lookup").await;
    let facade = McpRuntimeFacade::from_manager(manager);

    let result = facade
        .invoke_tool(
            "server-a",
            "lookup",
            "lookup",
            serde_json::json!({}),
            TraceContext::new("mcp-timeout-trace"),
            &McpRuntimeContext {
                app_id: Some(ApplicationId(uuid::Uuid::nil())),
                session_id: Some("session-a".into()),
                agent_name: Some("agent-a".into()),
            },
            &McpToolPolicy::default(),
        )
        .await;

    assert_eq!(result.status, "failed");
    assert_eq!(result.error_summary.as_deref(), Some("call_tool_timeout"));
}
