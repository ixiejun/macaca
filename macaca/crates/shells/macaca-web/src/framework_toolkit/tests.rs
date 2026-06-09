//! Unit and contract tests for the framework_toolkit Facade module.

use std::path::Path;

use super::contract_source::framework_toolkit_module_sources;
use super::mcp_bridge::{
mcp_runtime_event_plans, mcp_starting_event_plans, skill_mcp_alias_event_plans,
};
use super::policy::enforce_base_tool_allowlist;
use super::workspace_tools::{normalize_tool_input, resolve_workspace_path};
use async_trait::async_trait;
use macaca_sdk::framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_sdk::framework::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};
use macaca_runtime_host::{McpLifecycleScope, McpRuntimeStatus, McpRuntimeStatusState};

struct NamedTestTool(&'static str);

#[async_trait]
impl ToolHandler for NamedTestTool {
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::text("ok"))
        }

        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &str {
            "test tool"
        }

        fn schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
}

#[test]
fn final_tool_allowlist_removes_late_registered_tools() {
        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(NamedTestTool("shell")), None);
        toolkit.register(Box::new(NamedTestTool("bing_search")), None);
        toolkit.register(Box::new(NamedTestTool("file_write")), None);
        let allowlist = std::collections::HashSet::from(["shell".to_string()]);

        enforce_base_tool_allowlist(&mut toolkit, Some(&allowlist));

        assert!(toolkit.get_tool("shell").is_some());
        assert!(toolkit.get_tool("bing_search").is_none());
        assert!(toolkit.get_tool("file_write").is_none());
}

#[test]
fn resolve_workspace_path_joins_relative_path_to_workspace_root() {
        let workspace_root = Path::new("/tmp/macaca-workspace");

        let resolved = resolve_workspace_path(workspace_root, "shared/service/main.go");

        assert_eq!(
            resolved,
            workspace_root
                .join("shared")
                .join("service")
                .join("main.go")
        );
}

#[test]
fn resolve_workspace_path_preserves_absolute_path() {
        let workspace_root = Path::new("/tmp/macaca-workspace");
        let absolute = "/tmp/absolute/main.go";

        let resolved = resolve_workspace_path(workspace_root, absolute);

        assert_eq!(resolved, Path::new(absolute));
}

#[test]
fn normalize_tool_input_parses_stringified_json_object() {
        let input = serde_json::Value::String(
            r#"{"path":"shared/service/main.go","content":"package main"}"#.into(),
        );

        let normalized = normalize_tool_input(&input);

        assert_eq!(
            normalized.get("path").and_then(|value| value.as_str()),
            Some("shared/service/main.go")
        );
        assert_eq!(
            normalized.get("content").and_then(|value| value.as_str()),
            Some("package main")
        );
}

#[test]
fn normalize_tool_input_keeps_non_json_string_borrowed() {
        let input = serde_json::Value::String("not json".into());

        let normalized = normalize_tool_input(&input);

        assert_eq!(normalized.as_str(), Some("not json"));
}

#[test]
fn production_toolkit_assembly_does_not_register_direct_mcp_clients() {
        let source = framework_toolkit_module_sources();
        let forbidden = concat!("state.", "mcp_runtime.", "register_definitions");

        assert!(
            !source.contains(forbidden),
            "production toolkit assembly must adapt MCP Service descriptors instead of registering host-local MCP clients"
        );
        assert!(
            source.contains("service_tool_from_descriptor"),
            "production toolkit assembly should expose MCP tools through service-backed adapters"
        );
}

#[test]
fn mcp_runtime_event_plan_preserves_legacy_lifecycle_events() {
        let statuses = vec![
            mcp_status(
                "server-ready",
                McpRuntimeStatusState::Ready,
                vec!["lookup".into()],
                None,
            ),
            mcp_status(
                "server-failed",
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some("boom".into()),
            ),
            mcp_status(
                "server-disabled",
                McpRuntimeStatusState::Disabled,
                Vec::new(),
                None,
            ),
        ];

        let event_types = mcp_runtime_event_plans("agent-a", &statuses)
            .into_iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>();

        assert_eq!(
            event_types,
            vec![
                "mcp_server_resolved",
                "mcp_server_ready",
                "mcp_tools_registered",
                "mcp_server_resolved",
                "mcp_server_failed",
            ]
        );
}

#[test]
fn mcp_starting_event_plan_skips_disabled_definitions() {
        let enabled = mcp_definition("server-enabled", true);
        let disabled = mcp_definition("server-disabled", false);

        let plans = mcp_starting_event_plans("agent-a", &[enabled, disabled]);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].event_type, "mcp_server_starting");
        assert_eq!(
            plans[0]
                .payload
                .get("server_id")
                .and_then(|value| value.as_str()),
            Some("server-enabled")
        );
}

#[test]
fn skill_mcp_alias_event_plan_preserves_ready_and_tool_aliases() {
        let statuses = vec![
            mcp_status(
                "skill:browser",
                McpRuntimeStatusState::Ready,
                vec!["browser_click".into()],
                None,
            ),
            mcp_status(
                "global-browser",
                McpRuntimeStatusState::Ready,
                vec!["browser_click".into()],
                None,
            ),
        ];

        let event_types = skill_mcp_alias_event_plans("agent-a", &statuses)
            .into_iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>();

        assert_eq!(
            event_types,
            vec!["skill_mcp_ready", "skill_mcp_tools_registered"]
        );
}

fn mcp_definition(id: &str, enabled: bool) -> macaca_runtime_host::McpServerDefinition {
        macaca_runtime_host::McpServerDefinition {
            id: id.into(),
            transport: McpTransportConfig::Stdio {
                command: "sh".into(),
                args: Vec::new(),
                env: Default::default(),
                cwd: None,
            },
            lifecycle: McpLifecycleScope::AgentSession,
            session_mode: McpSessionMode::Stateful,
            tool_prefix: None,
            required_bins: Vec::new(),
            enabled,
            source: macaca_runtime_host::McpDefinitionSource::Global,
            concurrency_isolation: None,
        }
}

fn mcp_status(
        server_id: &str,
        state: McpRuntimeStatusState,
        exposed_tools: Vec<String>,
        failure_reason: Option<String>,
) -> McpRuntimeStatus {
        McpRuntimeStatus {
            server_id: server_id.into(),
            transport: "stdio".into(),
            lifecycle: McpLifecycleScope::AgentSession,
            session_mode: McpSessionMode::Stateful,
            state,
            exposed_tools,
            failure_reason,
        }
    }
