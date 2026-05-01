//! Framework-based agent runner — builds `ReActAgent` instances from persona
//! configuration and bridges events to SSE.
//!
//! This module replaces the ad-hoc `AgenticLoop` execution with the
//! `macaca-framework` `ReActAgent`, providing:
//! - Unified tool management via `Toolkit` (with middleware chain)
//! - Working memory with tag-based filtering
//! - Hook system for SSE event bridging
//! - Pause/resume via `ToolMiddleware` for `create_goal` coordination

use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::response::sse::Event;
use tokio::sync::{mpsc, Mutex};

use macaca_agent::{AgentCapabilitySet, AgentServices, AgentTransitionReason};
use macaca_app::{app_agent_manifest_view, app_agent_prompt_semantics};
use macaca_framework::adapter::RoutedLlmAdapter;
use macaca_framework::agent::{Hook, HookRegistry, HookedAgent};
use macaca_framework::construction::{
    AgentBuildIntent, AgentBuildRequest, AgentBuildRequestBuilder, AgentExecutionInput,
    AgentExecutionLauncher, AgentExecutionOutput, AgentIdentity, AgentLifecycleConfig,
    AgentToolConfig, AgentTraceContext, ApplicationPromptParts, ApplicationSemantics,
    ApplicationToolPolicy, TracedAgentFactory,
};
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::InMemoryWorkingMemory;
use macaca_framework::message::Msg;
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::tool::{ToolError, ToolMiddleware, ToolResponse, Toolkit};
use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::{AgentState, ApplicationId, Capability};
use macaca_runtime::agentic_loop::ResumeReason;
use macaca_sdk::AgentPersona;
use macaca_skill::{SkillPolicy, SkillRuntime, SkillRuntimeOptions};

use crate::state::AppState;

// ---------------------------------------------------------------------------
// FrameworkRunner — Agent factory
// ---------------------------------------------------------------------------

/// Builds `ReActAgent` instances from the existing Macaca OS infrastructure.
///
/// This is the bridge between the OS layer (AppState, personas, tool registry)
/// and the framework layer (ReActAgent, Toolkit, WorkingMemory).
pub struct FrameworkRunner;

enum FrameworkRunnerBuildMode {
    Executor {
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    },
    Runtime {
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    },
    Coordinator {
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        pause_signal: Arc<AtomicBool>,
        resume_rx: mpsc::Receiver<ResumeReason>,
    },
}

struct WebTracedAgentFactory {
    state: Arc<AppState>,
    build_mode: FrameworkRunnerBuildMode,
}

struct PreparedAgentParts {
    selection: macaca_llm::ModelSelection,
    toolkit: Toolkit,
}

enum StandardAgentMode {
    Executor {
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    },
    Runtime {
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    },
}

enum DriverTraceRoute {
    Executor {
        state: Arc<AppState>,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
        task_id: macaca_proto::TaskId,
        agent_name: String,
        session_id: Option<String>,
    },
    Runtime {
        tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
    },
    Coordinator {
        tx: mpsc::Sender<Result<Event, Infallible>>,
        event_log: Arc<EventLog>,
        agent_name: String,
        session_id: Option<String>,
    },
}

#[async_trait]
impl TracedAgentFactory for WebTracedAgentFactory {
    type Output = HookedAgent<ReActAgent>;

    async fn build(&self, request: AgentBuildRequest) -> Result<Self::Output, String> {
        match &self.build_mode {
            FrameworkRunnerBuildMode::Executor { executor } => {
                self.build_executor_agent(request, Arc::clone(executor))
                    .await
            }
            FrameworkRunnerBuildMode::Runtime { event_tx } => {
                self.build_runtime_agent(request, event_tx.clone()).await
            }
            FrameworkRunnerBuildMode::Coordinator { .. } => {
                Err("Coordinator construction requires owned channels".into())
            }
        }
    }
}

#[async_trait]
impl AgentExecutionLauncher for WebTracedAgentFactory {
    async fn launch(
        &self,
        intent: AgentBuildIntent,
        input: AgentExecutionInput,
    ) -> Result<AgentExecutionOutput, String> {
        let request = AgentBuildRequestBuilder::new(input.identity.clone(), intent)
            .system_prompt(input.prompt)
            .services(AgentServices::default())
            .capabilities(AgentCapabilitySet::default())
            .lifecycle(AgentLifecycleConfig::default())
            .trace(AgentTraceContext {
                session_id: input.session_id.clone(),
                task_id: input.task_id,
                source_agent: input.identity.agent_name.clone(),
            })
            .tools(AgentToolConfig::default())
            .build()?;

        let _ = <Self as TracedAgentFactory>::build(self, request).await?;
        Ok(AgentExecutionOutput {
            agent_name: input.identity.agent_name,
            session_id: input.session_id,
            task_id: input.task_id,
        })
    }
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
        Self::build_for_intent(
            state,
            app_id,
            agent_name,
            session_id,
            task_id,
            executor,
            AgentBuildIntent::RuntimeAgent,
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
        Self::build_for_intent(
            state,
            app_id,
            agent_name,
            session_id,
            task_id,
            executor,
            AgentBuildIntent::WorkerTask { task_id },
        )
        .await
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
        Self::build_for_intent(
            state,
            app_id,
            agent_name,
            session_id,
            task_id,
            executor,
            AgentBuildIntent::PlannerFollowUp { goal_id },
        )
        .await
    }

    /// Build a traced planner agent for goal decomposition only.
    ///
    /// This keeps decomposition visible while limiting the available action
    /// surface to todo creation, so the planner cannot drift into review,
    /// reassignment, or goal management during initial planning.
    pub async fn build_planner_decomposition_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
        goal_id: Option<macaca_proto::TaskId>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_for_intent(
            state,
            app_id,
            agent_name,
            session_id,
            task_id,
            executor,
            AgentBuildIntent::PlannerDecomposition { goal_id },
        )
        .await
    }

    /// Build a traced agent from an explicit framework build intent.
    ///
    /// This is the task-facing contract used by planner/worker runtime
    /// consumers so they do not need to know legacy web builder naming.
    pub async fn build_for_intent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
        intent: AgentBuildIntent,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let tools = match &intent {
            AgentBuildIntent::PlannerDecomposition { goal_id } => AgentToolConfig {
                goal_id: *goal_id,
                suppress_worker_lifecycle_tools: false,
                allowed_tool_names: Some(vec!["create_todo".into(), "create_todos".into()]),
            },
            AgentBuildIntent::PlannerFollowUp { goal_id }
            | AgentBuildIntent::GoalEvaluation { goal_id } => AgentToolConfig {
                goal_id: *goal_id,
                ..Default::default()
            },
            AgentBuildIntent::PlannerReview { .. } | AgentBuildIntent::WorkerTask { .. } => {
                AgentToolConfig {
                    suppress_worker_lifecycle_tools: true,
                    ..Default::default()
                }
            }
            _ => AgentToolConfig::default(),
        };
        let goal_id = tools.goal_id;
        let request = Self::build_request(
            state, app_id, agent_name, session_id, task_id, goal_id, intent, tools,
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Executor { executor },
        };
        factory.build(request).await
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
        let request = Self::build_request(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            goal_id.unwrap_or_else(macaca_proto::TaskId::new),
            goal_id,
            AgentBuildIntent::RuntimeAgent,
            AgentToolConfig {
                goal_id,
                ..Default::default()
            },
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Runtime { event_tx },
        };
        factory.build(request).await
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
        let request = Self::build_request(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            macaca_proto::TaskId::new(),
            None,
            AgentBuildIntent::CoordinatorChat,
            AgentToolConfig::default(),
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Coordinator {
                sse_tx,
                pause_signal,
                resume_rx,
            },
        };

        factory.build_coordinator(request).await
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    async fn build_request(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        goal_id: Option<macaca_proto::TaskId>,
        intent: AgentBuildIntent,
        tools: AgentToolConfig,
    ) -> Result<AgentBuildRequest, String> {
        let app_manifest = {
            let registry = state.registry.read().await;
            registry.get_app(app_id).map(|app| app.manifest.clone())
        };
        let capabilities = Self::resolve_agent_capability_set(state, app_id, agent_name).await;
        let system_prompt =
            Self::build_system_prompt(state, app_id, agent_name, session_id.clone(), &capabilities)
                .await;
        let application = app_manifest.as_ref().map(|manifest| {
            let semantics = app_agent_prompt_semantics(manifest, agent_name);
            ApplicationSemantics {
                workflow_name: Some(semantics.workflow_name),
                entry_agent: Some(semantics.entry_agent),
                prompt_parts: semantics.prompt_parts.map(|parts| ApplicationPromptParts {
                    role: parts.role,
                    constraints: parts.constraints,
                    tools: parts.tools,
                    handoff: parts.handoff,
                }),
                tool_policy: ApplicationToolPolicy {
                    allowed_tool_names: semantics.tool_policy.allowed_tools,
                    execution_tool_names: semantics.tool_policy.execution_tools,
                    is_entry_agent: semantics.tool_policy.is_entry_agent,
                },
            }
        });

        AgentBuildRequestBuilder::new(
            AgentIdentity {
                app_id: *app_id,
                agent_name: agent_name.to_string(),
                session_id: session_id.clone(),
            },
            intent,
        )
        .system_prompt(system_prompt)
        .services(AgentServices::default())
        .capabilities(capabilities)
        .lifecycle(AgentLifecycleConfig::default())
        .trace(AgentTraceContext {
            session_id,
            task_id: Some(task_id),
            source_agent: agent_name.to_string(),
        })
        .tools(AgentToolConfig { goal_id, ..tools })
        .application_opt(application)
        .build()
    }

    async fn resolve_agent_capability_set(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> AgentCapabilitySet {
        {
            let registry = state.registry.read().await;
            if let Some(app) = registry.get_app(app_id) {
                if let Some(agent) = app_agent_manifest_view(&app.manifest, agent_name) {
                    return AgentCapabilitySet::from_legacy(
                        agent
                            .capabilities()
                            .iter()
                            .map(|capability| Capability {
                                name: capability.name.clone(),
                                description: capability.description.clone(),
                            })
                            .collect(),
                    );
                }
            }
        }
        let manifests = state.kernel.list_agents().await;
        let capabilities = manifests
            .into_iter()
            .find(|manifest| manifest.name == agent_name)
            .map(|manifest| manifest.capabilities)
            .unwrap_or_default();
        AgentCapabilitySet::from_legacy(capabilities)
    }

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
        session_id: Option<String>,
        capabilities: &AgentCapabilitySet,
    ) -> String {
        let app_manifest = {
            let registry = state.registry.read().await;
            registry.get_app(app_id).map(|app| app.manifest.clone())
        };
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
        } else if let Some(manifest) = app_manifest.as_ref() {
            app_agent_prompt_semantics(manifest, agent_name).base_prompt
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };

        // Inject capabilities from the macaca-agent capability abstraction.
        let flattened = capabilities.flatten_for_legacy_api();
        let caps: Vec<&str> = flattened.iter().map(|cap| cap.name.as_str()).collect();
        if !caps.is_empty() {
            prompt.push_str(&format!("\n\nYour capabilities: {}", caps.join(", ")));
        }

        // Inject workspace paths
        let workspace_root = {
            let workspaces = state.config.app_workspaces.read().await;
            workspaces.get(app_id).map(|ws| ws.root.clone())
        };
        let skill_policy = resolve_agent_skill_policy(state, app_id, agent_name).await;
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

        // Inject AgentSkills catalog for progressive disclosure.
        let snapshot_module = format!("skill_snapshot/{agent_name}");
        let loaded_snapshot = if let Some(session_id) = session_id.as_deref() {
            state
                .sessions
                .framework_session_store
                .load(session_id, &snapshot_module)
                .await
                .ok()
                .flatten()
                .and_then(|value| serde_json::from_value::<macaca_skill::SkillSnapshot>(value).ok())
        } else {
            None
        };
        let skill_snapshot = match loaded_snapshot {
            Some(snapshot) => Ok(snapshot),
            None => {
                let snapshot = SkillRuntime
                    .build_snapshot(
                        agent_name,
                        SkillRuntimeOptions {
                            workspace_dir: workspace_root,
                            app_dir,
                            policy: skill_policy,
                            ..Default::default()
                        },
                    )
                    .await;
                if let (Some(session_id), Ok(snapshot)) = (session_id.as_deref(), &snapshot) {
                    if let Ok(value) = serde_json::to_value(snapshot) {
                        let _ = state
                            .sessions
                            .framework_session_store
                            .save(session_id, &snapshot_module, value)
                            .await;
                    }
                }
                snapshot
            }
        };
        match skill_snapshot {
            Ok(snapshot) => {
                tracing::info!(
                    agent = %agent_name,
                    visible = snapshot.skills.len(),
                    filtered = snapshot.filtered.len(),
                    truncated = snapshot.truncated,
                    compact = snapshot.compact,
                    "skill_catalog_built"
                );
                if let Some(session_id) = session_id.as_deref() {
                    state
                        .persist
                        .event_log
                        .append_command(AppendEventCommand::new(
                            session_id,
                            "skill_catalog_built",
                            agent_name,
                            serde_json::json!({
                                "agent": agent_name,
                                "visible_count": snapshot.skills.len(),
                                "filtered_count": snapshot.filtered.len(),
                                "truncated": snapshot.truncated,
                                "compact": snapshot.compact,
                            }),
                        ))
                        .await;
                    state
                        .persist
                        .event_log
                        .append_command(AppendEventCommand::new(
                            session_id,
                            "skill_snapshot_created",
                            agent_name,
                            serde_json::json!({
                                "agent": agent_name,
                                "version": snapshot.version,
                                "skills": snapshot.skills.iter().map(|skill| {
                                    serde_json::json!({
                                        "name": skill.name,
                                        "location": skill.location,
                                        "source": skill.source,
                                    })
                                }).collect::<Vec<_>>(),
                                "filtered": snapshot.filtered,
                            }),
                        ))
                        .await;
                }
                if !snapshot.prompt.trim().is_empty() {
                    prompt.push_str("\n\n## Available Skills\n\n");
                    prompt.push_str(&snapshot.prompt);
                }
            }
            Err(error) => {
                tracing::warn!(agent = %agent_name, error = %error, "failed to build skill catalog");
            }
        }

        prompt
    }
}

impl WebTracedAgentFactory {
    fn validate_lifecycle_config(lifecycle: &AgentLifecycleConfig) -> Result<(), String> {
        if lifecycle.initial_state != AgentState::Created {
            return Err(format!(
                "unsupported initial agent state for traced construction: {:?}",
                lifecycle.initial_state
            ));
        }
        if let Some(policy) = &lifecycle.policy {
            if !policy.can_transition(
                lifecycle.initial_state,
                AgentState::Running,
                AgentTransitionReason::Start,
            ) {
                return Err("agent lifecycle policy rejects Created -> Running".into());
            }
        }
        Ok(())
    }

    async fn prepare_agent_parts(
        &self,
        request: &AgentBuildRequest,
        goal_id_override: Option<Option<macaca_proto::TaskId>>,
    ) -> Result<PreparedAgentParts, String> {
        Self::validate_lifecycle_config(&request.lifecycle)?;

        let selection = FrameworkRunner::resolve_model_selection(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
        )
        .await?;
        let mut toolkit = crate::framework_toolkit::build_toolkit(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
            request.identity.session_id.clone(),
            goal_id_override.unwrap_or(request.tools.goal_id),
        )
        .await;

        if request.tools.suppress_worker_lifecycle_tools {
            for tool_name in [
                "claim_task",
                "start_task",
                "update_task_progress",
                "submit_task_for_review",
                "list_my_tasks",
            ] {
                toolkit.unregister(tool_name);
            }
        }
        if let Some(ref allowed_tool_names) = request.tools.allowed_tool_names {
            let allowed: HashSet<&str> = allowed_tool_names.iter().map(String::as_str).collect();
            for tool in toolkit.get_definitions() {
                if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                    if !allowed.contains(name) {
                        toolkit.unregister(name);
                    }
                }
            }
        }

        Ok(PreparedAgentParts { selection, toolkit })
    }

    fn build_react_agent(
        llm_router: Arc<macaca_llm::LlmRouter>,
        request: &AgentBuildRequest,
        selection: &macaca_llm::ModelSelection,
        toolkit: Toolkit,
        max_iters: usize,
    ) -> ReActAgent {
        let model = Arc::new(RoutedLlmAdapter::new(llm_router, selection.clone()));
        let formatter = Arc::new(OpenAiFormatter);
        ReActAgent::new(
            &request.identity.agent_name,
            &request.system_prompt,
            model,
            formatter,
        )
        .with_toolkit(toolkit)
        .with_memory(Box::new(InMemoryWorkingMemory::new()))
        .with_max_iters(max_iters)
        .with_model_name(selection.primary.reference())
    }

    async fn configure_standard_toolkit(
        state: Arc<AppState>,
        toolkit: &mut Toolkit,
        mode: &StandardAgentMode,
        task_id: macaca_proto::TaskId,
        agent_name: &str,
        session_id: Option<String>,
    ) {
        match mode {
            StandardAgentMode::Executor { executor } => {
                toolkit.add_middleware(Box::new(ExecutorToolMiddleware {
                    executor: Arc::clone(executor),
                    task_id,
                    agent_name: agent_name.to_string(),
                }));
                Self::attach_driver_trace_route(
                    toolkit,
                    DriverTraceRoute::Executor {
                        state,
                        executor: Arc::clone(executor),
                        task_id,
                        agent_name: agent_name.to_string(),
                        session_id,
                    },
                )
                .await;
            }
            StandardAgentMode::Runtime { event_tx } => {
                if let Some(ref agent_tx) = event_tx {
                    Self::attach_driver_trace_route(
                        toolkit,
                        DriverTraceRoute::Runtime {
                            tx: agent_tx.clone(),
                        },
                    )
                    .await;
                    toolkit.add_middleware(Box::new(ChannelToolMiddleware {
                        tx: agent_tx.clone(),
                    }));
                }
            }
        }
    }

    fn build_standard_hooks(
        mode: StandardAgentMode,
        task_id: macaca_proto::TaskId,
        agent_name: String,
    ) -> HookRegistry {
        let mut hooks = HookRegistry::new();
        match mode {
            StandardAgentMode::Executor { executor } => {
                hooks.register_instance_hook(Box::new(ExecutorEmitterHook {
                    executor,
                    task_id,
                    agent_name,
                    iteration: std::sync::atomic::AtomicUsize::new(0),
                }));
            }
            StandardAgentMode::Runtime { event_tx } => {
                if let Some(tx) = event_tx {
                    hooks.register_instance_hook(Box::new(ChannelEmitterHook {
                        tx,
                        iteration: std::sync::atomic::AtomicUsize::new(0),
                    }));
                }
            }
        }
        hooks
    }

    async fn build_standard_agent(
        &self,
        request: AgentBuildRequest,
        mode: StandardAgentMode,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let PreparedAgentParts {
            selection,
            mut toolkit,
        } = self.prepare_agent_parts(&request, None).await?;

        let task_id = request
            .trace
            .task_id
            .ok_or_else(|| "standard build request missing task_id".to_string())?;
        let session_id = request.identity.session_id.clone();
        let agent_name = request.identity.agent_name.clone();
        let llm_router = Arc::clone(&self.state.llm_router);

        Self::configure_standard_toolkit(
            Arc::clone(&self.state),
            &mut toolkit,
            &mode,
            task_id,
            &agent_name,
            session_id,
        )
        .await;

        let agent = Self::build_react_agent(llm_router, &request, &selection, toolkit, 25);
        let hooks = Self::build_standard_hooks(mode, task_id, agent_name);
        Ok(HookedAgent::new(agent, hooks))
    }

    async fn attach_driver_trace_route(toolkit: &mut Toolkit, route: DriverTraceRoute) {
        let (trace_tx, mut trace_rx) =
            tokio::sync::mpsc::unbounded_channel::<macaca_tools::TraceEvent>();
        toolkit.set_event_tx(trace_tx);

        tokio::spawn(async move {
            while let Some(trace) = trace_rx.recv().await {
                let driver_name = trace
                    .driver_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let trace_value = serde_json::to_value(&trace).unwrap_or_default();

                match &route {
                    DriverTraceRoute::Executor {
                        state,
                        executor,
                        task_id,
                        agent_name,
                        session_id,
                    } => {
                        let delegated_envelope = serde_json::json!({
                            "task_id": task_id.to_string(),
                            "agent": agent_name,
                            "agent_tab": agent_name,
                            "driver_name": driver_name,
                            "event": trace_value,
                        });

                        if let Some(sid) = session_id {
                            let sender_opt = {
                                let sessions = state.sessions.active_sessions.read().await;
                                sessions.get(sid).map(|session| Arc::clone(&session.sse_tx))
                            };
                            if let Some(sender) = sender_opt {
                                let event = Event::default()
                                    .event("delegated_driver_trace")
                                    .data(delegated_envelope.to_string());
                                let tx = sender.read().await;
                                let _ = tx.send(Ok(event)).await;
                            }
                        }

                        executor.broadcast_event(
                            macaca_kernel::executor::ExecutorEvent::AgentEvent {
                                task_id: *task_id,
                                agent: agent_name.clone(),
                                event: macaca_proto::AgentExecutionEvent::DriverTrace {
                                    driver_name: driver_name.clone(),
                                    trace: trace_value.clone(),
                                },
                            },
                        );
                    }
                    DriverTraceRoute::Runtime { tx } => {
                        let _ = tx
                            .send(macaca_proto::AgentExecutionEvent::DriverTrace {
                                driver_name: driver_name.clone(),
                                trace: trace_value.clone(),
                            })
                            .await;
                    }
                    DriverTraceRoute::Coordinator {
                        tx,
                        event_log,
                        agent_name,
                        session_id,
                    } => {
                        if let Some(sid) = session_id {
                            event_log
                                .append_command(AppendEventCommand::new(
                                    sid,
                                    "driver_trace",
                                    agent_name,
                                    trace_value.clone(),
                                ))
                                .await;
                        }
                        let event = Event::default().event("driver_trace").data(
                            serde_json::json!({
                                "driver_name": driver_name,
                                "event": trace_value,
                            })
                            .to_string(),
                        );
                        let _ = tx.send(Ok(event)).await;
                    }
                }
            }
        });
    }

    async fn build_executor_agent(
        &self,
        request: AgentBuildRequest,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        self.build_standard_agent(request, StandardAgentMode::Executor { executor })
            .await
    }

    async fn build_runtime_agent(
        &self,
        request: AgentBuildRequest,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        self.build_standard_agent(request, StandardAgentMode::Runtime { event_tx })
            .await
    }

    async fn build_coordinator(
        self,
        request: AgentBuildRequest,
    ) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
        let llm_router = Arc::clone(&self.state.llm_router);
        let PreparedAgentParts {
            selection,
            mut toolkit,
        } = self.prepare_agent_parts(&request, Some(None)).await?;

        let FrameworkRunnerBuildMode::Coordinator {
            sse_tx,
            pause_signal,
            resume_rx,
        } = self.build_mode
        else {
            return Err("invalid build mode for coordinator".into());
        };

        Self::attach_driver_trace_route(
            &mut toolkit,
            DriverTraceRoute::Coordinator {
                tx: sse_tx.clone(),
                event_log: Arc::clone(&self.state.persist.event_log),
                agent_name: request.identity.agent_name.clone(),
                session_id: request.identity.session_id.clone(),
            },
        )
        .await;

        toolkit.add_middleware(Box::new(SseToolMiddleware {
            tx: sse_tx.clone(),
            agent_name: request.identity.agent_name.clone(),
            event_log: Some(Arc::clone(&self.state.persist.event_log)),
            session_id: request.identity.session_id.clone(),
        }));
        toolkit.add_middleware(Box::new(PauseOnGoalMiddleware {
            pause_signal,
            resume_rx: Arc::new(Mutex::new(resume_rx)),
        }));

        let agent = Self::build_react_agent(llm_router, &request, &selection, toolkit, 50);

        let cancel_token = agent.cancel_token();
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(SseEmitterHook {
            tx: sse_tx,
            agent_name: request.identity.agent_name,
            event_log: Some(Arc::clone(&self.state.persist.event_log)),
            session_id: request.identity.session_id,
        }));
        Ok((HookedAgent::new(agent, hooks), cancel_token))
    }
}

async fn resolve_agent_skill_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> SkillPolicy {
    let registry = state.registry.read().await;
    let Some(app) = registry.get_app(app_id) else {
        return SkillPolicy::default();
    };
    for source in &app.manifest.agents {
        let macaca_app::model::AgentSource::Inline(inline) = source else {
            continue;
        };
        if inline.name != agent_name {
            continue;
        }
        let Some(skills) = &inline.skills else {
            return SkillPolicy::default();
        };
        return SkillPolicy {
            allow: skills.allow.clone(),
            deny: skills.deny.clone(),
        };
    }
    SkillPolicy::default()
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
                .append_command(AppendEventCommand::new(
                    session_id,
                    "thinking",
                    "coordinator",
                    serde_json::json!({
                        "iteration": 0,
                    }),
                ))
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
                .append_command(AppendEventCommand::new(
                    session_id,
                    "content",
                    "coordinator",
                    serde_json::json!({
                        "content": text,
                    }),
                ))
                .await;
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "done",
                    "coordinator",
                    serde_json::json!({
                        "model": "",
                        "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                        "iterations": 0,
                        "tools_used": [],
                    }),
                ))
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
                .append_command(AppendEventCommand::new(
                    session_id,
                    "tool_call",
                    &self.agent_name,
                    serde_json::json!({
                        "tool_name": name,
                        "tool_input": args.clone(),
                    }),
                ))
                .await;
            if name == "file_read" {
                if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
                    if path.ends_with("SKILL.md") && path.contains("/skills/") {
                        event_log
                            .append_command(AppendEventCommand::new(
                                session_id,
                                "skill_file_read",
                                &self.agent_name,
                                serde_json::json!({
                                    "agent": self.agent_name,
                                    "path": path,
                                }),
                            ))
                            .await;
                    }
                }
            }
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
                .append_command(AppendEventCommand::new(
                    session_id,
                    "tool_result",
                    &self.agent_name,
                    serde_json::json!({
                        "tool_name": name,
                        "output": display_result,
                    }),
                ))
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
