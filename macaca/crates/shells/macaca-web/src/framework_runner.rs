//! Framework-based agent runner — builds `ReActAgent` instances from persona
//! configuration and bridges events to SSE.
//!
//! This module replaces the ad-hoc `AgenticLoop` execution with the
//! `macaca-framework` `ReActAgent`, providing:
//! - Unified tool management via `Toolkit` (with middleware chain)
//! - Working memory with tag-based filtering
//! - Hook system for SSE event bridging
//! - Policy-selected execution control via `ToolMiddleware` barriers

use std::collections::HashSet;
use std::convert::Infallible;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::response::sse::Event;
use tokio::sync::{mpsc, Mutex};

use macaca_agent::{AgentCapabilitySet, AgentServices, AgentTransitionReason};
use macaca_app::model::AppContextConfig;
use macaca_app::{app_agent_manifest_view, app_agent_prompt_semantics};
use macaca_context::{
    ContextSourceKind, PromptComposer, PromptSection, PromptStability, TrustLevel,
};
use macaca_framework::adapter::ServiceChatModelAdapter;
use macaca_framework::agent::{Hook, HookRegistry, HookedAgent};
use macaca_framework::construction::{
    AgentBuildIntent, AgentBuildRequest, AgentBuildRequestBuilder, AgentIdentity,
    AgentLifecycleConfig, AgentToolConfig, AgentTraceContext, ApplicationPromptParts,
    ApplicationSemantics, ApplicationToolPolicy, TracedAgentFactory,
};
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::InMemoryWorkingMemory;
use macaca_framework::message::Msg;
use macaca_framework::model::ToolChoice;
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::tool::{ToolError, ToolMiddleware, ToolResponse, Toolkit};
use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::config::{
    AgentProfileContextConfig, AgentProfileRootKind, ContextConfig, WorkspaceGuideSourcesConfig,
};
use macaca_proto::{
    AgentId, AgentState, ApplicationId, Capability, ExecutionControlPolicy, ExecutionControlTrigger,
};
use macaca_sdk::AgentPersona;
use macaca_skill::SkillPolicy;

use crate::context_reporting_model::ContextReportingChatModel;
use crate::runtime_resume::RuntimeResumeSignal;
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
        execution_control: Option<RuntimeExecutionControl>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    },
    Coordinator {
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        execution_control: RuntimeExecutionControl,
    },
}

/// Runtime-agent pause/resume wiring selected by execution-control policy.
///
/// The web host owns the local session channel, but the pause trigger comes from
/// provider-neutral execution-control policy. This keeps runtime execution
/// generic while preserving the current in-process resume channel until
/// `service.execution_control` owns the state machine.
#[derive(Clone)]
pub(crate) struct RuntimeExecutionControl {
    pause_signal: Arc<AtomicBool>,
    resume_rx: Arc<Mutex<mpsc::Receiver<RuntimeResumeSignal>>>,
    policy: ExecutionControlPolicy,
    execution_id: String,
}

impl RuntimeExecutionControl {
    /// Build an in-process execution-control handle from the session-level pause
    /// signal and the receiver completed by the selected resume source.
    pub(crate) fn new(
        pause_signal: Arc<AtomicBool>,
        resume_rx: mpsc::Receiver<RuntimeResumeSignal>,
        policy: ExecutionControlPolicy,
        execution_id: impl Into<String>,
    ) -> Self {
        Self {
            pause_signal,
            resume_rx: Arc::new(Mutex::new(resume_rx)),
            policy,
            execution_id: execution_id.into(),
        }
    }
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
        execution_control: Option<RuntimeExecutionControl>,
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

impl DriverTraceRoute {
    /// Return a bounded route label for structured diagnostics.
    ///
    /// The label is intentionally static and provider-neutral. It is used only
    /// when trace routing suppresses a framework wrapper event that is already
    /// represented by a semantic agent-execution tool event.
    fn label(&self) -> &'static str {
        match self {
            DriverTraceRoute::Executor { .. } => "executor",
            DriverTraceRoute::Runtime { .. } => "runtime",
            DriverTraceRoute::Coordinator { .. } => "coordinator",
        }
    }
}

/// Return whether a tool trace is only the framework's generic wrapper event.
///
/// Macaca emits semantic `AgentExecutionEvent::ToolCall` and
/// `AgentExecutionEvent::ToolResult` events at the agent-execution layer. The
/// lower tool-command pipeline also emits `TraceEvent` values for the same
/// call/result lifecycle when a concrete driver does not provide its own trace
/// identity. Those no-driver wrapper traces are useful as a fallback in raw
/// tool pipelines, but forwarding them as `DriverTrace` inside agent execution
/// would persist the same logical operation twice. Concrete driver/provider
/// traces still pass through because they carry a real `driver_id` or a richer
/// diagnostic event type.
fn is_framework_tool_wrapper_trace(trace: &macaca_tools::TraceEvent) -> bool {
    trace.driver_id.is_none() && matches!(trace.event_type.as_str(), "tool_call" | "tool_result")
}

/// Decide whether a trace event should be forwarded as a driver trace.
///
/// This small Specification keeps Executor, Runtime, and Coordinator routing in
/// sync. It avoids frontend-only hiding and keeps EventLog replay faithful:
/// semantic tool events remain durable, while redundant framework wrappers are
/// suppressed before they become driver-trace events.
fn should_forward_driver_trace(trace: &macaca_tools::TraceEvent) -> bool {
    !is_framework_tool_wrapper_trace(trace)
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
            FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control,
                max_iters,
                tool_choice,
            } => {
                self.build_runtime_agent(
                    request,
                    event_tx.clone(),
                    execution_control.clone(),
                    *max_iters,
                    tool_choice.clone(),
                )
                .await
            }
            FrameworkRunnerBuildMode::Coordinator { .. } => {
                Err("Coordinator construction requires owned channels".into())
            }
        }
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
            build_mode: FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control: None,
                max_iters: 25,
                tool_choice: None,
            },
        };
        factory.build(request).await
    }

    /// Build a runtime agent from an Agent Context service snapshot.
    ///
    /// This path is used by `service.agent_execution` after it has already
    /// called `service.agent_context`.  It intentionally does not call
    /// `build_context_system_prompt` again, so persona, skill snapshot, tool
    /// policy, and workspace context have exactly one service-owned source of
    /// truth for the execution.
    pub(crate) async fn build_runtime_agent_from_context_snapshot(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_runtime_agent_from_context_snapshot_with_max_iters(
            state,
            context_snapshot,
            event_tx,
            25,
        )
        .await
    }

    /// Build a runtime agent from an Agent Context snapshot with a caller-owned
    /// ReAct iteration budget.
    ///
    /// Some Agent Execution callers already have a deterministic completion
    /// policy, such as "the expected artifact must change". Those callers can
    /// choose a short loop budget and let the evidence gate decide completion
    /// instead of waiting for a long natural-language wrap-up loop.
    pub(crate) async fn build_runtime_agent_from_context_snapshot_with_max_iters(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        max_iters: usize,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let task_id = context_snapshot
            .task_id
            .unwrap_or_else(macaca_proto::TaskId::new);
        let capabilities = Self::resolve_agent_capability_set(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
        )
        .await;
        let request = Self::build_request_with_system_prompt(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
            Some(context_snapshot.session_id.clone()),
            task_id,
            context_snapshot.task_id,
            AgentBuildIntent::RuntimeAgent,
            AgentToolConfig {
                goal_id: context_snapshot.task_id,
                ..Default::default()
            },
            capabilities,
            context_snapshot.system_prompt.clone(),
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control: None,
                max_iters: max_iters.max(1),
                tool_choice: None,
            },
        };
        factory.build(request).await
    }

    /// Build a runtime agent from an Agent Context snapshot and attach the
    /// execution-control middleware selected for this run.
    ///
    /// This keeps the main-thread coordinator on the serviceized execution path
    /// while preserving the goal/task contract: after `create_goal`, the entry
    /// agent waits for PlanLoop/WorkerLoop to finish instead of continuing to
    /// call implementation tools itself.
    pub(crate) async fn build_runtime_agent_from_context_snapshot_with_execution_control(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: RuntimeExecutionControl,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_runtime_agent_from_context_snapshot_with_execution_control_and_max_iters(
            state,
            context_snapshot,
            event_tx,
            execution_control,
            25,
        )
        .await
    }

    /// Build a runtime agent with execution-control middleware and a
    /// caller-owned ReAct iteration budget.
    pub(crate) async fn build_runtime_agent_from_context_snapshot_with_execution_control_and_max_iters(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: RuntimeExecutionControl,
        max_iters: usize,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let task_id = context_snapshot
            .task_id
            .unwrap_or_else(macaca_proto::TaskId::new);
        let capabilities = Self::resolve_agent_capability_set(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
        )
        .await;
        let request = Self::build_request_with_system_prompt(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
            Some(context_snapshot.session_id.clone()),
            task_id,
            context_snapshot.task_id,
            AgentBuildIntent::RuntimeAgent,
            AgentToolConfig {
                goal_id: context_snapshot.task_id,
                ..Default::default()
            },
            capabilities,
            context_snapshot.system_prompt.clone(),
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control: Some(execution_control),
                max_iters: max_iters.max(1),
                tool_choice: None,
            },
        };
        factory.build(request).await
    }

    /// Build a runtime agent with a service-selected execution policy.
    ///
    /// The policy arguments are deliberately generic: `max_iters` controls the
    /// ReAct loop budget and `tool_choice` controls whether the provider should
    /// be nudged or required to use the authorized Toolkit. The caller decides
    /// this from provider-neutral execution contracts, not from app-specific
    /// names, so YAML apps, WASM apps, and future gateway apps can share the
    /// same behavior.
    pub(crate) async fn build_runtime_agent_from_context_snapshot_with_execution_policy(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: Option<RuntimeExecutionControl>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let task_id = context_snapshot
            .task_id
            .unwrap_or_else(macaca_proto::TaskId::new);
        let capabilities = Self::resolve_agent_capability_set(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
        )
        .await;
        let request = Self::build_request_with_system_prompt(
            state,
            &context_snapshot.application_id,
            &context_snapshot.target_agent,
            Some(context_snapshot.session_id.clone()),
            task_id,
            context_snapshot.task_id,
            AgentBuildIntent::RuntimeAgent,
            AgentToolConfig {
                goal_id: context_snapshot.task_id,
                ..Default::default()
            },
            capabilities,
            context_snapshot.system_prompt.clone(),
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control,
                max_iters: max_iters.max(1),
                tool_choice,
            },
        };
        factory.build(request).await
    }

    /// Build a replayable system-context snapshot through the same Web
    /// composition path used by framework-native agents.
    ///
    /// Serviceized callers use this as the concrete backend for
    /// `service.agent_context`, which prevents WASM, YAML, SDK, and future
    /// gateway execution paths from constructing persona/skill/tool context
    /// through private shortcuts.
    pub(crate) async fn build_agent_context_snapshot(
        state: &Arc<AppState>,
        command: macaca_proto::AgentContextBuildCommand,
    ) -> macaca_proto::AgentContextSnapshot {
        let capabilities = Self::resolve_agent_capability_set(
            state,
            &command.application_id,
            &command.target_agent,
        )
        .await;
        let merged_context = FrameworkRunner::resolve_context_config(
            state,
            &command.application_id,
            &command.target_agent,
        )
        .await;
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == command.application_id)
                .map(|(_, path)| path.clone())
        };
        let workspace_root = {
            let workspaces = state.config.app_workspaces.read().await;
            workspaces
                .get(&command.application_id)
                .map(|ws| ws.root.clone())
        };
        let system_prompt = Self::build_context_system_prompt(
            state,
            &command.application_id,
            &command.target_agent,
            Some(command.session_id.clone()),
            &capabilities,
        )
        .await;
        let mut snapshot = macaca_proto::AgentContextSnapshot::minimal(&command, system_prompt);
        snapshot.sources.push(macaca_proto::AgentContextSource {
            kind: "persona_or_manifest".into(),
            name: command.target_agent.clone(),
            location: app_dir.as_ref().map(|dir| dir.display().to_string()),
            metadata: std::collections::BTreeMap::from([
                (
                    "agent_profile_enabled".into(),
                    merged_context.agent_profile.enabled.to_string(),
                ),
                (
                    "workspace_guide_entries".into(),
                    merged_context.workspace_guides.entries.len().to_string(),
                ),
            ]),
        });
        if merged_context.agent_profile.enabled && merged_context.agent_profile.inject_heartbeat {
            if let Some(profile_root) = Self::resolve_agent_profile_root(
                state,
                &command.application_id,
                &command.target_agent,
                &merged_context.agent_profile,
            )
            .await
            {
                let heartbeat_path = profile_root.join("HEARTBEAT.md");
                if heartbeat_path.is_file() {
                    // Record source evidence for heartbeat-intent execution.
                    // The Agent Context composer remains responsible for the
                    // actual profile-file content injection. This evidence row
                    // gives Agent Execution a fail-closed, auditable proof that
                    // HEARTBEAT.md participated in the trusted profile source
                    // set without reading or duplicating prompt text here.
                    snapshot.sources.push(macaca_proto::AgentContextSource {
                        kind: "profile_file".into(),
                        name: "HEARTBEAT.md".into(),
                        location: Some(heartbeat_path.display().to_string()),
                        metadata: std::collections::BTreeMap::from([(
                            "source_owner".into(),
                            "agent_context".into(),
                        )]),
                    });
                }
            }
        }
        snapshot.sources.push(macaca_proto::AgentContextSource {
            kind: "tool_policy".into(),
            name: command.target_agent.clone(),
            location: None,
            metadata: command.policy.metadata.clone(),
        });
        snapshot.tool_policy.insert(
            "capability_scope".into(),
            serde_json::to_string(&command.policy.capability_scope).unwrap_or_else(|_| "[]".into()),
        );
        snapshot.tool_policy.insert(
            "required_permissions".into(),
            serde_json::to_string(&command.policy.required_permissions)
                .unwrap_or_else(|_| "[]".into()),
        );
        let skill_policy =
            resolve_agent_skill_policy(state, &command.application_id, &command.target_agent).await;
        match crate::capability_catalog::resolve_skill_snapshot_cached(
            state,
            &command.application_id,
            &command.target_agent,
            Some(&command.session_id),
            skill_policy,
            workspace_root,
            app_dir.clone(),
        )
        .await
        {
            Ok(skill_snapshot) => {
                snapshot.visible_skills = skill_snapshot
                    .skills
                    .iter()
                    .map(|skill| skill.name.clone())
                    .collect();
                snapshot.filtered_skills = skill_snapshot
                    .filtered
                    .iter()
                    .map(|filtered| filtered.name.clone())
                    .collect();
                snapshot.sources.push(macaca_proto::AgentContextSource {
                    kind: "skill_snapshot".into(),
                    name: format!("{} skills", command.target_agent),
                    location: None,
                    metadata: std::collections::BTreeMap::from([
                        ("version".into(), skill_snapshot.version.to_string()),
                        ("truncated".into(), skill_snapshot.truncated.to_string()),
                        ("compact".into(), skill_snapshot.compact.to_string()),
                    ]),
                });
            }
            Err(error) => {
                snapshot.sources.push(macaca_proto::AgentContextSource {
                    kind: "skill_snapshot_unavailable".into(),
                    name: command.target_agent.clone(),
                    location: None,
                    metadata: std::collections::BTreeMap::from([(
                        "error".into(),
                        error.to_string(),
                    )]),
                });
            }
        }
        snapshot.metadata.insert(
            "provider".into(),
            "macaca.web.framework_runner.context".into(),
        );
        snapshot.metadata.insert(
            "execution_intent".into(),
            serde_json::to_string(&command.execution_intent).unwrap_or_else(|_| "unknown".into()),
        );
        state
            .persist
            .event_log
            .append_command(AppendEventCommand::new(
                &command.session_id,
                "agent_context_built",
                &command.target_agent,
                serde_json::json!({
                    "agent": command.target_agent,
                    "task_id": command.task_id.map(|id| id.0.to_string()),
                    "execution_intent": command.execution_intent,
                    "trace_id": command.trace.trace_id,
                    "system_prompt_chars": snapshot.system_prompt.chars().count(),
                    "visible_skill_count": snapshot.visible_skills.len(),
                    "filtered_skill_count": snapshot.filtered_skills.len(),
                    "source_count": snapshot.sources.len(),
                    "provider": snapshot.metadata.get("provider").cloned(),
                }),
            ))
            .await;
        snapshot
    }

    /// Build a coordinator `ReActAgent` wrapped with `HookedAgent` for SSE bridging
    /// and policy-driven execution-control barriers.
    ///
    /// Returns `(HookedAgent<ReActAgent>, CancellationToken)`.
    pub(crate) async fn build_coordinator(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        execution_control: RuntimeExecutionControl,
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
                execution_control,
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
        let capabilities = Self::resolve_agent_capability_set(state, app_id, agent_name).await;
        let system_prompt = Self::build_context_system_prompt(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            &capabilities,
        )
        .await;
        Self::build_request_with_system_prompt(
            state,
            app_id,
            agent_name,
            session_id,
            task_id,
            goal_id,
            intent,
            tools,
            capabilities,
            system_prompt,
        )
        .await
    }

    async fn build_request_with_system_prompt(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        goal_id: Option<macaca_proto::TaskId>,
        intent: AgentBuildIntent,
        tools: AgentToolConfig,
        capabilities: AgentCapabilitySet,
        system_prompt: String,
    ) -> Result<AgentBuildRequest, String> {
        let app_manifest = {
            let registry = state.registry.read().await;
            registry.get_app(app_id).map(|app| app.manifest.clone())
        };
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
        session_id: Option<&str>,
    ) -> Result<macaca_llm::ModelSelection, String> {
        let request_model = if let Some(session_id) = session_id {
            state
                .sessions
                .llm_route_hints
                .read()
                .await
                .get(session_id)
                .cloned()
        } else {
            None
        };
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
                request_model: request_model.clone(),
                agent_model,
                app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
                app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
                system_model: (!state.config.default_model.is_empty())
                    .then_some(state.config.default_model.clone()),
                ..Default::default()
            })
            .inspect(|selection| {
                tracing::info!(
                    app_id = %app_id,
                    agent = agent_name,
                    session_id = session_id.unwrap_or("framework-sessionless"),
                    route_source = selection.source,
                    provider = %selection.primary.provider,
                    model = %selection.primary.model,
                    request_model_present = request_model.is_some(),
                    "framework runner resolved LLM model selection"
                );
            })
            .map_err(|e| e.to_string())
    }

    async fn resolve_context_config(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> ContextConfig {
        let mut config = state.config.context.clone();
        let registry = state.registry.read().await;
        if let Some(app) = registry.get_app(app_id) {
            let agent_engine = app_agent_manifest_view(&app.manifest, agent_name)
                .and_then(|agent| agent.context_engine().map(str::to_owned))
                .filter(|value| !value.is_empty());
            config = Self::merge_context_config_overrides(
                config,
                app.manifest.context.as_ref(),
                agent_engine.as_deref(),
            );
        }
        config
    }

    /// Merge system-level context defaults with optional app and agent overrides.
    ///
    /// Precedence is intentionally narrow and deterministic:
    /// 1. system config provides the base
    /// 2. app context may override default/fallback engine and supporting profile fields
    /// 3. agent context engine may override only the primary engine selection
    fn merge_context_config_overrides(
        mut config: ContextConfig,
        app_context: Option<&AppContextConfig>,
        agent_engine: Option<&str>,
    ) -> ContextConfig {
        if let Some(app_context) = app_context {
            if let Some(engine) = app_context
                .engine
                .as_ref()
                .filter(|value| !value.is_empty())
            {
                config.default_engine = engine.clone();
            }
            if let Some(fallback) = app_context
                .fallback_engine
                .as_ref()
                .filter(|value| !value.is_empty())
            {
                config.fallback_engine = fallback.clone();
            }
            if let Some(ref guides) = app_context.workspace_guides {
                config.workspace_guides = guides.clone();
            }
            if let Some(ref ap) = app_context.agent_profile {
                config.agent_profile = ap.clone();
            }
        }
        if let Some(engine) = agent_engine.filter(|value| !value.is_empty()) {
            config.default_engine = engine.to_string();
        }
        config
    }

    /// Resolves the on-disk directory scanned by [`macaca_context::ProfileFileContextProvider`].
    ///
    /// The path is never interpreted as a workflow name — only as filesystem layout dictated by
    /// [`AgentProfileRootKind`] and the active [`ApplicationId`].
    async fn resolve_agent_profile_root(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        cfg: &AgentProfileContextConfig,
    ) -> Option<std::path::PathBuf> {
        if !cfg.enabled {
            return None;
        }
        match cfg.root_kind {
            AgentProfileRootKind::PersonaDirectory => {
                let dirs = state.config.app_dirs.read().await;
                let app_dir = dirs
                    .iter()
                    .find(|(id, _)| **id == *app_id)
                    .map(|(_, path)| path.clone())?;
                Some(app_dir.join("personas").join(agent_name))
            }
            AgentProfileRootKind::AgentPrivateWorkspace => {
                let workspaces = state.config.app_workspaces.read().await;
                workspaces
                    .get(app_id)
                    .map(|ws| ws.agent_workspace(agent_name))
            }
        }
    }

    /// Load the agent's persona and build the system prompt through the context boundary.
    async fn build_context_system_prompt(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        capabilities: &AgentCapabilitySet,
    ) -> String {
        let merged_context =
            FrameworkRunner::resolve_context_config(state, app_id, agent_name).await;
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

        let mut composer = PromptComposer::new();

        let base_prompt = if let Some(ref p) = persona {
            if merged_context.agent_profile.enabled {
                p.to_system_prompt_delegating_profile_files(None)
            } else {
                p.to_system_prompt(None)
            }
        } else if let Some(manifest) = app_manifest.as_ref() {
            app_agent_prompt_semantics(manifest, agent_name).base_prompt
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };
        composer = composer.push_section(prompt_section(
            "000-base",
            ContextSourceKind::SystemPrompt,
            PromptStability::Stable,
            TrustLevel::Trusted,
            base_prompt,
        ));

        // Inject capabilities from the macaca-agent capability abstraction.
        let flattened = capabilities.flatten_for_legacy_api();
        let caps: Vec<&str> = flattened.iter().map(|cap| cap.name.as_str()).collect();
        if !caps.is_empty() {
            composer = composer.push_section(prompt_section(
                "100-capabilities",
                ContextSourceKind::SystemPrompt,
                PromptStability::Stable,
                TrustLevel::Trusted,
                format!("Your capabilities: {}", caps.join(", ")),
            ));
        }

        if let Some(ref dir) = app_dir {
            for section in
                load_workspace_guide_sections(dir, &merged_context.workspace_guides).await
            {
                composer = composer.push_section(section);
            }
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
                composer = composer.push_section(prompt_section(
                    "300-workspace-paths",
                    ContextSourceKind::Workspace,
                    PromptStability::Dynamic,
                    TrustLevel::Trusted,
                    format!(
                        "## Workspace Paths\n\
                         - Workspace root (default cwd for file/shell tools): {}\n\
                         - Shared workspace: {}\n\
                         - Your private workspace: {}\n\
                         Relative paths are resolved from the workspace root above. \
                         Create project files in the shared workspace. \
                         Use your private workspace for temporary/scratch files only.",
                        ws.root.display(),
                        ws.shared.display(),
                        ws.agent_workspace(agent_name).display(),
                    ),
                ));
            }
        }

        // Skill discovery telemetry + session cache (Tier-1 progressive disclosure now flows through
        // context composer providers; do not duplicate `snapshot.prompt` here — see
        // `ContextReportingChatModel` capability providers / `capability_catalog`).
        match crate::capability_catalog::resolve_skill_snapshot_cached(
            state,
            app_id,
            agent_name,
            session_id.as_deref(),
            skill_policy,
            workspace_root,
            app_dir,
        )
        .await
        {
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
            }
            Err(error) => {
                tracing::warn!(agent = %agent_name, error = %error, "failed to build skill catalog");
            }
        }

        composer.compile().text
    }

    #[deprecated(
        note = "Use build_context_system_prompt through the context facade; kept for migration discovery."
    )]
    async fn build_system_prompt(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        capabilities: &AgentCapabilitySet,
    ) -> String {
        Self::build_context_system_prompt(state, app_id, agent_name, session_id, capabilities).await
    }
}

fn prompt_section(
    id: impl Into<String>,
    kind: ContextSourceKind,
    stability: PromptStability,
    trust_level: TrustLevel,
    content: impl Into<String>,
) -> PromptSection {
    PromptSection {
        id: id.into(),
        kind,
        stability,
        trust_level,
        content: content.into(),
    }
}

async fn load_workspace_guide_sections(
    app_dir: &Path,
    guides: &WorkspaceGuideSourcesConfig,
) -> Vec<PromptSection> {
    #[derive(Clone)]
    struct EntryRef<'a> {
        priority: i32,
        path: &'a str,
        max_bytes: u32,
    }
    let mut ordered: Vec<EntryRef<'_>> = guides
        .entries
        .iter()
        .map(|e| EntryRef {
            priority: e.priority,
            path: e.relative_path.as_str(),
            max_bytes: e.max_bytes.max(512),
        })
        .collect();
    ordered.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.path.cmp(b.path)));
    let mut sections = Vec::new();
    for (seq, entry) in ordered.into_iter().enumerate() {
        let path = app_dir.join(entry.path);
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let content = content.trim();
        if content.is_empty() {
            continue;
        }
        let label = entry.path.trim_end_matches(".md").replace('_', " ");
        sections.push(prompt_section(
            format!("200-guide-{seq:03}-{}", entry.path.replace('/', "__")),
            ContextSourceKind::Workspace,
            PromptStability::Stable,
            TrustLevel::Trusted,
            format!(
                "## {label}\n\n{}",
                bounded_guide_content(content, entry.max_bytes as usize)
            ),
        ));
    }
    sections
}

fn bounded_guide_content(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    let excerpt = content
        .chars()
        .scan(0usize, |used, ch| {
            let len = ch.len_utf8();
            if *used + len > max_bytes {
                None
            } else {
                *used += len;
                Some(ch)
            }
        })
        .collect::<String>();
    format!("{excerpt}\n\n[workspace guide truncated at {max_bytes} bytes]")
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
            request.identity.session_id.as_deref(),
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

    /// Maps skill/MCP/toolkit runtime state into composer-ready capability DTOs (Adapter pattern).
    async fn resolve_framework_capability_catalogs(
        state: &Arc<AppState>,
        request: &AgentBuildRequest,
        toolkit: &Toolkit,
        context_config: &ContextConfig,
    ) -> (
        Arc<macaca_context::SkillCapabilityCatalog>,
        Arc<macaca_context::McpCapabilityCatalog>,
        Arc<macaca_context::RuntimeToolCapabilityCatalog>,
        Arc<Vec<String>>,
    ) {
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == request.identity.app_id)
                .map(|(_, path)| path.clone())
        };
        let workspace_root = {
            let workspaces = state.config.app_workspaces.read().await;
            workspaces
                .get(&request.identity.app_id)
                .map(|ws| ws.root.clone())
        };
        let skill_policy = resolve_agent_skill_policy(
            state,
            &request.identity.app_id,
            &request.identity.agent_name,
        )
        .await;
        let lifecycle_visibility =
            crate::capability_catalog::skill_lifecycle_visibility_from_context(context_config);
        let skill_cap = Arc::new(
            match crate::capability_catalog::resolve_skill_snapshot_cached(
                state,
                &request.identity.app_id,
                &request.identity.agent_name,
                request.identity.session_id.as_deref(),
                skill_policy,
                workspace_root,
                app_dir,
            )
            .await
            {
                Ok(snap) => {
                    if let Some(governance) =
                        crate::capability_catalog::resolve_skill_governance_snapshot(
                            state,
                            &request.identity.app_id,
                            &request.identity.agent_name,
                            request.identity.session_id.as_deref(),
                        )
                        .await
                    {
                        tracing::info!(
                            agent = %request.identity.agent_name,
                            visibility_profile = lifecycle_visibility.profile_label(),
                            "skill lifecycle visibility profile selected for context catalog"
                        );
                        crate::capability_catalog::skill_capability_catalog_from_governance_snapshot_with_visibility(
                            &snap,
                            &governance,
                            &lifecycle_visibility,
                        )
                    } else {
                        crate::capability_catalog::skill_capability_catalog_from_snapshot(&snap)
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        agent = %request.identity.agent_name,
                        error = %error,
                        "skill snapshot failed for capability catalogs; composing empty skill index"
                    );
                    macaca_context::SkillCapabilityCatalog::default()
                }
            },
        );
        let mcp_policy = macaca_runtime_host::McpToolPolicy::default();
        let (mcp_cat, ready) =
            crate::capability_catalog::probe_mcp_capability_inputs(&state.mcp_runtime, &mcp_policy)
                .await;
        let rt_cap = Arc::new(
            crate::capability_catalog::runtime_tool_capability_catalog_from_toolkit(toolkit),
        );
        (skill_cap, Arc::new(mcp_cat), rt_cap, Arc::new(ready))
    }

    /// Builds a [`ReActAgent`] with [`ContextReportingChatModel`]: kernel-backed `routing_agent_id`,
    /// composer capability providers (skills/MCP/runtime tools), and vector recall when enabled.
    fn build_react_agent(
        llm_client: Arc<dyn macaca_sdk::SystemLlmClient>,
        context_client: Arc<dyn macaca_sdk::SystemContextClient>,
        memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
        event_log: Arc<EventLog>,
        persist_backend: Arc<dyn macaca_persist::PersistBackend>,
        workspace_memory_tombstones: Option<Arc<macaca_memory::SharedTombstoneRegistry>>,
        merged_context_config: ContextConfig,
        agent_profile_root: Option<std::path::PathBuf>,
        request: &AgentBuildRequest,
        selection: &macaca_llm::ModelSelection,
        toolkit: Toolkit,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
        routing_agent_id: Option<AgentId>,
        skill_capability_catalog: Arc<macaca_context::SkillCapabilityCatalog>,
        mcp_capability_catalog: Arc<macaca_context::McpCapabilityCatalog>,
        runtime_tool_capability_catalog: Arc<macaca_context::RuntimeToolCapabilityCatalog>,
        ready_mcp_server_ids: Arc<Vec<String>>,
        provider_health_ledger: Option<Arc<macaca_context::ProviderHealthLedger>>,
        context_engine_registry: Arc<macaca_context::ContextEngineRegistry>,
    ) -> ReActAgent {
        let llm_scope = macaca_llm::LlmServiceScope::new(
            request.identity.app_id,
            request
                .identity
                .session_id
                .clone()
                .unwrap_or_else(|| "framework-sessionless".into()),
            request.identity.agent_name.clone(),
        )
        .expect("framework runner builds agents only after request identity validation");
        let model = Arc::new(ContextReportingChatModel::new(
            Arc::new(ServiceChatModelAdapter::new(llm_client, llm_scope)),
            event_log,
            persist_backend,
            request.identity.app_id,
            request.identity.session_id.clone(),
            request.identity.agent_name.clone(),
            merged_context_config,
            agent_profile_root,
            context_client,
            memory_client,
            workspace_memory_tombstones,
            routing_agent_id,
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
            provider_health_ledger,
            context_engine_registry,
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut agent = ReActAgent::new(
            &request.identity.agent_name,
            &request.system_prompt,
            model,
            formatter,
        )
        .with_toolkit(toolkit)
        .with_memory(Box::new(InMemoryWorkingMemory::new()))
        .with_max_iters(max_iters)
        .with_model_name(selection.primary.reference());
        if let Some(tool_choice) = tool_choice {
            agent = agent.with_tool_choice(tool_choice);
        }
        agent
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
            StandardAgentMode::Runtime {
                event_tx,
                execution_control,
            } => {
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
                if let Some(execution_control) = execution_control {
                    toolkit.add_middleware(Box::new(ExecutionControlMiddleware {
                        pause_signal: Arc::clone(&execution_control.pause_signal),
                        resume_rx: Arc::clone(&execution_control.resume_rx),
                        policy: execution_control.policy.clone(),
                        execution_id: execution_control.execution_id.clone(),
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
            StandardAgentMode::Runtime { event_tx, .. } => {
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
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
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
        let llm_client = Arc::clone(&self.state.llm_client);
        let context_client = Arc::clone(&self.state.context_client);

        Self::configure_standard_toolkit(
            Arc::clone(&self.state),
            &mut toolkit,
            &mode,
            task_id,
            &agent_name,
            session_id,
        )
        .await;

        let merged_ctx = FrameworkRunner::resolve_context_config(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
        )
        .await;
        let profile_root = FrameworkRunner::resolve_agent_profile_root(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
            &merged_ctx.agent_profile,
        )
        .await;
        // Align recall visibility with rows stored under this kernel manifest id (see `MemoryEntry::agent_id`).
        let routing_agent_id = self
            .state
            .kernel
            .get_agent_by_name(&request.identity.agent_name)
            .await
            .map(|m| m.id);
        let (
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
        ) = Self::resolve_framework_capability_catalogs(
            &self.state,
            &request,
            &toolkit,
            &merged_ctx,
        )
        .await;

        let agent = Self::build_react_agent(
            llm_client,
            context_client,
            Arc::clone(&self.state.memory_client),
            Arc::clone(&self.state.persist.event_log),
            Arc::clone(&self.state.persist.session_store),
            self.state.workspace_memory_tombstones.clone(),
            merged_ctx,
            profile_root,
            &request,
            &selection,
            toolkit,
            max_iters.max(1),
            tool_choice.clone(),
            routing_agent_id,
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
            Some(Arc::clone(&self.state.provider_health_ledger)),
            Arc::clone(&self.state.context_engine_registry),
        );
        let hooks = Self::build_standard_hooks(mode, task_id, agent_name);
        tracing::info!(
            application_id = %request.identity.app_id,
            agent = %request.identity.agent_name,
            session_id = request.identity.session_id.as_deref().unwrap_or(""),
            max_iters = max_iters.max(1),
            tool_choice = ?tool_choice,
            "framework_runner.standard_agent built with provider-neutral execution policy"
        );
        Ok(HookedAgent::new(agent, hooks))
    }

    async fn attach_driver_trace_route(toolkit: &mut Toolkit, route: DriverTraceRoute) {
        let (trace_tx, mut trace_rx) =
            tokio::sync::mpsc::unbounded_channel::<macaca_tools::TraceEvent>();
        toolkit.set_event_tx(trace_tx);

        tokio::spawn(async move {
            while let Some(trace) = trace_rx.recv().await {
                if !should_forward_driver_trace(&trace) {
                    tracing::debug!(
                        route = route.label(),
                        event_type = %trace.event_type,
                        tool_name = trace.tool_name.as_deref().unwrap_or(""),
                        correlation_id = trace.correlation_id.as_deref().unwrap_or(""),
                        "suppressed framework wrapper trace already represented by semantic tool event"
                    );
                    continue;
                }
                let driver_name = trace
                    .driver_id
                    .clone()
                    .unwrap_or_else(|| "macaca-framework".to_string());
                let trace_value = serde_json::to_value(&trace).unwrap_or_default();

                match &route {
                    DriverTraceRoute::Executor {
                        executor,
                        task_id,
                        agent_name,
                        ..
                    } => {
                        // Executor routes publish through the executor broadcast channel only.
                        // post_chat_v2 owns the SSE forwarding for that channel; sending here
                        // as well would duplicate delegated_driver_trace events in the live UI.
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
        self.build_standard_agent(request, StandardAgentMode::Executor { executor }, 25, None)
            .await
    }

    async fn build_runtime_agent(
        &self,
        request: AgentBuildRequest,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: Option<RuntimeExecutionControl>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        self.build_standard_agent(
            request,
            StandardAgentMode::Runtime {
                event_tx,
                execution_control,
            },
            max_iters.max(1),
            tool_choice,
        )
        .await
    }

    async fn build_coordinator(
        self,
        request: AgentBuildRequest,
    ) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
        let llm_client = Arc::clone(&self.state.llm_client);
        let context_client = Arc::clone(&self.state.context_client);
        let PreparedAgentParts {
            selection,
            mut toolkit,
        } = self.prepare_agent_parts(&request, Some(None)).await?;

        let FrameworkRunnerBuildMode::Coordinator {
            sse_tx,
            execution_control,
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
        toolkit.add_middleware(Box::new(ExecutionControlMiddleware {
            pause_signal: Arc::clone(&execution_control.pause_signal),
            resume_rx: Arc::clone(&execution_control.resume_rx),
            policy: execution_control.policy.clone(),
            execution_id: execution_control.execution_id.clone(),
        }));

        let merged_ctx = FrameworkRunner::resolve_context_config(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
        )
        .await;
        let profile_root = FrameworkRunner::resolve_agent_profile_root(
            &self.state,
            &request.identity.app_id,
            &request.identity.agent_name,
            &merged_ctx.agent_profile,
        )
        .await;
        let routing_agent_id = self
            .state
            .kernel
            .get_agent_by_name(&request.identity.agent_name)
            .await
            .map(|m| m.id);
        let (
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
        ) = Self::resolve_framework_capability_catalogs(
            &self.state,
            &request,
            &toolkit,
            &merged_ctx,
        )
        .await;

        let agent = Self::build_react_agent(
            llm_client,
            context_client,
            Arc::clone(&self.state.memory_client),
            Arc::clone(&self.state.persist.event_log),
            Arc::clone(&self.state.persist.session_store),
            self.state.workspace_memory_tombstones.clone(),
            merged_ctx,
            profile_root,
            &request,
            &selection,
            toolkit,
            50,
            None,
            routing_agent_id,
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
            Some(Arc::clone(&self.state.provider_health_ledger)),
            Arc::clone(&self.state.context_engine_registry),
        );

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
    use super::{
        is_framework_tool_wrapper_trace, should_forward_driver_trace, tool_response_text,
        truncate_tool_output, ExecutionControlMiddleware, FrameworkRunner,
    };
    use macaca_app::model::AppContextConfig;
    use macaca_framework::message::{ContentBlock, TextBlock};
    use macaca_framework::tool::ToolResponse;
    use macaca_proto::config::{AgentProfileContextConfig, ContextConfig};
    use macaca_tools::TraceEvent;

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

    #[test]
    fn production_code_does_not_call_deprecated_build_system_prompt_shim() {
        let source = include_str!("framework_runner.rs");
        let forbidden = concat!("Self::", "build_system_prompt(");
        assert!(
            !source.contains(forbidden),
            "production path should call build_context_system_prompt directly"
        );
    }

    #[test]
    fn agent_context_snapshot_records_replayable_skill_and_policy_evidence() {
        let source = include_str!("framework_runner.rs");

        assert!(source.contains("snapshot.visible_skills"));
        assert!(source.contains("snapshot.filtered_skills"));
        assert!(source.contains("skill_snapshot_unavailable"));
        assert!(source.contains("snapshot.tool_policy"));
        assert!(source.contains("agent_context_built"));
    }

    #[test]
    fn framework_tool_wrapper_trace_is_suppressed_without_driver_identity() {
        let call_trace = TraceEvent {
            event_type: "tool_call".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let result_trace = TraceEvent {
            event_type: "tool_result".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };

        assert!(is_framework_tool_wrapper_trace(&call_trace));
        assert!(is_framework_tool_wrapper_trace(&result_trace));
        assert!(!should_forward_driver_trace(&call_trace));
        assert!(!should_forward_driver_trace(&result_trace));
    }

    #[test]
    fn non_wrapper_driver_traces_are_preserved_for_diagnostics() {
        let no_driver_diagnostic = TraceEvent {
            event_type: "thinking".into(),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let concrete_driver_call = TraceEvent {
            event_type: "tool_call".into(),
            driver_id: Some("browser-driver".into()),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };
        let concrete_driver_result = TraceEvent {
            event_type: "tool_result".into(),
            driver_id: Some("browser-driver".into()),
            tool_name: Some("browser_run_code".into()),
            ..Default::default()
        };

        assert!(!is_framework_tool_wrapper_trace(&no_driver_diagnostic));
        assert!(should_forward_driver_trace(&no_driver_diagnostic));
        assert!(!is_framework_tool_wrapper_trace(&concrete_driver_call));
        assert!(should_forward_driver_trace(&concrete_driver_call));
        assert!(!is_framework_tool_wrapper_trace(&concrete_driver_result));
        assert!(should_forward_driver_trace(&concrete_driver_result));
    }

    #[test]
    fn runtime_driver_trace_route_uses_shared_wrapper_suppression() {
        let source = include_str!("framework_runner.rs");
        let attach_start = source
            .find("async fn attach_driver_trace_route")
            .expect("driver trace route attachment should exist");
        let attach_end = source[attach_start..]
            .find("fn build_executor_agent")
            .map(|offset| attach_start + offset)
            .expect("driver trace route attachment should end before executor builder");
        let attach_source = &source[attach_start..attach_end];
        let guard_position = attach_source
            .find("if !should_forward_driver_trace(&trace)")
            .expect("driver trace routing should use shared suppression guard");
        let runtime_position = attach_source
            .find("DriverTraceRoute::Runtime")
            .expect("runtime driver trace route should exist");

        assert!(
            guard_position < runtime_position,
            "runtime route must pass through the shared wrapper suppression guard before forwarding"
        );
        assert!(
            !attach_source.contains("framework_tool_wrapper"),
            "route-specific wrapper predicates would let Runtime drift from Executor again"
        );
    }

    #[test]
    fn context_config_precedence_agent_overrides_app_and_system_engine() {
        let mut base = ContextConfig::default();
        base.default_engine = "system-windowed".into();
        base.fallback_engine = "system-legacy".into();

        let merged = FrameworkRunner::merge_context_config_overrides(
            base,
            Some(&AppContextConfig {
                engine: Some("app-pruning".into()),
                fallback_engine: Some("app-summary".into()),
                workspace_guides: None,
                agent_profile: Some(AgentProfileContextConfig {
                    enabled: true,
                    ..Default::default()
                }),
            }),
            Some("agent-custom"),
        );

        assert_eq!(merged.default_engine, "agent-custom");
        assert_eq!(merged.fallback_engine, "app-summary");
        assert!(merged.agent_profile.enabled);
    }

    #[test]
    fn context_config_precedence_keeps_system_defaults_when_overrides_absent() {
        let mut base = ContextConfig::default();
        base.default_engine = "system-windowed".into();
        base.fallback_engine = "system-legacy".into();

        let merged = FrameworkRunner::merge_context_config_overrides(base.clone(), None, None);
        assert_eq!(merged.default_engine, base.default_engine);
        assert_eq!(merged.fallback_engine, base.fallback_engine);
    }

    #[test]
    fn execution_control_middleware_matches_configured_tool_barrier() {
        let policy = macaca_proto::ExecutionControlPolicy::enabled(
            vec![macaca_proto::ExecutionControlTrigger::tool_call_barrier(
                "create_goal",
            )],
            vec![macaca_proto::ExecutionControlResumeSource::goal_lifecycle()],
            macaca_proto::ExecutionControlCheckpointMode::ReferenceOnly,
        );

        assert!(ExecutionControlMiddleware::policy_pauses_after_tool(
            &policy,
            "create_goal"
        ));
        assert!(!ExecutionControlMiddleware::policy_pauses_after_tool(
            &policy,
            "claude_code_execute"
        ));
    }

    #[test]
    fn coordinator_execution_control_uses_supplied_policy() {
        let source = include_str!("framework_runner.rs");
        let coordinator_start = source
            .find("async fn build_coordinator")
            .expect("coordinator builder should exist");
        let coordinator_end = source[coordinator_start..]
            .find("let merged_ctx")
            .map(|offset| coordinator_start + offset)
            .expect("coordinator context section should exist");
        let coordinator_source = &source[coordinator_start..coordinator_end];

        assert!(coordinator_source.contains("policy: execution_control.policy.clone()"));
        assert!(!coordinator_source.contains("ExecutionControlPolicy::enabled("));
        assert!(!coordinator_source.contains("tool_call_barrier(\"create_goal\")"));
    }
}

// ---------------------------------------------------------------------------
// ExecutionControlMiddleware — pauses runtime executions at configured barriers
// ---------------------------------------------------------------------------

/// Middleware that blocks after policy-selected tool barriers until an approved
/// resume source delivers a signal through the current in-process channel.
///
/// This is the stage-1 adapter for execution control. It is intentionally
/// configured by `ExecutionControlPolicy`, not by application or agent names.
pub struct ExecutionControlMiddleware {
    pause_signal: Arc<AtomicBool>,
    resume_rx: Arc<Mutex<mpsc::Receiver<RuntimeResumeSignal>>>,
    policy: ExecutionControlPolicy,
    execution_id: String,
}

impl ExecutionControlMiddleware {
    /// Return whether the policy declares a tool-call barrier for this tool.
    fn policy_pauses_after_tool(policy: &ExecutionControlPolicy, tool_name: &str) -> bool {
        policy.triggers.iter().any(|trigger| {
            matches!(
                trigger,
                ExecutionControlTrigger::ToolCallBarrier { tool_name: configured }
                    if configured == tool_name
            )
        })
    }
}

#[async_trait]
impl ToolMiddleware for ExecutionControlMiddleware {
    async fn before(&self, _name: &str, _args: &mut serde_json::Value) -> Result<(), ToolError> {
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        if !Self::policy_pauses_after_tool(&self.policy, name) {
            return Ok(());
        }

        tracing::info!(
            execution_id = %self.execution_id,
            tool = %name,
            "execution control barrier reached; pausing runtime execution"
        );
        self.pause_signal.store(true, Ordering::SeqCst);

        // Wait for the configured resume source to complete.  Autonomous goals
        // and delegated work can legitimately run longer than a fixed HTTP-era
        // timeout; ending this wait early loses the paused execution.
        let mut rx = self.resume_rx.lock().await;
        match rx.recv().await {
            Some(reason) => {
                self.pause_signal.store(false, Ordering::SeqCst);
                let context = match &reason {
                    RuntimeResumeSignal::DelegateCompleted { output, .. } => output.clone(),
                    _ => "Goal processing completed.".to_string(),
                };
                response
                    .content
                    .push(macaca_framework::message::ContentBlock::Text(
                        macaca_framework::message::TextBlock {
                            text: format!("\n\n[Goal completed: {}]", context),
                        },
                    ));
                tracing::info!(
                    execution_id = %self.execution_id,
                    tool = %name,
                    "execution control resume signal delivered"
                );
            }
            None => {
                self.pause_signal.store(false, Ordering::SeqCst);
                tracing::warn!(
                    execution_id = %self.execution_id,
                    tool = %name,
                    "execution control resume channel closed"
                );
            }
        }
        Ok(())
    }
}
