//! Contract tests for the MCP system service provider module tree.

use std::sync::Arc;

use super::support::service_definition_payloads;
use super::*;
use crate::mcp_runtime::McpRuntimeFacade;
use macaca_framework::mcp::McpTransportConfig;
use macaca_kernel::SystemService;
use macaca_proto::{
    ApplicationId, CapabilityToolInvocation, CapabilityToolInvocationResult,
    CapabilityToolInvocationScope, CapabilityToolPolicyHints, McpExposureRefreshCommand,
    McpExposureRefreshResult, McpOAuthLoginCommand, McpOAuthState, McpReloadCommand,
    McpServiceLifecycleScope, McpToolInvokeCommand, ServiceCommand, ServiceCommandName,
    TraceContext, MCP_EXPOSURE_REFRESH_COMMAND, MCP_OAUTH_LOGIN_COMMAND, MCP_RELOAD_COMMAND,
    MCP_TOOL_INVOKE_COMMAND,
};

fn scoped_invocation(trace_id: &str, session_id: &str) -> CapabilityToolInvocation {
    CapabilityToolInvocation {
        trace: TraceContext::new(trace_id),
        scope: CapabilityToolInvocationScope {
            application_id: ApplicationId::new(),
            session_id: session_id.into(),
            agent_name: "agent-a".into(),
        },
        tool_name: "mcp_lookup".into(),
        input: serde_json::json!({"query": "value"}),
        policy: CapabilityToolPolicyHints::default(),
    }
}

fn operator_definition(server_id: &str) -> crate::McpServerDefinition {
    crate::McpServerDefinition {
        id: server_id.into(),
        transport: McpTransportConfig::Stdio {
            command: "missing-test-mcp-binary".into(),
            args: Vec::new(),
            env: std::collections::BTreeMap::new(),
            cwd: None,
        },
        lifecycle: crate::mcp_runtime::McpLifecycleScope::AgentSession,
        session_mode: macaca_framework::mcp::McpSessionMode::Stateful,
        tool_prefix: Some("fixture".into()),
        required_bins: Vec::new(),
        enabled: true,
        source: crate::McpDefinitionSource::Global,
        concurrency_isolation: None,
    }
}

#[tokio::test]
async fn invoke_rejects_missing_trace_before_facade_side_effects() {
    let provider = McpSystemServiceProvider::unavailable();
    let command = McpToolInvokeCommand {
        invocation: scoped_invocation("", "session-a"),
        server_id: Some("server-a".into()),
        backend_tool_name: Some("lookup".into()),
        lifecycle: Some(McpServiceLifecycleScope::AgentSession),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("outer-trace"),
        ))
        .await
        .unwrap();
    let invocation: CapabilityToolInvocationResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(invocation.status, "failed");
    assert_eq!(invocation.error_summary.as_deref(), Some("missing_trace"));
    assert_eq!(
        invocation.metadata.get("mcp.policy_decision"),
        Some(&"deny".to_string())
    );
}

#[tokio::test]
async fn operator_reload_marks_thread_for_next_turn_exposure_refresh() {
    let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
    let app_id = ApplicationId::new();
    let scope =
        macaca_proto::McpServiceScope::agent_session(app_id, "thread-a", "agent-a").unwrap();
    let reload = McpReloadCommand {
        trace: TraceContext::new("trace-mcp-reload"),
        scope: scope.clone(),
        definitions: vec![serde_json::to_value(operator_definition("server-a")).unwrap()],
        reason: Some("test reload".into()),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_RELOAD_COMMAND),
            serde_json::to_value(reload).unwrap(),
            TraceContext::new("trace-mcp-reload"),
        ))
        .await
        .unwrap();
    let reload: macaca_proto::McpReloadResult = serde_json::from_value(result.output).unwrap();
    assert_eq!(reload.reloaded, 1);
    assert_eq!(reload.pending_thread_ids, vec!["thread-a".to_string()]);

    let refresh = McpExposureRefreshCommand {
        trace: TraceContext::new("trace-mcp-refresh"),
        scope,
        thread_id: "thread-a".into(),
        active_turn_id: Some("turn-a".into()),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_EXPOSURE_REFRESH_COMMAND),
            serde_json::to_value(refresh).unwrap(),
            TraceContext::new("trace-mcp-refresh"),
        ))
        .await
        .unwrap();
    let refresh: McpExposureRefreshResult = serde_json::from_value(result.output).unwrap();
    assert!(refresh.refreshed);
    assert_eq!(refresh.exposure_generation, 1);
}

#[tokio::test]
async fn operator_oauth_login_and_status_are_structured() {
    let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
    let command = McpOAuthLoginCommand {
        trace: TraceContext::new("trace-mcp-oauth"),
        scope: macaca_proto::McpServiceScope::default(),
        server_id: "server-a".into(),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_OAUTH_LOGIN_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("trace-mcp-oauth"),
        ))
        .await
        .unwrap();
    let status: macaca_proto::McpOAuthStatusResult = serde_json::from_value(result.output).unwrap();
    assert_eq!(status.state, McpOAuthState::LoginStarted);
    assert!(status.login_flow_ref.is_some());
}

#[tokio::test]
async fn invoke_rejects_missing_scope_before_facade_side_effects() {
    let provider = McpSystemServiceProvider::unavailable();
    let command = McpToolInvokeCommand {
        invocation: scoped_invocation("trace-mcp", ""),
        server_id: Some("server-a".into()),
        backend_tool_name: Some("lookup".into()),
        lifecycle: Some(McpServiceLifecycleScope::AgentSession),
    };
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("outer-trace"),
        ))
        .await
        .unwrap();
    let invocation: CapabilityToolInvocationResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(invocation.status, "failed");
    assert_eq!(invocation.error_summary.as_deref(), Some("scope_missing"));
}

#[tokio::test]
async fn invoke_returns_structured_failure_when_provider_is_unavailable() {
    let provider = McpSystemServiceProvider::unavailable();
    let command = McpToolInvokeCommand {
        invocation: scoped_invocation("trace-mcp", "session-a"),
        server_id: Some("server-a".into()),
        backend_tool_name: Some("lookup".into()),
        lifecycle: Some(McpServiceLifecycleScope::AgentSession),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("outer-trace"),
        ))
        .await
        .unwrap();
    let invocation: CapabilityToolInvocationResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(invocation.status, "failed");
    assert_eq!(
        invocation.error_summary.as_deref(),
        Some("mcp_provider_unavailable")
    );
    assert_eq!(
        invocation.metadata.get("mcp.policy_decision"),
        Some(&"deny".into())
    );
}

#[tokio::test]
async fn invoke_returns_structured_failure_when_oauth_is_required() {
    let provider = McpSystemServiceProvider::new(Arc::new(McpRuntimeFacade::new()));
    let mut invocation = scoped_invocation("trace-mcp-oauth-required", "session-a");
    invocation
        .policy
        .metadata
        .insert("mcp.oauth_required".into(), "true".into());
    let command = McpToolInvokeCommand {
        invocation,
        server_id: Some("server-a".into()),
        backend_tool_name: Some("lookup".into()),
        lifecycle: Some(McpServiceLifecycleScope::AgentSession),
    };

    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(MCP_TOOL_INVOKE_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("outer-trace"),
        ))
        .await
        .unwrap();
    let invocation: CapabilityToolInvocationResult = serde_json::from_value(result.output).unwrap();

    assert_eq!(invocation.status, "failed");
    assert_eq!(
        invocation.error_summary.as_deref(),
        Some("mcp_oauth_required")
    );
    assert_eq!(
        invocation.metadata.get("mcp.policy_decision"),
        Some(&"deny".into())
    );
}

#[test]
fn snapshot_definition_payloads_are_explicitly_requested() {
    let definition = crate::McpServerDefinition {
        id: "server-a".into(),
        transport: McpTransportConfig::Stdio {
            command: "service-binary".into(),
            args: vec!["--stdio".into()],
            env: std::collections::BTreeMap::new(),
            cwd: None,
        },
        lifecycle: crate::mcp_runtime::McpLifecycleScope::AgentSession,
        session_mode: macaca_framework::mcp::McpSessionMode::Stateful,
        tool_prefix: Some("sample".into()),
        required_bins: Vec::new(),
        enabled: true,
        source: crate::McpDefinitionSource::Global,
        concurrency_isolation: None,
    };

    // The snapshot contract uses a Command-style flag because most callers
    // only need counts and health.  Toolkit assembly opts into the payloads
    // during migration, while status dashboards can keep the cheaper and
    // less detailed default view.
    assert!(service_definition_payloads(false, vec![definition.clone()]).is_empty());

    let payloads = service_definition_payloads(true, vec![definition]);
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads[0].get("id").and_then(|value| value.as_str()),
        Some("server-a")
    );
    assert_eq!(
        payloads[0]
            .get("transport")
            .and_then(|value| value.as_str()),
        Some("stdio")
    );
}
