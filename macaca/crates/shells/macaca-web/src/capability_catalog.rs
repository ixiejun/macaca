//! Host-side **Adapter** functions: bridge `macaca-skill`, MCP runtime probes, and the framework
//! `Toolkit` into neutral [`macaca_sdk::context::capability`] DTOs used by composer providers.
//!
//! The context crate intentionally avoids depending on skill/MCP crates; this module is the
//! application-facing composition root that performs the mapping while keeping transport and
//! discovery inside their respective runtimes.

use std::path::PathBuf;
use std::sync::Arc;

mod alias_resolution;
mod lifecycle_visibility;

use macaca_sdk::context::{
    mcp_tool_collisions, McpCapabilityCatalog, McpServerCapabilitySummary,
    RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
};
use macaca_sdk::framework::tool::Toolkit;
use macaca_proto::config::ContextConfig;
use macaca_proto::{
    ApplicationId, McpProbeCommand, McpRuntimeStatusView, McpToolPolicySnapshot, TraceContext,
    MCP_PROBE_COMMAND, MCP_SERVICE_ID,
};
use macaca_runtime_host::{
    McpRuntimeFacade, McpRuntimeStatus, McpRuntimeStatusState, McpToolPolicy,
};
use macaca_sdk::SystemMcpClient;
use macaca_sdk::skill::{
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillLifecycleState, SkillPolicy, SkillRuntimeFacade,
    SkillServiceScope, SkillSnapshot, SkillSnapshotRequest, SkillSnapshotServiceCommand,
};

use crate::state::AppState;

pub(crate) use lifecycle_visibility::SkillLifecycleVisibilitySettings;

/// Freeze a progressive-disclosure snapshot into immutable capability rows (Tier-1 only).
///
/// We intentionally reshape fields from [`SkillSnapshotEntry`] instead of reusing [`SkillSnapshot::prompt`]
/// so capability context guarantees **frontmatter/metadata only** regardless of snapshot prompt mode.
#[must_use]
pub fn skill_capability_catalog_from_snapshot(snapshot: &SkillSnapshot) -> SkillCapabilityCatalog {
    alias_resolution::catalog_from_snapshot_with_aliases(snapshot, &[])
}

/// Freeze a Skill service snapshot with a governance snapshot for normal
/// Context Composer visibility.
///
/// The Skill service owns lifecycle state and alias semantics.  This adapter
/// only maps those service-owned decisions into neutral context DTOs so
/// `macaca-context` can stay provider-neutral and body-free.
#[must_use]
#[cfg(test)]
pub fn skill_capability_catalog_from_governance_snapshot(
    snapshot: &SkillSnapshot,
    governance: &SkillGovernanceSnapshotResult,
) -> SkillCapabilityCatalog {
    skill_capability_catalog_from_governance_snapshot_with_visibility(
        snapshot,
        governance,
        &SkillLifecycleVisibilitySettings::default(),
    )
}

/// Freeze a Skill service snapshot with explicit lifecycle visibility settings.
///
/// The settings come from neutral Context provider configuration, not from
/// application-specific code.  This lets normal profiles keep active-only
/// catalogs while review/experimental profiles can opt into annotated
/// non-active rows through a traceable configuration surface.
#[must_use]
pub(crate) fn skill_capability_catalog_from_governance_snapshot_with_visibility(
    snapshot: &SkillSnapshot,
    governance: &SkillGovernanceSnapshotResult,
    visibility: &SkillLifecycleVisibilitySettings,
) -> SkillCapabilityCatalog {
    alias_resolution::catalog_from_snapshot_with_aliases_and_governance(
        snapshot,
        &[],
        Some(governance),
        visibility,
    )
}

/// Resolve the visibility settings used by skill capability catalog assembly.
#[must_use]
pub(crate) fn skill_lifecycle_visibility_from_context(
    context_config: &ContextConfig,
) -> SkillLifecycleVisibilitySettings {
    SkillLifecycleVisibilitySettings::from_context_config(context_config)
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

/// Build a capability catalog from service-owned MCP status views.
///
/// This Adapter keeps composer inputs provider-neutral while allowing the MCP
/// service to own lifecycle/state semantics behind `service.mcp` commands.
#[must_use]
pub fn mcp_capability_catalog_from_status_views(
    views: &[McpRuntimeStatusView],
) -> McpCapabilityCatalog {
    let servers: Vec<McpServerCapabilitySummary> = views
        .iter()
        .map(|view| McpServerCapabilitySummary {
            server_id: view.server_id.clone(),
            transport: view.transport.clone(),
            lifecycle: format!("{:?}", view.lifecycle),
            state: view.state.clone(),
            exposed_tools: view.exposed_tools.clone(),
        })
        .collect();
    let collisions = mcp_tool_collisions(&servers);
    McpCapabilityCatalog {
        servers,
        collisions,
    }
}

/// Return ready MCP server ids from service status views.
#[must_use]
pub fn ready_mcp_server_ids_from_views(views: &[McpRuntimeStatusView]) -> Vec<String> {
    views
        .iter()
        .filter(|view| view.state.eq_ignore_ascii_case("ready"))
        .map(|view| view.server_id.clone())
        .collect()
}

/// Probe MCP capability inputs through the serviceized MCP client.
///
/// Preferred Route C path for composer assembly: decorators, policy gates, and
/// audit evidence all flow through `service.mcp` instead of direct runtime
/// facade reads on shell-owned `AppState` fields.
pub async fn probe_mcp_capability_inputs_via_client(
    mcp_client: &Arc<dyn SystemMcpClient>,
    trace: TraceContext,
    agent_name: &str,
) -> (McpCapabilityCatalog, Vec<String>) {
    tracing::info!(
        trace_id = %trace.trace_id,
        agent = %agent_name,
        service_id = MCP_SERVICE_ID,
        command = MCP_PROBE_COMMAND,
        "probing MCP capability inputs through service client"
    );
    let command = match McpProbeCommand::new(trace, McpToolPolicySnapshot::default()) {
        Ok(command) => command,
        Err(error) => {
            tracing::warn!(
                agent = %agent_name,
                error = %error,
                "mcp probe command rejected; returning empty capability catalog"
            );
            return (McpCapabilityCatalog::default(), Vec::new());
        }
    };
    match mcp_client.probe(command).await {
        Ok(result) => {
            let ready = ready_mcp_server_ids_from_views(&result.statuses);
            let catalog = mcp_capability_catalog_from_status_views(&result.statuses);
            tracing::info!(
                agent = %agent_name,
                servers = catalog.servers.len(),
                ready_servers = ready.len(),
                "mcp capability probe completed through service client"
            );
            (catalog, ready)
        }
        Err(error) => {
            tracing::warn!(
                agent = %agent_name,
                error = %error,
                "mcp service probe failed; returning empty capability catalog"
            );
            (McpCapabilityCatalog::default(), Vec::new())
        }
    }
}

/// Async probe facade used when constructing long-lived [`crate::context_reporting_model::ContextReportingChatModel`] instances.
///
/// Deprecated migration surface: new call sites must use
/// [`probe_mcp_capability_inputs_via_client`] so MCP visibility flows through
/// the service boundary instead of `AppState::mcp_runtime`.
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
            let alias_scope =
                SkillServiceScope::agent(*app_id, session_id.unwrap_or("no-session"), agent_name)
                    .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
            let aliases = resolve_aliases_for_snapshot(state, alias_scope, &snapshot).await;
            let snapshot = alias_resolution::apply_to_snapshot(snapshot, &aliases);
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

async fn resolve_aliases_for_snapshot(
    state: &Arc<AppState>,
    scope: SkillServiceScope,
    snapshot: &SkillSnapshot,
) -> Vec<SkillAliasResolveResult> {
    let mut aliases = Vec::new();

    for entry in &snapshot.skills {
        let command = SkillAliasResolveCommand {
            trace: TraceContext::new("web-capability-skill-alias-resolve"),
            scope: scope.clone(),
            skill_id: alias_resolution::governed_skill_id(entry),
            name: Some(entry.name.clone()),
        };
        match state.skill_client.alias_resolve(command).await {
            Ok(result) => {
                if result.resolved {
                    tracing::info!(
                        requested_skill_id = %result.requested_skill_id,
                        requested_name = ?result.requested_name,
                        target_skill_id = ?result.target_skill_id,
                        target_name = ?result.target_name,
                        kind = alias_resolution::alias_kind_label(&result),
                        status = alias_resolution::alias_status_label(&result),
                        "skill alias hit during context catalog assembly"
                    );
                } else if matches!(
                    result.status,
                    macaca_sdk::skill::SkillAliasResolutionStatus::Denied
                        | macaca_sdk::skill::SkillAliasResolutionStatus::Expired
                        | macaca_sdk::skill::SkillAliasResolutionStatus::LoopPrevented
                ) {
                    tracing::warn!(
                        requested_skill_id = %result.requested_skill_id,
                        requested_name = ?result.requested_name,
                        status = alias_resolution::alias_status_label(&result),
                        "skill alias guarded decision during context catalog assembly"
                    );
                } else {
                    tracing::debug!(
                        requested_skill_id = %result.requested_skill_id,
                        requested_name = ?result.requested_name,
                        "skill alias miss during context catalog assembly"
                    );
                }
                aliases.push(result);
            }
            Err(error) => {
                tracing::warn!(
                    skill = %entry.name,
                    error = %error,
                    "skill alias resolution unavailable during context catalog assembly"
                );
            }
        }
    }

    aliases
}

/// Read the normal-profile governance snapshot used by capability context.
///
/// This requests every lifecycle explicitly instead of relying on legacy
/// `include_archived` semantics. The adapter then applies the normal-profile
/// `Active` visibility rule with enough service evidence to explain every
/// filtered row in the Context report.
pub async fn resolve_skill_governance_snapshot(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<&str>,
) -> Option<SkillGovernanceSnapshotResult> {
    let scope =
        match SkillServiceScope::agent(*app_id, session_id.unwrap_or("no-session"), agent_name) {
            Ok(scope) => scope,
            Err(error) => {
                tracing::warn!(
                    agent = %agent_name,
                    error = %error,
                    "skill governance scope validation failed during context catalog assembly"
                );
                return None;
            }
        };
    match state
        .skill_client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: TraceContext::new("web-capability-skill-governance-snapshot"),
            scope,
            include_archived: true,
            lifecycle_filters: vec![
                SkillLifecycleState::Draft,
                SkillLifecycleState::Active,
                SkillLifecycleState::Stale,
                SkillLifecycleState::Archived,
                SkillLifecycleState::Quarantined,
                SkillLifecycleState::Superseded,
                SkillLifecycleState::Rejected,
            ],
        })
        .await
    {
        Ok(snapshot) => {
            tracing::info!(
                agent = %agent_name,
                records = snapshot.records.len(),
                successful_tasks = snapshot.telemetry_aggregate.successful_task_count,
                failed_tasks = snapshot.telemetry_aggregate.failed_task_count,
                "skill governance snapshot consumed for context catalog assembly"
            );
            Some(snapshot)
        }
        Err(error) => {
            tracing::warn!(
                agent = %agent_name,
                error = %error,
                "skill governance snapshot unavailable during context catalog assembly"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "capability_catalog_tests.rs"]
mod tests;
