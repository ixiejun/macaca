//! Standard agent construction: toolkit wiring, capability catalogs, ReAct assembly.

use std::collections::HashSet;
use std::sync::Arc;
use macaca_agent::{AgentServices, AgentTransitionReason};
use macaca_proto::AgentState;
use macaca_framework::adapter::ServiceChatModelAdapter;
use macaca_framework::agent::{HookRegistry, HookedAgent};
use macaca_framework::construction::{AgentBuildRequest, AgentLifecycleConfig};
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::InMemoryWorkingMemory;
use macaca_framework::model::ToolChoice;
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::tool::{ToolMiddleware, Toolkit};
use macaca_runtime_host::persist::EventLog;
use macaca_proto::config::ContextConfig;
use macaca_proto::AgentId;
use tokio::sync::mpsc;
use crate::context_reporting_model::ContextReportingChatModel;
use crate::state::AppState;
use super::FrameworkRunner;
use super::build_mode::{DriverTraceRoute, StandardAgentMode};
use super::channel_emitter_adapter::{ChannelEmitterHook, ChannelToolMiddleware};
use super::driver_trace_adapter::attach_driver_trace_route;
use super::execution_control_middleware::ExecutionControlMiddleware;
use super::executor_emitter_adapter::{ExecutorEmitterHook, ExecutorToolMiddleware};
use super::runtime_execution_control::RuntimeExecutionControl;
use super::skill_policy::resolve_agent_skill_policy;

pub(crate) struct WebTracedAgentFactory {
    pub(crate) state: Arc<AppState>,
    pub(crate) build_mode: super::build_mode::FrameworkRunnerBuildMode,
}

pub(crate) struct PreparedAgentParts {
    pub(crate) selection: macaca_sdk::llm::ModelSelection,
    pub(crate) toolkit: Toolkit,
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

    pub(crate) async fn prepare_agent_parts(
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
    pub(crate) async fn resolve_framework_capability_catalogs(
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
        let mcp_trace = macaca_proto::TraceContext::new(format!(
            "web-framework-mcp-capability-{}",
            request.identity.agent_name
        ));
        let (mcp_cat, ready) = crate::capability_catalog::probe_mcp_capability_inputs_via_client(
            &state.mcp_client,
            mcp_trace,
            &request.identity.agent_name,
        )
        .await;
        let rt_cap = Arc::new(
            crate::capability_catalog::runtime_tool_capability_catalog_from_toolkit(toolkit),
        );
        (skill_cap, Arc::new(mcp_cat), rt_cap, Arc::new(ready))
    }

    /// Builds a [`ReActAgent`] with [`ContextReportingChatModel`]: kernel-backed `routing_agent_id`,
    /// composer capability providers (skills/MCP/runtime tools), and vector recall when enabled.
    pub(crate) fn build_react_agent(
        llm_client: Arc<dyn macaca_sdk::SystemLlmClient>,
        context_client: Arc<dyn macaca_sdk::SystemContextClient>,
        memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
        event_log: Arc<EventLog>,
        persist_backend: Arc<dyn macaca_runtime_host::persist::PersistBackend>,
        workspace_memory_tombstones: Option<Arc<macaca_sdk::memory::SharedTombstoneRegistry>>,
        merged_context_config: ContextConfig,
        agent_profile_root: Option<std::path::PathBuf>,
        request: &AgentBuildRequest,
        selection: &macaca_sdk::llm::ModelSelection,
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
        let llm_scope = macaca_sdk::llm::LlmServiceScope::new(
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
                attach_driver_trace_route(
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
                    attach_driver_trace_route(
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
    pub(crate) async fn build_executor_agent(
        &self,
        request: AgentBuildRequest,
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        self.build_standard_agent(request, StandardAgentMode::Executor { executor }, 25, None)
            .await
    }

    pub(crate) async fn build_runtime_agent(
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
}
