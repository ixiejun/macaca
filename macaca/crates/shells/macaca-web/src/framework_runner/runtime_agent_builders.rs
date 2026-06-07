//! Runtime agent builders for service-backed execution paths.

use std::sync::Arc;
use macaca_framework::agent::HookedAgent;
use macaca_framework::construction::{AgentBuildIntent, AgentToolConfig, TracedAgentFactory};
use macaca_framework::model::ToolChoice;
use macaca_framework::react_agent::ReActAgent;
use macaca_proto::ApplicationId;
use tokio::sync::mpsc;
use crate::state::AppState;
use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;
use super::request_composition;
use super::runtime_execution_control::RuntimeExecutionControl;
use super::FrameworkRunner;

impl FrameworkRunner {
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
        let request = request_composition::build_request(
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
        let request = request_composition::build_request_with_system_prompt(
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
        let request = request_composition::build_request_with_system_prompt(
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
        let request = request_composition::build_request_with_system_prompt(
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
}
