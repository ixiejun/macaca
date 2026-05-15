//! Host-side **Adapter** functions: bridge `macaca-skill`, MCP runtime probes, and the framework
//! `Toolkit` into neutral [`macaca_context::capability`] DTOs used by composer providers.
//!
//! The context crate intentionally avoids depending on skill/MCP crates; this module is the
//! application-facing composition root that performs the mapping while keeping transport and
//! discovery inside their respective runtimes.

use std::path::PathBuf;
use std::sync::Arc;

use macaca_context::{
    mcp_tool_collisions, DeclaredCapabilityDependency, McpCapabilityCatalog,
    McpServerCapabilitySummary, RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
    SkillCapabilityRecord, SkillFilterDiagnostic,
};
use macaca_framework::tool::Toolkit;
use macaca_proto::{ApplicationId, TraceContext};
use macaca_runtime_host::{
    McpRuntimeFacade, McpRuntimeStatus, McpRuntimeStatusState, McpToolPolicy,
};
use macaca_skill::{
    SkillPolicy, SkillRuntimeFacade, SkillServiceScope, SkillSnapshot, SkillSnapshotRequest,
    SkillSnapshotServiceCommand,
};

use crate::state::AppState;

/// Freeze a progressive-disclosure snapshot into immutable capability rows (Tier-1 only).
///
/// We intentionally reshape fields from [`SkillSnapshotEntry`] instead of reusing [`SkillSnapshot::prompt`]
/// so capability context guarantees **frontmatter/metadata only** regardless of snapshot prompt mode.
#[must_use]
pub fn skill_capability_catalog_from_snapshot(snapshot: &SkillSnapshot) -> SkillCapabilityCatalog {
    let entries: Vec<SkillCapabilityRecord> = snapshot
        .skills
        .iter()
        .map(|e| {
            let stable_id = format!("skill:{}:{}", e.source_scope.as_str(), e.name);
            let declared_dependencies: Vec<DeclaredCapabilityDependency> = e
                .mcp_servers
                .iter()
                .map(|m| DeclaredCapabilityDependency {
                    capability_id: format!("mcp_server:{}", m.id),
                    notes: Some(format!(
                        "declared in SKILL.md manifest (transport={})",
                        m.transport
                    )),
                })
                .collect();
            SkillCapabilityRecord {
                stable_id,
                name: e.name.clone(),
                description: e.description.clone(),
                location_ref: e.location.display().to_string(),
                source_label: e.source.clone(),
                source_scope: e.source_scope.as_str().to_string(),
                declared_dependencies,
            }
        })
        .collect();

    let filtered: Vec<SkillFilterDiagnostic> = snapshot
        .filtered
        .iter()
        .map(|f| SkillFilterDiagnostic {
            name: f.name.clone(),
            reason: f.reason.clone(),
        })
        .collect();

    SkillCapabilityCatalog {
        entries,
        filtered,
        truncated: snapshot.truncated,
    }
}

#[must_use]
pub fn mcp_capability_catalog_from_statuses(statuses: &[McpRuntimeStatus]) -> McpCapabilityCatalog {
    let servers: Vec<McpServerCapabilitySummary> = statuses
        .iter()
        .map(|s| McpServerCapabilitySummary {
            server_id: s.server_id.clone(),
            transport: s.transport.clone(),
            lifecycle: format!("{:?}", s.lifecycle),
            state: format!("{:?}", s.state),
            exposed_tools: s.exposed_tools.clone(),
        })
        .collect();
    let collisions = mcp_tool_collisions(&servers);
    McpCapabilityCatalog {
        servers,
        collisions,
    }
}

/// Collects tool **names** only from JSON tool definitions (compact router index).
#[must_use]
pub fn runtime_tool_capability_catalog_from_toolkit(
    toolkit: &Toolkit,
) -> RuntimeToolCapabilityCatalog {
    let mut tool_names: Vec<String> = toolkit
        .get_definitions()
        .iter()
        .filter_map(|def| {
            def.get("name")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string)
        })
        .collect();
    tool_names.sort();
    tool_names.dedup();
    RuntimeToolCapabilityCatalog { tool_names }
}

#[must_use]
pub fn ready_mcp_server_ids(statuses: &[McpRuntimeStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter(|s| matches!(s.state, McpRuntimeStatusState::Ready))
        .map(|s| s.server_id.clone())
        .collect()
}

/// Async probe facade used when constructing long-lived [`crate::context_reporting_model::ContextReportingChatModel`] instances.
pub async fn probe_mcp_capability_inputs(
    facade: &Arc<McpRuntimeFacade>,
    policy: &McpToolPolicy,
) -> (McpCapabilityCatalog, Vec<String>) {
    let statuses = facade.probe(policy).await;
    let ready = ready_mcp_server_ids(&statuses);
    let catalog = mcp_capability_catalog_from_statuses(&statuses);
    (catalog, ready)
}

/// Load or rebuild the skill snapshot with the same semantics as workspace system-prompt wiring.
///
/// Shared between static system prompt telemetry and composer capability providers so in-session
/// cache keys stay consistent (`skill_snapshot/{agent}` module).
pub async fn resolve_skill_snapshot_cached(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
    skill_policy: SkillPolicy,
    workspace_root: Option<PathBuf>,
    app_dir: Option<PathBuf>,
) -> macaca_proto::MacacaResult<SkillSnapshot> {
    let snapshot_module = format!("skill_snapshot/{agent_name}");
    let loaded_snapshot = if let Some(session_id) = session_id {
        state
            .sessions
            .framework_session_store
            .load(session_id, &snapshot_module)
            .await
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_value::<SkillSnapshot>(value).ok())
    } else {
        None
    };

    match loaded_snapshot {
        Some(snapshot) => Ok(snapshot),
        None => {
            let request = SkillSnapshotRequest::builder(agent_name)
                .workspace_dir(workspace_root.clone())
                .app_dir(app_dir.clone())
                .policy(skill_policy.clone())
                .build();
            let snapshot = match state
                .skill_client
                .snapshot(SkillSnapshotServiceCommand {
                    trace: TraceContext::new("web-capability-skill-snapshot"),
                    scope: SkillServiceScope::agent(
                        *app_id,
                        session_id.unwrap_or("no-session"),
                        agent_name,
                    )
                    .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?,
                    agent_name: agent_name.to_string(),
                    workspace_dir: workspace_root,
                    app_dir: app_dir.clone(),
                    include_instructions: true,
                    // Preserve the application/agent exposure policy across
                    // the serviceized skill boundary. The deprecated facade
                    // fallback below already receives this policy through the
                    // request builder; omitting it here made the primary
                    // service path report an empty visible catalog for agents
                    // that allowlist skills in app.yaml.
                    exposure_policy: skill_policy,
                    policy: Default::default(),
                })
                .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        agent = %agent_name,
                        "Skill Service capability snapshot failed; using deprecated facade fallback"
                    );
                    #[allow(deprecated)]
                    SkillRuntimeFacade::new().build_snapshot(request).await?
                }
            };
            if let Some(session_id) = session_id {
                if let Ok(value) = serde_json::to_value(&snapshot) {
                    let _ = state
                        .sessions
                        .framework_session_store
                        .save(session_id, &snapshot_module, value)
                        .await;
                }
            }
            Ok(snapshot)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use async_trait::async_trait;
    use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
    use macaca_framework::tool::{ToolError, ToolHandler, ToolResponse};
    use macaca_runtime_host::{
        McpDefinitionSource, McpLifecycleScope, McpRuntimeFacade, McpRuntimeStatus,
        McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
    };
    use macaca_skill::{SkillMcpServerConfig, SkillSnapshot, SkillSnapshotEntry, SkillSourceScope};
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
    fn skill_capability_catalog_derives_stable_ids_and_mcp_dependencies() {
        let snapshot = SkillSnapshot {
            agent: "architect".into(),
            prompt: "".into(),
            skills: vec![SkillSnapshotEntry {
                name: "figma-mcp".into(),
                description: "Figma context".into(),
                source_location: PathBuf::from("/app/skills/figma-mcp/SKILL.md"),
                source_base_dir: PathBuf::from("/app/skills/figma-mcp"),
                location: PathBuf::from("/app/skills/figma-mcp/SKILL.md"),
                base_dir: PathBuf::from("/app/skills/figma-mcp"),
                source: "app".into(),
                source_scope: SkillSourceScope::Application,
                primary_env: None,
                required_env: vec![],
                install: vec![],
                mcp_servers: vec![SkillMcpServerConfig {
                    id: "figma".into(),
                    command: "npx".into(),
                    args: vec![],
                    transport: "stdio".into(),
                    tool_prefix: None,
                }],
            }],
            filtered: vec![],
            truncated: false,
            compact: true,
            version: 1,
        };

        let cat = skill_capability_catalog_from_snapshot(&snapshot);
        assert_eq!(cat.entries.len(), 1);
        let row = &cat.entries[0];
        assert_eq!(row.stable_id, "skill:application:figma-mcp");
        assert_eq!(row.declared_dependencies.len(), 1);
        assert_eq!(
            row.declared_dependencies[0].capability_id,
            "mcp_server:figma"
        );
        assert!(row.declared_dependencies[0]
            .notes
            .as_ref()
            .expect("transport note")
            .contains("stdio"));
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

        let (catalog, ready) =
            probe_mcp_capability_inputs(&facade, &McpToolPolicy::default()).await;

        assert!(ready.is_empty());
        assert_eq!(catalog.servers.len(), 1);
        assert_eq!(catalog.servers[0].server_id, "fixture_disabled_stdio");
        assert!(catalog.servers[0].state.contains("Disabled"));
        assert!(
            catalog.servers[0].exposed_tools.is_empty(),
            "disabled servers should surface no MCP tools until enabled"
        );
    }
}
