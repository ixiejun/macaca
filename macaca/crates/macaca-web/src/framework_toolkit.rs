//! Toolkit construction and tool policy for framework-based agents.
//!
//! This module keeps `FrameworkRunner` focused on agent construction while
//! preserving the existing toolkit registration behavior.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::response::sse::Event;
use tokio::{fs, process::Command, time::timeout};

use macaca_app::{app_agent_manifest_view, discovered_app_agent_names};
use macaca_framework::adapter::{SingleToolAdapter, ToolSetBridge};
use macaca_framework::execution::ExecutionContext;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_framework::tool::Toolkit;
use macaca_persist::AppendEventCommand;
use macaca_proto::ApplicationId;

use crate::state::AppState;
use macaca_runtime_host::{
    McpDefinitionSource, McpRegistryConfig, McpRuntimeContext, McpRuntimeStatus,
    McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
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

    // Dynamically aggregate driver tools from the DriverRegistry.
    // This ensures tools from drivers loaded via `/api/drivers/reload` are
    // visible to agents without requiring a full restart.
    {
        let driver_tools = state.driver_runtime.collect_tools().await;
        for tool in driver_tools {
            toolkit.register(Box::new(SingleToolAdapter::new(tool)), None);
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

    if let Some(runtime) = state.memory_runtime.as_ref() {
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
                    crate::context_memory_tools::WorkspaceMemorySearchTool {
                        runtime: Arc::clone(runtime),
                        scope: scope.clone(),
                        default_limit: limit,
                    },
                ))),
                None,
            );
        }
        if policy.allows_base_tool("memory_get") {
            toolkit.register(
                Box::new(SingleToolAdapter::new(Box::new(
                    crate::context_memory_tools::WorkspaceMemoryGetTool {
                        runtime: Arc::clone(runtime),
                        scope: scope.clone(),
                    },
                ))),
                None,
            );
        }
        if policy.allows_base_tool("memory_forget") {
            if let Some(ts) = state.workspace_memory_tombstones.as_ref() {
                toolkit.register(
                    Box::new(SingleToolAdapter::new(Box::new(
                        crate::context_memory_tools::WorkspaceMemoryForgetTool {
                            runtime: Arc::clone(runtime),
                            scope: scope.clone(),
                            tombstones: Arc::clone(ts),
                        },
                    ))),
                    None,
                );
            }
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

    let mcp_context = McpRuntimeContext::for_agent(app_id, session_id.as_deref(), agent_name);
    let mcp_policy = McpToolPolicy::default();
    let close_emitter = mcp_close_event_emitter(state, session_id.as_deref(), agent_name);

    let mut mcp_definitions = state.mcp_runtime.definitions().await;
    mcp_definitions.extend(load_app_mcp_overlay_definitions(state, app_id).await);
    emit_mcp_starting_events(state, session_id.as_deref(), agent_name, &mcp_definitions).await;
    let mcp_statuses = state
        .mcp_runtime
        .register_definitions(
            &mut toolkit,
            mcp_definitions,
            &mcp_policy,
            &mcp_context,
            close_emitter.clone(),
        )
        .await;
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
        let skill_statuses = state
            .mcp_runtime
            .register_definitions(
                &mut toolkit,
                skill_definitions,
                &mcp_policy,
                &mcp_context,
                close_emitter,
            )
            .await;
        emit_skill_mcp_alias_events(state, session_id.as_deref(), agent_name, &skill_statuses)
            .await;
        emit_mcp_runtime_events(state, session_id.as_deref(), agent_name, &skill_statuses).await;
    }

    toolkit
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

fn mcp_close_event_emitter(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    agent_name: &str,
) -> Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>> {
    let session_id = session_id.map(ToString::to_string)?;
    let state = Arc::clone(state);
    let agent_name = agent_name.to_string();
    Some(Arc::new(move |status: McpRuntimeStatus| {
        let state = Arc::clone(&state);
        let session_id = session_id.clone();
        let agent_name = agent_name.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let sse_tx = {
                    let active_sessions = state.sessions.active_sessions.read().await;
                    active_sessions
                        .get(&session_id)
                        .map(|session| Arc::clone(&session.sse_tx))
                };
                let payload = serde_json::json!({
                    "agent": agent_name,
                    "server_id": status.server_id,
                    "lifecycle": status.lifecycle,
                    "session_mode": status.session_mode,
                    "state": "closed",
                    "failure_reason": status.failure_reason,
                });
                emit_mcp_event(
                    &state,
                    &session_id,
                    &sse_tx,
                    "mcp_server_closed",
                    &agent_name,
                    &payload,
                )
                .await;
            });
        }
    }))
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

    let sse_tx = {
        let active_sessions = state.sessions.active_sessions.read().await;
        active_sessions
            .get(session_id)
            .map(|session| Arc::clone(&session.sse_tx))
    };

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

        emit_mcp_event(
            state,
            session_id,
            &sse_tx,
            "mcp_server_resolved",
            agent_name,
            &payload,
        )
        .await;

        match status.state {
            McpRuntimeStatusState::Ready => {
                emit_mcp_event(
                    state,
                    session_id,
                    &sse_tx,
                    "mcp_server_ready",
                    agent_name,
                    &payload,
                )
                .await;
                if !status.exposed_tools.is_empty() {
                    emit_mcp_event(
                        state,
                        session_id,
                        &sse_tx,
                        "mcp_tools_registered",
                        agent_name,
                        &payload,
                    )
                    .await;
                }
            }
            McpRuntimeStatusState::Failed | McpRuntimeStatusState::DependencyMissing => {
                emit_mcp_event(
                    state,
                    session_id,
                    &sse_tx,
                    "mcp_server_failed",
                    agent_name,
                    &payload,
                )
                .await;
            }
            McpRuntimeStatusState::Disabled => {}
        }
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
    let sse_tx = {
        let active_sessions = state.sessions.active_sessions.read().await;
        active_sessions
            .get(session_id)
            .map(|session| Arc::clone(&session.sse_tx))
    };

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
        emit_mcp_event(state, session_id, &sse_tx, event_type, agent_name, &payload).await;
        if matches!(status.state, McpRuntimeStatusState::Ready) && !status.exposed_tools.is_empty()
        {
            emit_mcp_event(
                state,
                session_id,
                &sse_tx,
                "skill_mcp_tools_registered",
                agent_name,
                &payload,
            )
            .await;
        }
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
    let sse_tx = {
        let active_sessions = state.sessions.active_sessions.read().await;
        active_sessions
            .get(session_id)
            .map(|session| Arc::clone(&session.sse_tx))
    };

    for definition in definitions.iter().filter(|definition| definition.enabled) {
        let payload = serde_json::json!({
            "agent": agent_name,
            "server_id": definition.id,
            "lifecycle": definition.lifecycle,
            "session_mode": definition.session_mode,
            "state": "starting",
        });
        emit_mcp_event(
            state,
            session_id,
            &sse_tx,
            "mcp_server_starting",
            agent_name,
            &payload,
        )
        .await;
    }
}

async fn emit_mcp_event(
    state: &Arc<AppState>,
    session_id: &str,
    sse_tx: &Option<Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>>,
    event_type: &str,
    source: &str,
    payload: &serde_json::Value,
) {
    state
        .persist
        .event_log
        .append_command(AppendEventCommand::new(
            session_id,
            event_type,
            source,
            payload.clone(),
        ))
        .await;

    if let Some(sse_tx) = sse_tx {
        let sender = sse_tx.read().await;
        let _ = sender
            .send(Ok(Event::default()
                .event(event_type)
                .data(payload.to_string())))
            .await;
    }
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

    use super::{normalize_tool_input, resolve_workspace_path};

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
}
