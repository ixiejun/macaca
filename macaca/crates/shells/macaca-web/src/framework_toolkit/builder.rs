//! Toolkit builder orchestrating service-backed tool contributors (Builder + Facade).
//!
//! `build_toolkit` is the single entry consumed by `FrameworkRunner`. It sequences
//! contributors in a deterministic order, logs key assembly nodes, and re-applies
//! manifest allowlists after late MCP/skill registrations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use macaca_sdk::framework::adapter::{SingleToolAdapter, ToolSetBridge};
use macaca_sdk::framework::tool::Toolkit;
use macaca_proto::{
    ApplicationId, McpServicePolicyHints, McpServiceScope, McpToolCatalogCommand, TraceContext,
};
use macaca_sdk::driver::{DriverServiceScope, DriverToolCatalogCommand};
use macaca_sdk::skill::{SkillServiceScope, SkillToolCatalogCommand};

use crate::state::AppState;
use macaca_runtime_host::{McpServerDefinition, McpToolPolicy};

use super::agent_tools::register_agent_tools;
use super::mcp_bridge::{
    emit_driver_catalog_unavailable, emit_mcp_runtime_events, emit_mcp_starting_events,
    emit_skill_mcp_alias_events, load_app_mcp_overlay_definitions,
    load_service_mcp_snapshot_definitions, register_mcp_definitions_with_service,
};
use super::policy::{app_agent_names, enforce_base_tool_allowlist, resolve_tool_policy};
use super::workspace_tools::{WorkspaceFileReadTool, WorkspaceFileWriteTool, WorkspaceShellTool};

/// Build a `Toolkit` with base tools + per-agent todo tools.
pub(crate) async fn build_toolkit(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    goal_id: Option<macaca_proto::TaskId>,
) -> Toolkit {
    let policy = resolve_tool_policy(state, app_id, agent_name).await;

    // Base tools from the global ToolSet via ToolSetBridge.
    // state.tools is Arc<dyn ToolCatalog>, which ToolSetBridge accepts directly.
    let mut toolkit = ToolSetBridge::from_tool_set(Arc::clone(&state.tools));

    // Dynamically aggregate driver tools through Driver Service.  Driver
    // discovery is intentionally fail-closed at the shell boundary: if the
    // focused service client is unavailable, Web records an auditable
    // diagnostic and continues without manufacturing tools from runtime
    // internals.  This keeps the Toolkit Builder as a Facade/Command consumer
    // rather than a second driver runtime owner.
    let driver_catalog = state
        .driver_client
        .tool_catalog(DriverToolCatalogCommand {
            trace: TraceContext::new("web-toolkit-driver-catalog"),
            scope: DriverServiceScope::session(
                *app_id,
                session_id.clone().unwrap_or_default(),
                agent_name,
            )
            .unwrap_or_default(),
            include_disabled: false,
        })
        .await;
    match driver_catalog {
        Ok(catalog) => {
            for descriptor in catalog.tools {
                if let Some(tool) = crate::service_tool_adapter::service_tool_from_descriptor(
                    descriptor,
                    state,
                    *app_id,
                    session_id.clone(),
                    agent_name,
                ) {
                    toolkit.register(Box::new(SingleToolAdapter::new(tool)), None);
                }
            }
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                app_id = %app_id,
                session_id = session_id.as_deref().unwrap_or_default(),
                agent = agent_name,
                "Driver Service catalog unavailable during toolkit assembly"
            );
            emit_driver_catalog_unavailable(
                state,
                session_id.as_deref(),
                agent_name,
                error.to_string(),
            )
            .await;
        }
    }

    let skill_catalog = state
        .skill_client
        .tool_catalog(SkillToolCatalogCommand {
            trace: TraceContext::new("web-toolkit-skill-catalog"),
            scope: SkillServiceScope::agent(
                *app_id,
                session_id.clone().unwrap_or_default(),
                agent_name,
            )
            .unwrap_or_default(),
            include_disabled: false,
        })
        .await;
    if let Ok(catalog) = skill_catalog {
        for descriptor in catalog.tools {
            if let Some(tool) = crate::service_tool_adapter::service_tool_from_descriptor(
                descriptor,
                state,
                *app_id,
                session_id.clone(),
                agent_name,
            ) {
                toolkit.register(Box::new(SingleToolAdapter::new(tool)), None);
            }
        }
    }

    enforce_base_tool_allowlist(&mut toolkit, policy.base_allowed_tools.as_ref());

    for tool_name in ["file_read", "file_write", "shell"] {
        toolkit.unregister(tool_name);
    }
    if let Some(ws) = state
        .config
        .app_workspaces
        .read()
        .await
        .get(app_id)
        .cloned()
    {
        if policy.allows_base_tool("file_read") {
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(WorkspaceFileReadTool {
                    workspace_root: ws.root.clone(),
                }))),
                None,
            );
        }
        if policy.allows_base_tool("file_write") {
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(WorkspaceFileWriteTool {
                    workspace_root: ws.root.clone(),
                }))),
                None,
            );
        }
        if policy.allows_base_tool("shell") {
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(WorkspaceShellTool {
                    workspace_root: ws.root,
                    default_timeout: Duration::from_secs(30),
                }))),
                None,
            );
        }
    }

    let limit = state
        .config
        .context
        .recall
        .memory_search_default_limit
        .clamp(1, 32);
    let scope = crate::context_memory_tools::workspace_tool_scope(*app_id);
    if policy.allows_base_tool("memory_search") {
        toolkit.register(
            Box::new(SingleToolAdapter::new(Box::new(
                crate::context_memory_tools::ServiceWorkspaceMemorySearchTool {
                    client: Arc::clone(&state.memory_client),
                    scope: scope.clone(),
                    default_limit: limit,
                    session_id: session_id.clone(),
                    agent_name: agent_name.to_string(),
                },
            ))),
            None,
        );
    }
    if policy.allows_base_tool("memory_get") {
        toolkit.register(
            Box::new(SingleToolAdapter::new(Box::new(
                crate::context_memory_tools::ServiceWorkspaceMemoryGetTool {
                    client: Arc::clone(&state.memory_client),
                    scope: scope.clone(),
                    session_id: session_id.clone(),
                    agent_name: agent_name.to_string(),
                },
            ))),
            None,
        );
    }
    if policy.allows_base_tool("memory_forget") {
        if let Some(ts) = state.workspace_memory_tombstones.as_ref() {
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    crate::context_memory_tools::ServiceWorkspaceMemoryForgetTool {
                        client: Arc::clone(&state.memory_client),
                        scope: scope.clone(),
                        tombstones: Arc::clone(ts),
                        session_id: session_id.clone(),
                        agent_name: agent_name.to_string(),
                    },
                ))),
                None,
            );
        }
    }

    // Scheduled-agent-task creation is exposed as a thin Adapter over the
    // focused SDK client. The Web shell contributes only argument projection
    // into a typed Command; the Scheduled Agent Task service remains the owner
    // of prompt persistence, Scheduler registration, policy, audit, and
    // structured unavailable behavior.
    if policy.can_create_scheduled_agent_tasks
        && policy.allows_base_tool(
            crate::scheduled_agent_task_tool::SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME,
        )
    {
        toolkit.register(
            Box::new(SingleToolAdapter::new(Box::new(
                crate::scheduled_agent_task_tool::ScheduledAgentTaskCreateTool::new(
                    Arc::clone(&state.scheduled_agent_task_client),
                    *app_id,
                    agent_name,
                ),
            ))),
            Some("autonomy"),
        );
        tracing::info!(
            app_id = %app_id,
            agent = agent_name,
            tool = crate::scheduled_agent_task_tool::SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME,
            "registered service-backed scheduled agent task tool"
        );
    }

    let app_agent_names = app_agent_names(state, app_id).await;
    let assignee_capabilities: HashMap<String, Vec<String>> = state
        .kernel
        .list_agents()
        .await
        .into_iter()
        .filter(|m| {
            app_agent_names
                .as_ref()
                .is_none_or(|names| names.contains(&m.name))
        })
        .map(|m| {
            let profile = m
                .capabilities
                .into_iter()
                .map(|c| format!("{} {}", c.name, c.description))
                .collect::<Vec<_>>();
            (m.name, profile)
        })
        .collect();

    // Register per-agent todo tools.
    register_agent_tools(
        &mut toolkit,
        state,
        app_id,
        agent_name,
        session_id.clone(),
        goal_id,
        &policy,
        &assignee_capabilities,
    );

    let mcp_policy = McpToolPolicy::default();
    let mut mcp_definitions =
        load_service_mcp_snapshot_definitions(state, app_id, session_id.as_deref(), agent_name)
            .await;
    mcp_definitions.extend(load_app_mcp_overlay_definitions(state, app_id).await);
    emit_mcp_starting_events(state, session_id.as_deref(), agent_name, &mcp_definitions).await;
    register_mcp_definitions_with_service(
        state,
        app_id,
        session_id.as_deref(),
        agent_name,
        &mcp_definitions,
    )
    .await;
    let mcp_statuses =
        macaca_runtime_host::probe_definition_statuses(mcp_definitions.clone(), &mcp_policy).await;
    emit_mcp_runtime_events(state, session_id.as_deref(), agent_name, &mcp_statuses).await;

    if let Some(snapshot) = crate::skill_mcp::load_or_build_skill_snapshot(
        state,
        app_id,
        agent_name,
        session_id.as_deref(),
    )
    .await
    {
        let skill_definitions = macaca_runtime_host::McpServerFactory::with_bundled_mapping_registry()
            .from_skill_snapshot(&snapshot);
        emit_mcp_starting_events(state, session_id.as_deref(), agent_name, &skill_definitions)
            .await;
        register_mcp_definitions_with_service(
            state,
            app_id,
            session_id.as_deref(),
            agent_name,
            &skill_definitions,
        )
        .await;
        let skill_statuses =
            macaca_runtime_host::probe_definition_statuses(skill_definitions, &mcp_policy).await;
        emit_skill_mcp_alias_events(state, session_id.as_deref(), agent_name, &skill_statuses)
            .await;
        emit_mcp_runtime_events(state, session_id.as_deref(), agent_name, &skill_statuses).await;
    }

    let mcp_catalog = state
        .mcp_client
        .tool_catalog(McpToolCatalogCommand {
            trace: TraceContext::new("web-toolkit-mcp-catalog"),
            scope: McpServiceScope::agent_session(
                *app_id,
                session_id.clone().unwrap_or_default(),
                agent_name,
            )
            .unwrap_or_default(),
            policy: McpServicePolicyHints::default(),
        })
        .await;
    match mcp_catalog {
        Ok(catalog) => {
            for descriptor in catalog.tools {
                if let Some(tool) = crate::service_tool_adapter::service_tool_from_descriptor(
                    descriptor,
                    state,
                    *app_id,
                    session_id.clone(),
                    agent_name,
                ) {
                    toolkit.register(Box::new(SingleToolAdapter::new(tool)), None);
                }
            }
        }
        Err(error) => tracing::warn!(
            error = %error,
            "MCP Service catalog failed; no direct production MCP toolkit fallback will be used"
        ),
    }

    // Tool contributors run in phases: base tools, driver/skill service tools,
    // workspace tools, autonomy tools, and finally MCP descriptors.  Some
    // contributors intentionally execute late because they depend on
    // service-owned catalogs.  Re-applying the manifest allowlist here makes the
    // final Toolkit state match application policy instead of trusting every
    // contributor to remember the same filtering order.
    enforce_base_tool_allowlist(&mut toolkit, policy.base_allowed_tools.as_ref());

    toolkit
}
