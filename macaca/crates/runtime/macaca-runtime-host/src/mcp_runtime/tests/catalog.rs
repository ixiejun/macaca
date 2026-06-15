//! Catalog, config adapter, policy, and descriptor contract tests.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use macaca_framework::mcp::McpTransportConfig;
use macaca_proto::{
    MCP_DESCRIPTOR_BACKEND_TOOL_NAME, MCP_DESCRIPTOR_LIFECYCLE_SCOPE, MCP_SERVICE_ID,
};
use macaca_skill::{
    SkillInstallSpec, SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry, SkillSourceScope,
};

use crate::mcp_runtime::{
    apply_concurrency_isolation, ConcurrencyIsolationPolicy, McpDefinitionSource,
    McpLifecycleScope, McpRegistryConfig, McpRuntimeFacade, McpToolPolicy,
};
use crate::McpServerFactory;

use super::super::descriptors::descriptor_from_tool;
use super::fixtures::{manager_with_fixture_client, stdio_definition, TestMcpClientBehavior};

#[tokio::test]
async fn managed_resource_listing_and_read_use_mcp_protocol_client() {
    let manager = manager_with_fixture_client(TestMcpClientBehavior::Success, Default::default());
    let facade = McpRuntimeFacade::from_manager(manager);
    let mut definition = stdio_definition("server-a", "fixture-mcp");
    definition.transport = McpTransportConfig::StreamableHttp {
        url: "http://127.0.0.1/mcp".into(),
        headers: BTreeMap::new(),
    };
    definition.required_bins.clear();
    facade.upsert_definition(definition).await;

    let resources = facade
        .list_resources(Some("server-a"), &McpToolPolicy::default())
        .await;
    assert_eq!(resources.len(), 1);
    assert_eq!(
        resources[0].as_ref().unwrap().1[0].uri,
        "fixture://resource-a"
    );

    let templates = facade
        .list_resource_templates(Some("server-a"), &McpToolPolicy::default())
        .await;
    assert_eq!(
        templates[0].as_ref().unwrap().1[0].uri_template,
        "fixture://{id}"
    );

    let resource = facade
        .read_resource(
            "server-a",
            "fixture://resource-a",
            &McpToolPolicy::default(),
        )
        .await
        .unwrap();
    assert_eq!(resource.text.as_deref(), Some("fixture resource body"));
}

#[test]
fn parses_registry_config() {
    let config: McpRegistryConfig = serde_yaml::from_str(
        r#"
mcpServers:
  playwright:
    transport: stdio
    command: playwright-mcp
    args: ["--headless", "--isolated"]
    lifecycle: agent_session
    session_mode: stateful
    toolPrefix: browser_
"#,
    )
    .unwrap();
    let entry = config.mcp_servers.get("playwright").unwrap().clone();
    let definition = entry.into_definition("playwright".to_string()).unwrap();
    assert_eq!(definition.id, "playwright");
    assert_eq!(definition.lifecycle, McpLifecycleScope::AgentSession);
    assert_eq!(definition.tool_prefix.as_deref(), Some("browser_"));
}

#[test]
fn yaml_entry_honors_authored_concurrency_isolation_policy() {
    let config: McpRegistryConfig = serde_yaml::from_str(
        r#"
mcpServers:
  custom:
    transport: stdio
    command: some-mcp-bin
    args: []
    concurrencyIsolation:
      required_args: ["--single"]
      skip_if_any_arg_prefix: ["--data-dir"]
"#,
    )
    .unwrap();
    let entry = config.mcp_servers.get("custom").unwrap().clone();
    let definition = entry.into_definition("custom".into()).unwrap();
    match definition.transport {
        McpTransportConfig::Stdio { args, .. } => {
            assert!(args.iter().any(|a| a == "--single"));
        }
        _ => panic!("expected stdio"),
    }
    assert!(definition.concurrency_isolation.is_some());
}

#[test]
fn apply_concurrency_isolation_is_idempotent() {
    let policy = ConcurrencyIsolationPolicy {
        required_args: vec!["--isolated".into()],
        skip_if_any_arg_prefix: vec!["--user-data-dir".into(), "--isolated".into()],
    };
    let args = apply_concurrency_isolation(&policy, vec!["--headless".into()]);
    assert_eq!(args, vec!["--headless".to_string(), "--isolated".into()]);
    let again = apply_concurrency_isolation(&policy, args);
    assert_eq!(again, vec!["--headless".to_string(), "--isolated".into()]);
}

#[test]
fn apply_concurrency_isolation_skips_when_operator_overrode() {
    let policy = ConcurrencyIsolationPolicy {
        required_args: vec!["--isolated".into()],
        skip_if_any_arg_prefix: vec!["--user-data-dir".into()],
    };
    let args = apply_concurrency_isolation(&policy, vec!["--user-data-dir=/tmp/profile".into()]);
    assert_eq!(args, vec!["--user-data-dir=/tmp/profile".to_string()]);
}

#[test]
fn registry_config_redacts_into_app_source_definitions() {
    let config: McpRegistryConfig = serde_yaml::from_str(
        r#"
mcpServers:
  search:
    transport: streamable_http
    url: "http://127.0.0.1:9000/mcp"
    headers:
      Authorization: "Bearer secret"
"#,
    )
    .unwrap();
    let definitions = McpServerFactory::with_bundled_mapping_registry()
        .from_registry_config(config, McpDefinitionSource::App)
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].source, McpDefinitionSource::App);
    assert!(matches!(
        definitions[0].transport,
        McpTransportConfig::StreamableHttp { .. }
    ));
}

#[test]
fn skill_snapshot_imports_explicit_and_mapped_mcp_definitions() {
    let snapshot = SkillSnapshot {
        agent: "researcher".into(),
        prompt: String::new(),
        skills: vec![SkillSnapshotEntry {
            name: "playwright-mcp".into(),
            description: "Browser".into(),
            source_location: PathBuf::from("/tmp/playwright/SKILL.md"),
            source_base_dir: PathBuf::from("/tmp/playwright"),
            location: PathBuf::from("/tmp/playwright/SKILL.md"),
            base_dir: PathBuf::from("/tmp/playwright"),
            source: "test".into(),
            source_scope: SkillSourceScope::MacacaCentral,
            primary_env: None,
            required_env: Vec::new(),
            install: vec![SkillInstallSpec {
                kind: "npm".into(),
                package: Some("@playwright/mcp".into()),
                bins: vec!["playwright-mcp".into()],
                ..Default::default()
            }],
            mcp_servers: vec![SkillMcpServerConfig {
                id: "browser".into(),
                command: "playwright-mcp".into(),
                args: vec!["--headless".into()],
                transport: "stdio".into(),
                tool_prefix: None,
            }],
        }],
        filtered: Vec::new(),
        truncated: false,
        compact: false,
        version: 1,
    };

    let definitions =
        McpServerFactory::with_bundled_mapping_registry().from_skill_snapshot(&snapshot);
    assert_eq!(definitions.len(), 2);
    assert!(definitions
        .iter()
        .all(|definition| definition.lifecycle == McpLifecycleScope::AgentSession));
    assert!(definitions.iter().any(|definition| matches!(
        definition.transport,
        McpTransportConfig::Stdio { ref args, .. } if args.iter().any(|arg| arg == "--isolated")
    )));
}

#[test]
fn descriptor_from_tool_carries_sanitized_service_routing_metadata() {
    let mut definition = stdio_definition("server-a", "server-bin");
    definition.tool_prefix = Some("mcp_".into());
    let tool = macaca_framework::mcp::McpToolDef {
        name: "lookup".into(),
        description: "Lookup values".into(),
        input_schema: serde_json::json!({"type": "object"}),
    };

    let descriptor = descriptor_from_tool(&definition, tool, "mcp_lookup".into()).unwrap();

    assert_eq!(descriptor.service_id, MCP_SERVICE_ID);
    assert_eq!(descriptor.provider_id, "server-a");
    assert_eq!(descriptor.tool_name, "mcp_lookup");
    assert_eq!(
        descriptor.metadata.get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME),
        Some(&"lookup".to_string())
    );
    assert_eq!(
        descriptor.metadata.get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE),
        Some(&"agent_session".to_string())
    );
    assert!(
        !serde_json::to_string(&descriptor)
            .unwrap()
            .contains("server-bin"),
        "descriptors must not expose concrete commands or environment data"
    );
}

#[test]
fn policy_filters_servers_and_tools() {
    let mut deny_servers = HashSet::new();
    deny_servers.insert("blocked".to_string());
    let mut deny_tools = HashSet::new();
    deny_tools.insert("browser_install".to_string());
    let policy = McpToolPolicy {
        allow_servers: None,
        deny_servers,
        allow_tools: None,
        deny_tools,
    };
    assert!(!policy.allows_server("blocked"));
    assert!(policy.allows_server("playwright"));
    assert!(!policy.allows_tool("browser_install"));
    assert!(policy.allows_tool("browser_navigate"));
}
