use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use macaca_host_composition::framework::mcp::{McpSessionMode, McpTransportConfig};
use macaca_host_composition::framework::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};
use macaca_host_composition::mcp_runtime::{
    McpDefinitionSource, McpLifecycleScope, McpRuntimeFacade, McpRuntimeStatus,
    McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};
use macaca_host_composition::runtime_host::{
    SkillAuthorKind, SkillGovernanceProvenance, SkillGovernanceRecord,
    SkillGovernanceSnapshotResult, SkillLifecycleState, SkillSnapshot, SkillSnapshotEntry,
    SkillSourceScope, SkillUsageTelemetry,
};
use macaca_proto::config::{ContextConfig, ContextProviderFamilyConfig};
use macaca_proto::{
    MacacaError, MacacaResult, McpCleanupCommand, McpDiagnosticsSnapshot,
    McpDiagnosticsSnapshotCommand, McpExposureRefreshCommand, McpExposureRefreshResult,
    McpOAuthLoginCommand, McpOAuthStatusCommand, McpOAuthStatusResult, McpProbeCommand,
    McpRegisterCommand, McpRegisterResult, McpReloadCommand, McpReloadResult,
    McpResourceListCommand, McpResourceListResult, McpResourceReadCommand, McpResourceReadResult,
    McpResourceTemplateListResult, McpRuntimeStatusView, McpServerStatusListCommand,
    McpServerStatusListResult, McpServiceLifecycleScope, McpServiceSnapshot,
    McpServiceSnapshotCommand, McpStatusCommand, McpStatusResult, McpToolAttachCommand,
    McpToolAttachResult, McpToolCatalogCommand, McpToolCatalogResult, McpToolInvokeCommand,
    McpToolInvokeResult, TraceContext,
};
use macaca_sdk::SystemMcpClient;
use serde_json::Value;

use super::*;

fn sample_mcp_status(
    server_id: &str,
    state: McpRuntimeStatusState,
    exposed_tools: Vec<String>,
) -> McpRuntimeStatus {
    McpRuntimeStatus {
        server_id: server_id.to_string(),
        transport: "stdio".into(),
        lifecycle: McpLifecycleScope::Global,
        session_mode: McpSessionMode::Stateful,
        state,
        exposed_tools,
        failure_reason: None,
    }
}

#[test]
fn mcp_catalog_maps_statuses_and_detects_cross_server_tool_collision() {
    let statuses = [
        sample_mcp_status(
            "figma",
            McpRuntimeStatusState::Ready,
            vec!["get_figma_data".into(), "shared_tool".into()],
        ),
        sample_mcp_status(
            "other",
            McpRuntimeStatusState::Ready,
            vec!["shared_tool".into()],
        ),
    ];

    assert_eq!(
        ready_mcp_server_ids(&statuses),
        vec!["figma".to_string(), "other".to_string()]
    );

    let cat = mcp_capability_catalog_from_statuses(&statuses);
    assert_eq!(cat.servers.len(), 2);
    assert_eq!(cat.collisions.len(), 1);
    assert_eq!(cat.collisions[0].local_key, "shared_tool");
    assert!(cat.collisions[0].claimants.contains(&"figma".to_string()));
}

#[test]
fn ready_mcp_server_ids_ignores_non_ready() {
    let statuses = [
        sample_mcp_status("up", McpRuntimeStatusState::Ready, vec![]),
        sample_mcp_status("down", McpRuntimeStatusState::Failed, vec![]),
    ];
    assert_eq!(ready_mcp_server_ids(&statuses), vec!["up"]);
}

#[test]
fn runtime_tool_catalog_lists_tool_names_sorted_and_deduped() {
    struct NamedTool(&'static str);

    #[async_trait]
    impl ToolHandler for NamedTool {
        async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::json(args))
        }
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "test tool"
        }
        fn schema(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }
    }

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(NamedTool("zzz")), None);
    toolkit.register(Box::new(NamedTool("aaa")), None);
    toolkit.register(Box::new(NamedTool("zzz")), None);

    let cat = runtime_tool_capability_catalog_from_toolkit(&toolkit);
    assert_eq!(cat.tool_names, vec!["aaa", "zzz"]);
}

/// Fixture MCP client that returns deterministic probe statuses for protocol tests.
struct FixtureMcpProbeClient {
    statuses: Vec<McpRuntimeStatusView>,
}

#[async_trait]
impl SystemMcpClient for FixtureMcpProbeClient {
    async fn register(&self, _command: McpRegisterCommand) -> MacacaResult<McpRegisterResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn probe(&self, _command: McpProbeCommand) -> MacacaResult<McpStatusResult> {
        Ok(McpStatusResult::new(self.statuses.clone()))
    }

    async fn tool_catalog(
        &self,
        _command: McpToolCatalogCommand,
    ) -> MacacaResult<McpToolCatalogResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn attach_tools(
        &self,
        _command: McpToolAttachCommand,
    ) -> MacacaResult<McpToolAttachResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn invoke_tool(
        &self,
        _command: McpToolInvokeCommand,
    ) -> MacacaResult<McpToolInvokeResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn status(&self, _command: McpStatusCommand) -> MacacaResult<McpStatusResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn server_status_list(
        &self,
        _command: McpServerStatusListCommand,
    ) -> MacacaResult<McpServerStatusListResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn reload(&self, _command: McpReloadCommand) -> MacacaResult<McpReloadResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn oauth_login(
        &self,
        _command: McpOAuthLoginCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn oauth_status(
        &self,
        _command: McpOAuthStatusCommand,
    ) -> MacacaResult<McpOAuthStatusResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn resource_list(
        &self,
        _command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceListResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn resource_template_list(
        &self,
        _command: McpResourceListCommand,
    ) -> MacacaResult<McpResourceTemplateListResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn resource_read(
        &self,
        _command: McpResourceReadCommand,
    ) -> MacacaResult<McpResourceReadResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn diagnostics_snapshot(
        &self,
        _command: McpDiagnosticsSnapshotCommand,
    ) -> MacacaResult<McpDiagnosticsSnapshot> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn exposure_refresh(
        &self,
        _command: McpExposureRefreshCommand,
    ) -> MacacaResult<McpExposureRefreshResult> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn snapshot(
        &self,
        _command: McpServiceSnapshotCommand,
    ) -> MacacaResult<McpServiceSnapshot> {
        Err(MacacaError::Config("fixture client".into()))
    }

    async fn cleanup(&self, _command: McpCleanupCommand) -> MacacaResult<McpStatusResult> {
        Err(MacacaError::Config("fixture client".into()))
    }
}

#[tokio::test]
async fn probe_mcp_capability_inputs_via_client_maps_service_status_views() {
    let client: Arc<dyn SystemMcpClient> = Arc::new(FixtureMcpProbeClient {
        statuses: vec![
            McpRuntimeStatusView {
                server_id: "fixture_ready".into(),
                transport: "stdio".into(),
                lifecycle: McpServiceLifecycleScope::Session,
                session_mode: "stateful".into(),
                state: "Ready".into(),
                exposed_tools: vec!["tool_a".into()],
                failure_reason: None,
            },
            McpRuntimeStatusView {
                server_id: "fixture_disabled".into(),
                transport: "stdio".into(),
                lifecycle: McpServiceLifecycleScope::Session,
                session_mode: "stateful".into(),
                state: "Disabled".into(),
                exposed_tools: Vec::new(),
                failure_reason: None,
            },
        ],
    });

    let (catalog, ready) = probe_mcp_capability_inputs_via_client(
        &client,
        TraceContext::new("fixture-mcp-probe"),
        "agent-a",
    )
    .await;

    assert_eq!(ready, vec!["fixture_ready".to_string()]);
    assert_eq!(catalog.servers.len(), 2);
    assert_eq!(catalog.servers[0].server_id, "fixture_ready");
    assert_eq!(catalog.servers[0].exposed_tools, vec!["tool_a".to_string()]);
    assert_eq!(catalog.servers[1].server_id, "fixture_disabled");
    assert!(catalog.servers[1].exposed_tools.is_empty());
}

#[tokio::test]
async fn probe_mcp_capability_inputs_matches_facade_probe_without_subprocess_enabled() {
    let facade = Arc::new(McpRuntimeFacade::new());
    let def = McpServerDefinition {
        id: "fixture_disabled_stdio".into(),
        transport: McpTransportConfig::Stdio {
            command: "no-such-binary-for-probe-fixture".into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
        },
        lifecycle: McpLifecycleScope::Session,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: None,
        required_bins: vec!["no-such-binary-for-probe-fixture".into()],
        enabled: false,
        source: McpDefinitionSource::App,
        concurrency_isolation: None,
    };
    facade.upsert_definition(def).await;

    let (catalog, ready) = probe_mcp_capability_inputs(&facade, &McpToolPolicy::default()).await;

    assert!(ready.is_empty());
    assert_eq!(catalog.servers.len(), 1);
    assert_eq!(catalog.servers[0].server_id, "fixture_disabled_stdio");
    assert!(catalog.servers[0].state.contains("Disabled"));
    assert!(
        catalog.servers[0].exposed_tools.is_empty(),
        "disabled servers should surface no MCP tools until enabled"
    );
}

fn skill_snapshot(names: &[&str]) -> SkillSnapshot {
    SkillSnapshot {
        agent: "fixture-catalog-agent".into(),
        prompt: "".into(),
        skills: names
            .iter()
            .map(|name| SkillSnapshotEntry {
                name: (*name).into(),
                description: format!("{name} skill"),
                source_location: PathBuf::from(format!("/skills/{name}/SKILL.md")),
                source_base_dir: PathBuf::from(format!("/skills/{name}")),
                location: PathBuf::from(format!("/skills/{name}/SKILL.md")),
                base_dir: PathBuf::from(format!("/skills/{name}")),
                source: "app".into(),
                source_scope: SkillSourceScope::Application,
                primary_env: None,
                required_env: vec![],
                install: vec![],
                mcp_servers: vec![],
            })
            .collect(),
        filtered: vec![],
        truncated: false,
        compact: true,
        version: 1,
    }
}

fn governance_record(
    name: &str,
    lifecycle: SkillLifecycleState,
    activations: u64,
) -> SkillGovernanceRecord {
    let mut telemetry = SkillUsageTelemetry::default();
    telemetry.activation_count = activations;
    SkillGovernanceRecord {
        provenance: SkillGovernanceProvenance::new(
            format!("skill://application/{name}"),
            name,
            "app",
            "application",
            SkillAuthorKind::Agent,
        ),
        lifecycle,
        pinned: false,
        telemetry,
        diagnostics: Default::default(),
        updated_at: Utc::now(),
        evidence_ids: vec![format!("evidence://governance/{name}")],
    }
}

#[test]
fn skill_catalog_filters_non_active_lifecycle_from_governance_snapshot() {
    let snapshot = skill_snapshot(&["active", "draft", "rejected"]);
    let governance = SkillGovernanceSnapshotResult {
        records: vec![
            governance_record("active", SkillLifecycleState::Active, 2),
            governance_record("draft", SkillLifecycleState::Draft, 1),
            governance_record("rejected", SkillLifecycleState::Rejected, 0),
        ],
        telemetry_aggregate: Default::default(),
        captured_at: Utc::now(),
    };

    let catalog = skill_capability_catalog_from_governance_snapshot(&snapshot, &governance);

    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].name, "active");
    assert_eq!(catalog.entries[0].lifecycle.as_deref(), Some("active"));
    assert_eq!(catalog.governance_report.visible_count, 1);
    assert_eq!(catalog.governance_report.lifecycle_filtered_count, 2);
    assert_eq!(catalog.governance_report.activation_observation_count, 3);
    assert!(catalog
        .filtered
        .iter()
        .any(|row| row.reason.starts_with("lifecycle_filtered:draft")));
    assert!(catalog
        .governance_report
        .trace_refs
        .iter()
        .any(|row| row.contains("active")));
}

#[test]
fn skill_catalog_review_profile_can_annotate_non_active_rows_but_not_drafts() {
    let snapshot = skill_snapshot(&["active", "stale", "draft"]);
    let governance = SkillGovernanceSnapshotResult {
        records: vec![
            governance_record("active", SkillLifecycleState::Active, 0),
            governance_record("stale", SkillLifecycleState::Stale, 0),
            governance_record("draft", SkillLifecycleState::Draft, 0),
        ],
        telemetry_aggregate: Default::default(),
        captured_at: Utc::now(),
    };
    let mut context = ContextConfig::default();
    context.provider_families = vec![ContextProviderFamilyConfig {
        family_id: "skill_capability".into(),
        enabled: true,
        params: serde_json::json!({
            "lifecycle_profile": "review",
            "visible_lifecycles": ["active", "stale", "draft"]
        }),
    }];
    let visibility = skill_lifecycle_visibility_from_context(&context);

    let catalog = skill_capability_catalog_from_governance_snapshot_with_visibility(
        &snapshot,
        &governance,
        &visibility,
    );

    let names = catalog
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["active", "stale"]);
    assert_eq!(catalog.entries[1].lifecycle.as_deref(), Some("stale"));
    assert!(catalog
        .filtered
        .iter()
        .any(|row| row.reason.starts_with("lifecycle_filtered:draft")));
}

#[test]
fn skill_catalog_experimental_profile_is_required_for_draft_visibility() {
    let snapshot = skill_snapshot(&["active", "draft"]);
    let governance = SkillGovernanceSnapshotResult {
        records: vec![
            governance_record("active", SkillLifecycleState::Active, 0),
            governance_record("draft", SkillLifecycleState::Draft, 0),
        ],
        telemetry_aggregate: Default::default(),
        captured_at: Utc::now(),
    };
    let mut context = ContextConfig::default();
    context.provider_families = vec![ContextProviderFamilyConfig {
        family_id: "skill_capability".into(),
        enabled: true,
        params: serde_json::json!({
            "lifecycle_profile": "experimental",
            "visible_lifecycles": ["active", "draft"]
        }),
    }];
    let visibility = skill_lifecycle_visibility_from_context(&context);

    let catalog = skill_capability_catalog_from_governance_snapshot_with_visibility(
        &snapshot,
        &governance,
        &visibility,
    );

    let draft = catalog
        .entries
        .iter()
        .find(|entry| entry.name == "draft")
        .expect("experimental profile should expose annotated draft row");
    assert_eq!(draft.lifecycle.as_deref(), Some("draft"));
    assert!(catalog.filtered.is_empty());
}
