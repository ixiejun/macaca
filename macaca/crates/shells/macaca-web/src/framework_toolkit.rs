//! Toolkit construction and tool policy for framework-based agents.
//!
//! This module keeps `FrameworkRunner` focused on agent construction while
//! preserving the existing toolkit registration behavior.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::{fs, process::Command, time::timeout};

use macaca_app::{app_agent_manifest_view, discovered_app_agent_names};
use macaca_driver::{DriverServiceScope, DriverToolCatalogCommand};
use macaca_framework::adapter::{SingleToolAdapter, ToolSetBridge};
use macaca_framework::execution::ExecutionContext;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_framework::tool::Toolkit;
use macaca_proto::{
    ApplicationId, McpRegisterCommand, McpServicePolicyHints, McpServiceScope,
    McpToolCatalogCommand, TraceContext,
};
use macaca_skill::{SkillServiceScope, SkillToolCatalogCommand};

use crate::runtime_event_bridge::emit_runtime_event;
use crate::state::AppState;
use macaca_runtime_host::{
    McpDefinitionSource, McpRegistryConfig, McpRuntimeStatus, McpRuntimeStatusState,
    McpServerDefinition, McpToolPolicy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoToolPolicy {
    GoalManager,
    Planner,
    Worker,
}

#[derive(Debug, Clone)]
struct AgentToolPolicy {
    base_allowed_tools: Option<HashSet<String>>,
    todo_policy: TodoToolPolicy,
    disallowed_task_assignees: HashSet<String>,
}

impl AgentToolPolicy {
    fn allows_base_tool(&self, tool_name: &str) -> bool {
        self.base_allowed_tools
            .as_ref()
            .map(|allowlist| allowlist.contains(tool_name))
            .unwrap_or(true)
    }
}

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

    // Dynamically aggregate driver tools through Driver Service.  The direct
    // runtime path remains as a deprecated fallback so S6 can be rolled back
    // without changing user-visible tool availability.
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
                "Driver Service catalog failed; using deprecated direct driver runtime fallback"
            );
            #[allow(deprecated)]
            let driver_tools = state.driver_runtime.collect_tools().await;
            for tool in driver_tools {
                toolkit.register(Box::new(SingleToolAdapter::new(tool)), None);
            }
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

    if let Some(ref allowlist) = policy.base_allowed_tools {
        for tool in toolkit.get_definitions() {
            if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                if !allowlist.contains(name) {
                    toolkit.unregister(name);
                }
            }
        }
    }

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
    let mut mcp_definitions = state.mcp_runtime.definitions().await;
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
        let skill_definitions = macaca_runtime_host::McpServerFactory::with_default_registry()
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

    toolkit
}

async fn register_mcp_definitions_with_service(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    agent_name: &str,
    definitions: &[McpServerDefinition],
) {
    if definitions.is_empty() {
        return;
    }
    let payloads = definitions
        .iter()
        .filter_map(|definition| match serde_json::to_value(definition) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(
                    server_id = %definition.id,
                    error = %error,
                    "failed to serialize MCP definition for service registration"
                );
                None
            }
        })
        .collect::<Vec<_>>();
    if payloads.is_empty() {
        return;
    }
    let command = McpRegisterCommand {
        trace: TraceContext::new("web-toolkit-mcp-register"),
        scope: McpServiceScope::agent_session(*app_id, session_id.unwrap_or_default(), agent_name)
            .unwrap_or_default(),
        definitions: payloads,
        policy: McpServicePolicyHints::default(),
    };
    if let Err(error) = state.mcp_client.register(command).await {
        tracing::warn!(
            error = %error,
            "MCP Service registration failed during toolkit assembly"
        );
    }
}

async fn load_app_mcp_overlay_definitions(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Vec<McpServerDefinition> {
    let app = {
        let registry = state.registry.read().await;
        registry.get_app(app_id).cloned()
    };
    let Some(app) = app else {
        return Vec::new();
    };
    let path = app.path.join("mcp.yaml");
    if !path.exists() {
        return Vec::new();
    }
    let Ok(content) = tokio::fs::read_to_string(&path).await else {
        return Vec::new();
    };
    match serde_yaml::from_str::<McpRegistryConfig>(&content)
        .map_err(|e| e.to_string())
        .and_then(|config| {
            macaca_runtime_host::McpServerFactory::with_default_registry()
                .from_registry_config(config, McpDefinitionSource::App)
        }) {
        Ok(definitions) => definitions,
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                path = %path.display(),
                error = %error,
                "Failed to load app MCP overlay"
            );
            Vec::new()
        }
    }
}

async fn emit_mcp_runtime_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) {
    let Some(session_id) = session_id else {
        return;
    };

    for plan in mcp_runtime_event_plans(agent_name, statuses) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

async fn emit_skill_mcp_alias_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) {
    let Some(session_id) = session_id else {
        return;
    };
    for plan in skill_mcp_alias_event_plans(agent_name, statuses) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

async fn emit_mcp_starting_events(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
    definitions: &[McpServerDefinition],
) {
    let Some(session_id) = session_id else {
        return;
    };
    for plan in mcp_starting_event_plans(agent_name, definitions) {
        emit_mcp_event(
            state,
            session_id,
            plan.event_type,
            agent_name,
            &plan.payload,
        )
        .await;
    }
}

/// A deterministic EventLog/SSE event plan for MCP lifecycle visibility.
///
/// `build_toolkit` emits these plans through [`emit_runtime_event`], which
/// persists to EventLog before forwarding to SSE.  Keeping this as a pure value
/// object makes the service-backed migration auditable without constructing a
/// full `AppState` in unit tests, while preserving the existing event names and
/// payload shape used by the UI.
#[derive(Debug, Clone, PartialEq)]
struct McpRuntimeEventPlan {
    event_type: &'static str,
    payload: serde_json::Value,
}

/// Build the legacy MCP runtime events from service-probed statuses.
///
/// The event ordering is part of the user-visible runtime trace contract:
/// every non-disabled status emits `mcp_server_resolved` first, then a terminal
/// readiness/failure event, and ready servers with exposed tools emit the
/// `mcp_tools_registered` follow-up.
fn mcp_runtime_event_plans(
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) -> Vec<McpRuntimeEventPlan> {
    let mut plans = Vec::new();
    for status in statuses {
        if matches!(status.state, McpRuntimeStatusState::Disabled) {
            continue;
        }

        let payload = serde_json::json!({
            "agent": agent_name,
            "server_id": status.server_id,
            "transport": status.transport,
            "lifecycle": status.lifecycle,
            "session_mode": status.session_mode,
            "state": status.state,
            "exposed_tools": status.exposed_tools,
            "failure_reason": status.failure_reason,
        });

        plans.push(McpRuntimeEventPlan {
            event_type: "mcp_server_resolved",
            payload: payload.clone(),
        });
        match status.state {
            McpRuntimeStatusState::Ready => {
                plans.push(McpRuntimeEventPlan {
                    event_type: "mcp_server_ready",
                    payload: payload.clone(),
                });
                if !status.exposed_tools.is_empty() {
                    plans.push(McpRuntimeEventPlan {
                        event_type: "mcp_tools_registered",
                        payload,
                    });
                }
            }
            McpRuntimeStatusState::Failed | McpRuntimeStatusState::DependencyMissing => {
                plans.push(McpRuntimeEventPlan {
                    event_type: "mcp_server_failed",
                    payload,
                });
            }
            McpRuntimeStatusState::Disabled => {}
        }
    }
    plans
}

/// Build skill-backed MCP alias events without changing service ownership.
///
/// Skill alias events are a Web/UI compatibility surface.  The service-backed
/// runtime still owns registration and invocation, while this planner preserves
/// the existing alias event stream for users who watch skill-specific MCP
/// readiness in EventLog/SSE.
fn skill_mcp_alias_event_plans(
    agent_name: &str,
    statuses: &[McpRuntimeStatus],
) -> Vec<McpRuntimeEventPlan> {
    let mut plans = Vec::new();
    for status in statuses
        .iter()
        .filter(|status| status.server_id.starts_with("skill:"))
    {
        let payload = serde_json::json!({
            "agent": agent_name,
            "server_id": status.server_id,
            "state": status.state,
            "exposed_tools": status.exposed_tools,
            "failure_reason": status.failure_reason,
        });
        let event_type = match status.state {
            McpRuntimeStatusState::Ready => "skill_mcp_ready",
            McpRuntimeStatusState::Failed | McpRuntimeStatusState::DependencyMissing => {
                "skill_mcp_failed"
            }
            McpRuntimeStatusState::Disabled => "skill_mcp_disabled",
        };
        plans.push(McpRuntimeEventPlan {
            event_type,
            payload: payload.clone(),
        });
        if matches!(status.state, McpRuntimeStatusState::Ready) && !status.exposed_tools.is_empty()
        {
            plans.push(McpRuntimeEventPlan {
                event_type: "skill_mcp_tools_registered",
                payload,
            });
        }
    }
    plans
}

/// Build `mcp_server_starting` events for enabled definitions.
///
/// Starting events intentionally happen before the service registration call in
/// `build_toolkit`, matching the old direct-registration lifecycle narrative
/// while allowing the actual runtime state to be owned by `service.mcp`.
fn mcp_starting_event_plans(
    agent_name: &str,
    definitions: &[McpServerDefinition],
) -> Vec<McpRuntimeEventPlan> {
    definitions
        .iter()
        .filter(|definition| definition.enabled)
        .map(|definition| McpRuntimeEventPlan {
            event_type: "mcp_server_starting",
            payload: serde_json::json!({
                "agent": agent_name,
                "server_id": definition.id,
                "lifecycle": definition.lifecycle,
                "session_mode": definition.session_mode,
                "state": "starting",
            }),
        })
        .collect()
}

async fn emit_mcp_event(
    state: &Arc<AppState>,
    session_id: &str,
    event_type: &str,
    source: &str,
    payload: &serde_json::Value,
) {
    emit_runtime_event(
        state,
        session_id,
        event_type,
        source,
        payload.get("agent").and_then(|value| value.as_str()),
        payload.clone(),
    )
    .await;
}

async fn app_agent_names(state: &Arc<AppState>, app_id: &ApplicationId) -> Option<HashSet<String>> {
    let app = {
        let registry = state.registry.read().await;
        registry.get_app(app_id).cloned()
    }?;

    match discovered_app_agent_names(&app) {
        Ok(agent_names) => Some(agent_names.into_iter().collect()),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Failed to resolve app-scoped agent names; falling back to global agents"
            );
            None
        }
    }
}

async fn resolve_tool_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> AgentToolPolicy {
    let manifest = state.kernel.get_agent_by_name(agent_name).await;
    let capabilities: HashSet<String> = manifest
        .as_ref()
        .map(|m| m.capabilities.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();
    let (manifest_allowed_tools, is_entry_agent) = {
        let registry = state.registry.read().await;
        registry
            .get_app(app_id)
            .and_then(|app| app_agent_manifest_view(&app.manifest, agent_name))
            .map(|agent| (Some(agent.allowed_tools().to_vec()), agent.is_entry_agent()))
            .unwrap_or((None, false))
    };
    let base_allowed_tools = manifest_allowed_tools
        .and_then(|allowed_tools| (!allowed_tools.is_empty()).then_some(allowed_tools))
        .map(|allowed_tools| allowed_tools.into_iter().collect())
        .or_else(|| {
            manifest.as_ref().and_then(|m| {
                (!m.permission.allowed_tools.is_empty())
                    .then_some(m.permission.allowed_tools.iter().cloned().collect())
            })
        });
    // Capability-driven policy (preferred):
    // - todo_goal_management: can create/check goals
    // - task_planning / todo_planning: can create/review/reassign todos
    // - todo_execution: worker task-board operations
    //
    // Backward compatibility:
    // - entry agent defaults to GoalManager
    // - non-planner/non-entry agents default to Worker
    let todo_policy = if capabilities.contains("todo_goal_management") {
        TodoToolPolicy::GoalManager
    } else if capabilities.contains("task_planning") || capabilities.contains("todo_planning") {
        TodoToolPolicy::Planner
    } else if capabilities.contains("todo_execution") {
        TodoToolPolicy::Worker
    } else if is_entry_agent {
        TodoToolPolicy::GoalManager
    } else {
        TodoToolPolicy::Worker
    };

    // Any supervisor-like agent should not receive executable TaskBoard todos.
    // Keep this capability-driven first, with entry-agent compatibility fallback.
    let app_agent_names = app_agent_names(state, app_id).await;
    let mut disallowed_task_assignees: HashSet<String> = state
        .kernel
        .list_agents()
        .await
        .into_iter()
        .filter(|m| {
            app_agent_names
                .as_ref()
                .is_none_or(|names| names.contains(&m.name))
        })
        .filter_map(|m| {
            let caps: HashSet<String> = m.capabilities.into_iter().map(|c| c.name).collect();
            let is_supervisor = caps.contains("todo_goal_management")
                || caps.contains("task_planning")
                || caps.contains("todo_planning");
            if is_supervisor {
                Some(m.name)
            } else {
                None
            }
        })
        .collect();
    if is_entry_agent {
        disallowed_task_assignees.insert(agent_name.to_string());
    }

    AgentToolPolicy {
        base_allowed_tools,
        todo_policy,
        disallowed_task_assignees,
    }
}

/// Register per-agent todo tools into the toolkit.
fn register_agent_tools(
    toolkit: &mut Toolkit,
    state: &AppState,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    goal_id: Option<macaca_proto::TaskId>,
    policy: &AgentToolPolicy,
    assignee_capabilities: &HashMap<String, Vec<String>>,
) {
    match policy.todo_policy {
        TodoToolPolicy::GoalManager => {
            let space = Arc::new(macaca_task::TaskSpace::for_session(
                app_id.clone(),
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            let rt = Arc::clone(&state.persist.run_tracer);
            let app = app_id.clone();
            let goal_to_session = Arc::clone(&state.sessions.goal_to_session);
            let framework_session_store = Arc::clone(&state.sessions.framework_session_store);
            let owner_agent = agent_name.to_string();
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CreateGoalTool {
                        space: Arc::clone(&space),
                        on_created: None,
                        on_goal_recorded: Some(Arc::new(move |goal: macaca_proto::TodoGoal| {
                            let rt = Arc::clone(&rt);
                            let app = app.clone();
                            let goal_to_session = Arc::clone(&goal_to_session);
                            let framework_session_store = Arc::clone(&framework_session_store);
                            let owner_agent = owner_agent.clone();
                            tokio::spawn(async move {
                                if let Some(session_id) = goal.session_id.clone() {
                                    goal_to_session
                                        .write()
                                        .await
                                        .insert(goal.id.to_string(), session_id.clone());
                                    let mut ctx = ExecutionContext::new(
                                        session_id.clone(),
                                        app.0.to_string(),
                                        owner_agent.clone(),
                                    );
                                    let _ = load_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &mut ctx,
                                    )
                                    .await;
                                    ctx.mark_paused(Some(format!(
                                        "waiting_goal_completion:{}",
                                        goal.id
                                    )));
                                    let _ = save_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &ctx,
                                    )
                                    .await;
                                }
                                crate::run_trace::emit_for_scope(
                                    &rt,
                                    goal.session_id.as_deref(),
                                    &app,
                                    crate::run_trace::phase::GOAL_CREATE_TOOL,
                                    "create_goal_tool",
                                    crate::run_trace::status::OK,
                                    Some(format!("goal_id={}", goal.id)),
                                    None,
                                    Some(goal.id.to_string()),
                                    None,
                                )
                                .await;
                            });
                        })),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CheckTodoProgressTool { space },
                ))),
                Some("todo"),
            );
        }
        TodoToolPolicy::Planner => {
            let space = Arc::new(macaca_task::TaskSpace::for_session(
                app_id.clone(),
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CreateTodoTool {
                        space: Arc::clone(&space),
                        coordinator_name: agent_name.to_string(),
                        disallowed_assignees: policy
                            .disallowed_task_assignees
                            .iter()
                            .cloned()
                            .collect(),
                        assignee_capabilities: assignee_capabilities.clone(),
                        active_goal_id: goal_id,
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CreateTodosTool {
                        create_todo: macaca_tools::CreateTodoTool {
                            space: Arc::clone(&space),
                            coordinator_name: agent_name.to_string(),
                            disallowed_assignees: policy
                                .disallowed_task_assignees
                                .iter()
                                .cloned()
                                .collect(),
                            assignee_capabilities: assignee_capabilities.clone(),
                            active_goal_id: goal_id,
                        },
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::ReviewTodoTool {
                        space: Arc::clone(&space),
                        on_reviewed: None,
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CheckTodoProgressTool {
                        space: Arc::clone(&space),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::ReassignTaskTool {
                        space: Arc::clone(&space),
                    },
                ))),
                Some("todo"),
            );
            let rt = Arc::clone(&state.persist.run_tracer);
            let app = app_id.clone();
            let framework_session_store = Arc::clone(&state.sessions.framework_session_store);
            let owner_agent = agent_name.to_string();
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::CreateGoalTool {
                        space,
                        on_created: None,
                        on_goal_recorded: Some(Arc::new(move |goal: macaca_proto::TodoGoal| {
                            let rt = Arc::clone(&rt);
                            let app = app.clone();
                            let framework_session_store = Arc::clone(&framework_session_store);
                            let owner_agent = owner_agent.clone();
                            tokio::spawn(async move {
                                if let Some(session_id) = goal.session_id.clone() {
                                    let mut ctx = ExecutionContext::new(
                                        session_id.clone(),
                                        app.0.to_string(),
                                        owner_agent.clone(),
                                    );
                                    let _ = load_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &mut ctx,
                                    )
                                    .await;
                                    ctx.mark_paused(Some(format!(
                                        "waiting_goal_completion:{}",
                                        goal.id
                                    )));
                                    let _ = save_module_state(
                                        framework_session_store.as_ref(),
                                        &session_id,
                                        &ctx,
                                    )
                                    .await;
                                }
                                crate::run_trace::emit_for_scope(
                                    &rt,
                                    goal.session_id.as_deref(),
                                    &app,
                                    crate::run_trace::phase::GOAL_CREATE_TOOL,
                                    "create_goal_tool",
                                    crate::run_trace::status::OK,
                                    Some(format!("goal_id={}", goal.id)),
                                    None,
                                    Some(goal.id.to_string()),
                                    None,
                                )
                                .await;
                            });
                        })),
                    },
                ))),
                Some("todo"),
            );
        }
        TodoToolPolicy::Worker => {
            // Worker agents: task board tools.
            let board = Arc::new(macaca_task::TaskBoard::for_agent(
                app_id.clone(),
                agent_name,
                session_id,
                Arc::clone(&state.persist.todo_store),
            ));
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::ClaimTaskTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::StartTaskTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::UpdateTaskProgressTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::SubmitTaskForReviewTool {
                        board: Arc::clone(&board),
                    },
                ))),
                Some("todo"),
            );
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    macaca_tools::ListMyTasksTool { board },
                ))),
                Some("todo"),
            );
        }
    }
}

fn normalize_tool_input(input: &serde_json::Value) -> Cow<'_, serde_json::Value> {
    if let Some(s) = input.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            if parsed.is_object() {
                return Cow::Owned(parsed);
            }
        }
    }
    Cow::Borrowed(input)
}

fn pick_path_str(input: &serde_json::Value) -> Option<&str> {
    for key in ["path", "file_path", "filepath", "file", "filename"] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn pick_content_str(input: &serde_json::Value) -> Option<&str> {
    for key in ["content", "text", "body"] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            return Some(s);
        }
    }
    None
}

fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> PathBuf {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

struct WorkspaceFileReadTool {
    workspace_root: PathBuf,
}

#[async_trait]
impl macaca_tools::Tool for WorkspaceFileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Relative paths resolve from the app workspace root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or workspace-relative file path to read" },
                "file_path": { "type": "string", "description": "Alias for path (same meaning)" },
                "filepath": { "type": "string", "description": "Alias for path (same meaning)" }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let raw_path = pick_path_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent(
                "file_read requires non-empty 'path' (or alias 'file_path' / 'filepath')".into(),
            )
        })?;
        let path = resolve_workspace_path(&self.workspace_root, raw_path);
        let content = fs::read_to_string(&path).await.map_err(|e| {
            macaca_proto::MacacaError::Agent(format!(
                "file_read failed for '{}': {}",
                path.display(),
                e
            ))
        })?;
        Ok(serde_json::json!({ "content": content, "path": path.display().to_string() }))
    }
}

struct WorkspaceFileWriteTool {
    workspace_root: PathBuf,
}

#[async_trait]
impl macaca_tools::Tool for WorkspaceFileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Relative paths resolve from the app workspace root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Workspace-relative or absolute file path to write" },
                "file_path": { "type": "string", "description": "Alias for path" },
                "filepath": { "type": "string", "description": "Alias for path" },
                "content": { "type": "string", "description": "Full file content as a string" },
                "text": { "type": "string", "description": "Alias for content" },
                "body": { "type": "string", "description": "Alias for content" }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let raw_path = pick_path_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent("file_write requires non-empty 'path'".into())
        })?;
        let content = pick_content_str(&input).ok_or_else(|| {
            macaca_proto::MacacaError::Agent(
                "file_write requires 'content' as a string (or alias 'text' / 'body')".into(),
            )
        })?;
        let path = resolve_workspace_path(&self.workspace_root, raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                macaca_proto::MacacaError::Agent(format!(
                    "file_write: failed to create dirs for '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }
        fs::write(&path, content).await.map_err(|e| {
            macaca_proto::MacacaError::Agent(format!(
                "file_write failed for '{}': {}",
                path.display(),
                e
            ))
        })?;
        Ok(serde_json::json!({
            "bytes_written": content.len(),
            "path": path.display().to_string()
        }))
    }
}

struct WorkspaceShellTool {
    workspace_root: PathBuf,
    default_timeout: Duration,
}

#[async_trait]
impl macaca_tools::Tool for WorkspaceShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command from the app workspace root. Relative paths resolve from that root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (optional)" }
            },
            "required": ["command"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> macaca_proto::MacacaResult<serde_json::Value> {
        let input = normalize_tool_input(&input);
        let command = input["command"].as_str().ok_or_else(|| {
            macaca_proto::MacacaError::Agent("shell requires 'command' field".into())
        })?;
        let timeout_secs = input["timeout_secs"]
            .as_u64()
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let fut = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace_root)
            .output();

        let output = timeout(timeout_secs, fut)
            .await
            .map_err(|_| {
                macaca_proto::MacacaError::Timeout(format!(
                    "shell command timed out after {}s: {}",
                    timeout_secs.as_secs(),
                    command
                ))
            })?
            .map_err(|e| {
                macaca_proto::MacacaError::Agent(format!("shell command failed to spawn: {}", e))
            })?;

        Ok(serde_json::json!({
            "stdout": String::from_utf8_lossy(&output.stdout).into_owned(),
            "stderr": String::from_utf8_lossy(&output.stderr).into_owned(),
            "exit_code": output.status.code().unwrap_or(-1),
            "cwd": self.workspace_root.display().to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        mcp_runtime_event_plans, mcp_starting_event_plans, normalize_tool_input,
        resolve_workspace_path, skill_mcp_alias_event_plans,
    };
    use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
    use macaca_runtime_host::{McpLifecycleScope, McpRuntimeStatus, McpRuntimeStatusState};

    #[test]
    fn resolve_workspace_path_joins_relative_path_to_workspace_root() {
        let workspace_root = Path::new("/tmp/macaca-workspace");

        let resolved = resolve_workspace_path(workspace_root, "shared/backend/main.go");

        assert_eq!(
            resolved,
            workspace_root
                .join("shared")
                .join("backend")
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
            r#"{"path":"shared/backend/main.go","content":"package main"}"#.into(),
        );

        let normalized = normalize_tool_input(&input);

        assert_eq!(
            normalized.get("path").and_then(|value| value.as_str()),
            Some("shared/backend/main.go")
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
        let source = include_str!("framework_toolkit.rs");
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
}
