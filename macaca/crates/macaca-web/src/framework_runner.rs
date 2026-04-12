//! Framework-based agent runner — builds `ReActAgent` instances from persona
//! configuration and bridges events to SSE.
//!
//! This module replaces the ad-hoc `AgenticLoop` execution with the
//! `macaca-framework` `ReActAgent`, providing:
//! - Unified tool management via `Toolkit` (with middleware chain)
//! - Working memory with tag-based filtering
//! - Hook system for SSE event bridging
//! - Pause/resume via `ToolMiddleware` for `create_goal` coordination

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::response::sse::Event;
use tokio::sync::{mpsc, Mutex};
use tokio::{fs, process::Command, time::timeout};

use macaca_framework::adapter::{RoutedLlmAdapter, SingleToolAdapter, ToolSetBridge};
use macaca_framework::agent::{Hook, HookRegistry, HookedAgent};
use macaca_framework::execution::ExecutionContext;
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::InMemoryWorkingMemory;
use macaca_framework::message::Msg;
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_framework::tool::{ToolError, ToolMiddleware, ToolResponse, Toolkit};
use macaca_persist::EventLog;
use macaca_proto::ApplicationId;
use macaca_runtime::agentic_loop::ResumeReason;
use macaca_sdk::AgentPersona;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// FrameworkRunner — Agent factory
// ---------------------------------------------------------------------------

/// Builds `ReActAgent` instances from the existing Macaca OS infrastructure.
///
/// This is the bridge between the OS layer (AppState, personas, tool registry)
/// and the framework layer (ReActAgent, Toolkit, WorkingMemory).
pub struct FrameworkRunner;

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

impl FrameworkRunner {
    /// Deprecated: do not use. All agents must be constructed through traced
    /// builders so execution is visible in EventLog and SSE.
    #[deprecated(
        note = "build_agent is disabled. Use build_traced_agent/build_traced_agent_with_goal/build_worker_agent/build_coordinator instead."
    )]
    pub async fn build_agent(
        _state: &Arc<AppState>,
        _app_id: &ApplicationId,
        _agent_name: &str,
        _session_id: Option<String>,
    ) -> Result<ReActAgent, String> {
        Err("FrameworkRunner::build_agent is disabled. Use a traced builder instead.".into())
    }

    /// Deprecated: do not use. All agents must be constructed through traced
    /// builders so execution is visible in EventLog and SSE.
    #[deprecated(
        note = "build_agent_with_goal is disabled. Use build_traced_agent_with_goal instead."
    )]
    pub async fn build_agent_with_goal(
        _state: &Arc<AppState>,
        _app_id: &ApplicationId,
        _agent_name: &str,
        _session_id: Option<String>,
        _goal_id: Option<macaca_proto::TaskId>,
    ) -> Result<ReActAgent, String> {
        Err(
            "FrameworkRunner::build_agent_with_goal is disabled. Use a traced builder instead."
                .into(),
        )
    }

    /// Build a traced `ReActAgent` without goal context.
    pub async fn build_traced_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_traced_agent_with_goal(
            state, app_id, agent_name, session_id, task_id, executor, None,
        )
        .await
    }

    /// Build a worker `ReActAgent` wrapped with `HookedAgent` that emits execution
    /// events (thinking, tool_call, tool_result, assistant) to the executor broadcast
    /// channel for SSE + EventLog persistence.
    pub async fn build_worker_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_traced_agent(state, app_id, agent_name, session_id, task_id, executor).await
    }

    /// Build a traced `ReActAgent` that emits execution events through the
    /// executor broadcast channel. Supports optional goal context so planner
    /// calls to `create_todo` can be linked to the active goal.
    pub async fn build_traced_agent_with_goal(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
        goal_id: Option<macaca_proto::TaskId>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit =
            Self::build_toolkit(state, app_id, agent_name, session_id.clone(), goal_id).await;

        // Executor tool middleware — emits tool_call / tool_result via broadcast
        toolkit.add_middleware(Box::new(ExecutorToolMiddleware {
            executor: Arc::clone(&executor),
            task_id,
            agent_name: agent_name.to_string(),
        }));

        let model_name = selection.primary.reference();

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(25)
            .with_model_name(model_name);

        // Wrap with HookedAgent + ExecutorEmitterHook
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(ExecutorEmitterHook {
            executor: Arc::clone(&executor),
            task_id,
            agent_name: agent_name.to_string(),
            iteration: std::sync::atomic::AtomicUsize::new(0),
        }));
        let hooked = HookedAgent::new(agent, hooks);

        Ok(hooked)
    }

    /// Build a framework-native runtime agent for executor call sites that
    /// still depend on `AgentRunner`. Optional event channels receive
    /// `AgentExecutionEvent` updates directly from framework hooks.
    pub async fn build_runtime_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        goal_id: Option<macaca_proto::TaskId>,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit = Self::build_toolkit(state, app_id, agent_name, session_id, goal_id).await;

        if let Some(ref tx) = event_tx {
            toolkit.add_middleware(Box::new(ChannelToolMiddleware { tx: tx.clone() }));
        }

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(25)
            .with_model_name(selection.primary.reference());

        let mut hooks = HookRegistry::new();
        if let Some(tx) = event_tx {
            hooks.register_instance_hook(Box::new(ChannelEmitterHook {
                tx,
                iteration: std::sync::atomic::AtomicUsize::new(0),
            }));
        }

        Ok(HookedAgent::new(agent, hooks))
    }

    /// Build a coordinator `ReActAgent` wrapped with `HookedAgent` for SSE bridging
    /// and `PauseOnGoalMiddleware` for pause/resume on `create_goal`.
    ///
    /// Returns `(HookedAgent<ReActAgent>, CancellationToken)`.
    pub async fn build_coordinator(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        pause_signal: Arc<AtomicBool>,
        resume_rx: mpsc::Receiver<ResumeReason>,
    ) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit =
            Self::build_toolkit(state, app_id, agent_name, session_id.clone(), None).await;

        // SSE tool middleware — emits tool_call / tool_result events
        toolkit.add_middleware(Box::new(SseToolMiddleware {
            tx: sse_tx.clone(),
            agent_name: agent_name.to_string(),
            event_log: Some(Arc::clone(&state.persist.event_log)),
            session_id: session_id.clone(),
        }));

        // Pause-on-goal middleware — blocks until goal completes
        toolkit.add_middleware(Box::new(PauseOnGoalMiddleware {
            pause_signal,
            resume_rx: Arc::new(Mutex::new(resume_rx)),
        }));

        let model_name = selection.primary.reference();

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(50)
            .with_model_name(model_name);

        let cancel_token = agent.cancel_token();

        // Wrap with HookedAgent + SseEmitterHook
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(SseEmitterHook {
            tx: sse_tx,
            agent_name: agent_name.to_string(),
            event_log: Some(Arc::clone(&state.persist.event_log)),
            session_id,
        }));
        let hooked = HookedAgent::new(agent, hooks);

        Ok((hooked, cancel_token))
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Resolve the routed model selection for an agent.
    /// Priority: agent manifest model > app llm_config > system default.
    async fn resolve_model_selection(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> Result<macaca_llm::ModelSelection, String> {
        let agent_model = state
            .kernel
            .get_agent_by_name(agent_name)
            .await
            .and_then(|manifest| (!manifest.model.is_empty()).then_some(manifest.model));

        let app_defaults = {
            let registry = state.registry.read().await;
            registry
                .get_app(app_id)
                .and_then(|app| app.manifest.llm_config.clone())
        };

        state
            .llm_router
            .resolve_selection(&macaca_llm::ModelSelectionRequest {
                agent_model,
                app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
                app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
                system_model: (!state.config.default_model.is_empty())
                    .then_some(state.config.default_model.clone()),
                ..Default::default()
            })
            .map_err(|e| e.to_string())
    }

    /// Load the agent's persona and build the system prompt.
    async fn build_system_prompt(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> String {
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == *app_id)
                .map(|(_, path)| path.clone())
        };

        let persona = if let Some(ref dir) = app_dir {
            let persona_dir = dir.join("personas").join(agent_name);
            if persona_dir.exists() {
                AgentPersona::load_from_directory(&persona_dir).await.ok()
            } else {
                None
            }
        } else {
            None
        };

        let mut prompt = if let Some(ref p) = persona {
            p.to_system_prompt(None)
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };

        // Inject capabilities
        let manifests = state.kernel.list_agents().await;
        if let Some(info) = manifests.iter().find(|m| m.name == agent_name) {
            let caps: Vec<&str> = info.capabilities.iter().map(|c| c.name.as_str()).collect();
            if !caps.is_empty() {
                prompt.push_str(&format!("\n\nYour capabilities: {}", caps.join(", ")));
            }
        }

        // Inject workspace paths
        {
            let workspaces = state.config.app_workspaces.read().await;
            if let Some(ws) = workspaces.get(app_id) {
                prompt.push_str(&format!(
                    "\n\n## Workspace Paths\n\
                     - Workspace root (default cwd for file/shell tools): {}\n\
                     - Shared workspace: {}\n\
                     - Your private workspace: {}\n\
                     Relative paths are resolved from the workspace root above. \
                     Create project files in the shared workspace. \
                     Use your private workspace for temporary/scratch files only.",
                    ws.root.display(),
                    ws.shared.display(),
                    ws.agent_workspace(agent_name).display(),
                ));
            }
        }

        prompt
    }

    /// Build a `Toolkit` with base tools + per-agent todo tools.
    async fn build_toolkit(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        goal_id: Option<macaca_proto::TaskId>,
    ) -> Toolkit {
        let policy = Self::resolve_tool_policy(state, app_id, agent_name).await;

        // Base tools from the global ToolSet via ToolSetBridge.
        // state.tools is Arc<dyn ToolSet>, which ToolSetBridge accepts directly.
        let mut toolkit = ToolSetBridge::from_tool_set(Arc::clone(&state.tools));

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

        let assignee_capabilities: HashMap<String, Vec<String>> = state
            .kernel
            .list_agents()
            .await
            .into_iter()
            .map(|m| {
                let profile = m
                    .capabilities
                    .into_iter()
                    .map(|c| format!("{} {}", c.name, c.description))
                    .collect::<Vec<_>>();
                (m.name, profile)
            })
            .collect();

        // Register per-agent todo tools
        Self::register_agent_tools(
            &mut toolkit,
            state,
            app_id,
            agent_name,
            session_id,
            goal_id,
            &policy,
            &assignee_capabilities,
        );

        toolkit
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
        let base_allowed_tools = manifest.as_ref().and_then(|m| {
            (!m.permission.allowed_tools.is_empty())
                .then_some(m.permission.allowed_tools.iter().cloned().collect())
        });
        let is_entry_agent = {
            let registry = state.registry.read().await;
            registry
                .get_app(app_id)
                .and_then(|app| app.manifest.entry_agent.clone())
                .is_some_and(|entry| entry == agent_name)
        };
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
        let mut disallowed_task_assignees: HashSet<String> = state
            .kernel
            .list_agents()
            .await
            .into_iter()
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
                let space = Arc::new(macaca_task::TaskSpace::new(
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
                            on_goal_recorded: Some(Arc::new(
                                move |goal: macaca_proto::TodoGoal| {
                                    let rt = Arc::clone(&rt);
                                    let app = app.clone();
                                    let goal_to_session = Arc::clone(&goal_to_session);
                                    let framework_session_store =
                                        Arc::clone(&framework_session_store);
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
                                },
                            )),
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
                let space = Arc::new(macaca_task::TaskSpace::new(
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
                            on_goal_recorded: Some(Arc::new(
                                move |goal: macaca_proto::TodoGoal| {
                                    let rt = Arc::clone(&rt);
                                    let app = app.clone();
                                    let framework_session_store =
                                        Arc::clone(&framework_session_store);
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
                                },
                            )),
                        },
                    ))),
                    Some("todo"),
                );
            }
            TodoToolPolicy::Worker => {
                // Worker agents: task board tools
                let board = Arc::new(macaca_task::TaskBoard::new(
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
}

impl AgentToolPolicy {
    fn allows_base_tool(&self, tool_name: &str) -> bool {
        self.base_allowed_tools
            .as_ref()
            .map(|allowlist| allowlist.contains(tool_name))
            .unwrap_or(true)
    }
}

fn normalize_tool_input(input: &serde_json::Value) -> std::borrow::Cow<'_, serde_json::Value> {
    if let Some(s) = input.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            if parsed.is_object() {
                return std::borrow::Cow::Owned(parsed);
            }
        }
    }
    std::borrow::Cow::Borrowed(input)
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

fn truncate_tool_output(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...[truncated, {} bytes]", &text[..end], text.len())
}

const TOOL_TRACE_OUTPUT_MAX_BYTES: usize = 2000;

fn tool_response_text(response: &ToolResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            macaca_framework::message::ContentBlock::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn tool_trace_output(response: &ToolResponse) -> String {
    truncate_tool_output(&tool_response_text(response), TOOL_TRACE_OUTPUT_MAX_BYTES)
}

fn tool_call_event(name: &str, args: &serde_json::Value) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolCall {
        tool_name: name.to_string(),
        tool_input: args.clone(),
        call_id: None,
    }
}

fn tool_result_event(name: &str, output: String) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolResult {
        tool_name: name.to_string(),
        output,
        is_error: None,
    }
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

// ---------------------------------------------------------------------------
// SseEmitterHook — bridges ReActAgent lifecycle to SSE
// ---------------------------------------------------------------------------

/// Hook that emits SSE events at the start and end of a `reply` call.
pub struct SseEmitterHook {
    tx: mpsc::Sender<Result<Event, Infallible>>,
    agent_name: String,
    event_log: Option<Arc<EventLog>>,
    session_id: Option<String>,
}

#[async_trait]
impl Hook for SseEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "thinking",
                    "coordinator",
                    serde_json::json!({
                        "iteration": 0,
                    }),
                )
                .await;
        }
        let event = Event::default().event("thinking").data(
            serde_json::json!({
                "iteration": 0,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "content",
                    "coordinator",
                    serde_json::json!({
                        "content": text,
                    }),
                )
                .await;
            event_log
                .append(
                    session_id,
                    "done",
                    "coordinator",
                    serde_json::json!({
                        "model": "",
                        "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                        "iterations": 0,
                        "tools_used": [],
                    }),
                )
                .await;
        }
        let content_event = Event::default().event("content").data(
            serde_json::json!({
                "content": text,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(content_event)).await;

        let done_event = Event::default().event("done").data(
            serde_json::json!({
                "model": "",
                "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                "iterations": 0,
                "tools_used": [],
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(done_event)).await;

        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// SseToolMiddleware — bridges tool calls/results to SSE
// ---------------------------------------------------------------------------

/// Middleware that emits SSE events for every tool invocation.
pub struct SseToolMiddleware {
    tx: mpsc::Sender<Result<Event, Infallible>>,
    agent_name: String,
    event_log: Option<Arc<EventLog>>,
    session_id: Option<String>,
}

#[async_trait]
impl ToolMiddleware for SseToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "tool_call",
                    "coordinator",
                    serde_json::json!({
                        "tool_name": name,
                        "tool_input": args.clone(),
                    }),
                )
                .await;
        }
        let event = Event::default().event("tool_call").data(
            serde_json::json!({
                "tool_name": name,
                "tool_input": args.clone(),
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        let display_result = tool_trace_output(response);

        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "tool_result",
                    "coordinator",
                    serde_json::json!({
                        "tool_name": name,
                        "output": display_result,
                    }),
                )
                .await;
        }

        let event = Event::default().event("tool_result").data(
            serde_json::json!({
                "tool_name": name,
                "output": display_result,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ChannelEmitterHook — bridges ReActAgent lifecycle to AgentExecutionEvent
// ---------------------------------------------------------------------------

pub struct ChannelEmitterHook {
    tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
    iteration: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl Hook for ChannelEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let iter = self.iteration.fetch_add(1, Ordering::Relaxed);
        let _ = self
            .tx
            .send(macaca_proto::AgentExecutionEvent::Thinking {
                iteration: iter,
                content: None,
            })
            .await;
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if !text.is_empty() {
            let _ = self
                .tx
                .send(macaca_proto::AgentExecutionEvent::Assistant { content: text })
                .await;
        }
        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// ChannelToolMiddleware — bridges tool calls/results to AgentExecutionEvent
// ---------------------------------------------------------------------------

pub struct ChannelToolMiddleware {
    tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
}

#[async_trait]
impl ToolMiddleware for ChannelToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        let _ = self.tx.send(tool_call_event(name, args)).await;
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        let _ = self
            .tx
            .send(tool_result_event(name, tool_trace_output(response)))
            .await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ExecutorEmitterHook — bridges ReActAgent lifecycle to executor broadcast
// ---------------------------------------------------------------------------

/// Hook that emits executor events at the start and end of a `reply` call.
/// Used by worker agents to push thinking/assistant events to SSE + EventLog.
pub struct ExecutorEmitterHook {
    executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    task_id: macaca_proto::TaskId,
    agent_name: String,
    iteration: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl Hook for ExecutorEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let iter = self.iteration.fetch_add(1, Ordering::Relaxed);
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: macaca_proto::AgentExecutionEvent::Thinking {
                    iteration: iter,
                    content: None,
                },
            });
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if !text.is_empty() {
            self.executor
                .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                    task_id: self.task_id,
                    agent: self.agent_name.clone(),
                    event: macaca_proto::AgentExecutionEvent::Assistant { content: text },
                });
        }
        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// ExecutorToolMiddleware — bridges tool calls/results to executor broadcast
// ---------------------------------------------------------------------------

/// Middleware that emits executor events for every tool invocation.
/// Used by worker agents to push tool_call/tool_result events to SSE + EventLog.
pub struct ExecutorToolMiddleware {
    executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    task_id: macaca_proto::TaskId,
    agent_name: String,
}

#[async_trait]
impl ToolMiddleware for ExecutorToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_call_event(name, args),
            });
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_result_event(name, tool_trace_output(response)),
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{tool_response_text, truncate_tool_output};
    use macaca_framework::message::{ContentBlock, TextBlock};
    use macaca_framework::tool::ToolResponse;

    #[test]
    fn truncate_tool_output_respects_utf8_boundaries() {
        let text = "─".repeat(800);

        let truncated = truncate_tool_output(&text, 2000);

        assert!(truncated.ends_with("[truncated, 2400 bytes]"));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn truncate_tool_output_keeps_short_text_unchanged() {
        let text = "北京 weather";

        assert_eq!(truncate_tool_output(text, 2000), text);
    }

    #[test]
    fn tool_response_text_joins_multiple_text_blocks() {
        let response = ToolResponse {
            content: vec![
                ContentBlock::Text(TextBlock {
                    text: "hello".into(),
                }),
                ContentBlock::Text(TextBlock {
                    text: " world".into(),
                }),
            ],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "hello world");
    }

    #[test]
    fn tool_response_text_returns_empty_string_for_empty_response() {
        let response = ToolResponse {
            content: Vec::new(),
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "");
    }
}

// ---------------------------------------------------------------------------
// PauseOnGoalMiddleware — pauses coordinator when create_goal is called
// ---------------------------------------------------------------------------

/// Middleware that blocks after `create_goal` tool execution until the goal
/// completes (via `resume_rx`). This replaces the `PausableAgenticLoop`'s
/// external pause signal mechanism with a tool-level block.
pub struct PauseOnGoalMiddleware {
    pause_signal: Arc<AtomicBool>,
    resume_rx: Arc<Mutex<mpsc::Receiver<ResumeReason>>>,
}

#[async_trait]
impl ToolMiddleware for PauseOnGoalMiddleware {
    async fn before(&self, _name: &str, _args: &mut serde_json::Value) -> Result<(), ToolError> {
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        if name != "create_goal" {
            return Ok(());
        }

        tracing::info!("PauseOnGoalMiddleware: create_goal detected, pausing coordinator");
        self.pause_signal.store(true, Ordering::SeqCst);

        // Wait for the goal to complete (GoalCompleted sends resume signal).
        // Autonomous goals can legitimately run longer than a fixed HTTP-era
        // timeout; ending this wait early loses the paused coordinator.
        let mut rx = self.resume_rx.lock().await;
        match rx.recv().await {
            Some(reason) => {
                self.pause_signal.store(false, Ordering::SeqCst);
                let context = match &reason {
                    ResumeReason::DelegateCompleted { output, .. } => output.clone(),
                    _ => "Goal processing completed.".to_string(),
                };
                response
                    .content
                    .push(macaca_framework::message::ContentBlock::Text(
                        macaca_framework::message::TextBlock {
                            text: format!("\n\n[Goal completed: {}]", context),
                        },
                    ));
                tracing::info!("PauseOnGoalMiddleware: resumed after goal completion");
            }
            None => {
                self.pause_signal.store(false, Ordering::SeqCst);
                tracing::warn!("PauseOnGoalMiddleware: resume channel closed");
            }
        }
        Ok(())
    }
}
