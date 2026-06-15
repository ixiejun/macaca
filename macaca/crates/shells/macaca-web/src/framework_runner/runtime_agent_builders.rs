//! Runtime agent materializers for service-backed execution paths.
//!
//! Runtime-host owns framework agent construction. This Web module only
//! materializes the host-local `ReActAgent` instance behind the
//! `FrameworkAgentMaterializationPort` adapter so web tools, hooks, and local
//! notification handles can be attached without exposing construction ownership
//! to shell callers.

use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;
use super::request_composition;
use super::runtime_execution_control::RuntimeExecutionControl;
use super::FrameworkRunner;
use crate::state::AppState;
use macaca_host_composition::framework::agent::HookedAgent;
use macaca_host_composition::framework::construction::{
    AgentBuildIntent, AgentToolConfig, TracedAgentFactory,
};
use macaca_host_composition::framework::model::ToolChoice;
use macaca_host_composition::framework::react_agent::ReActAgent;
use std::sync::Arc;
use tokio::sync::mpsc;

impl FrameworkRunner {
    /// Materialize a runtime ReAct agent from an Agent Context service snapshot.
    ///
    /// This path is used by `service.agent_execution` after it has already
    /// called `service.agent_context`.  It intentionally does not call
    /// `build_context_system_prompt` again, so persona, skill snapshot, tool
    /// policy, and workspace context have exactly one service-owned source of
    /// truth for the execution.
    pub(crate) async fn materialize_runtime_react_agent_from_context_snapshot(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::materialize_runtime_react_agent_from_context_snapshot_with_max_iters(
            state,
            context_snapshot,
            event_tx,
            25,
        )
        .await
    }

    /// Materialize a runtime ReAct agent from an Agent Context snapshot with a
    /// service-selected iteration budget.
    pub(crate) async fn materialize_runtime_react_agent_from_context_snapshot_with_max_iters(
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

    /// Materialize a runtime ReAct agent from an Agent Context snapshot and
    /// attach the execution-control middleware selected for this run.
    pub(crate) async fn materialize_runtime_react_agent_from_context_snapshot_with_execution_control(
        state: &Arc<AppState>,
        context_snapshot: &macaca_proto::AgentContextSnapshot,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: RuntimeExecutionControl,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::materialize_runtime_react_agent_from_context_snapshot_with_execution_control_and_max_iters(
            state,
            context_snapshot,
            event_tx,
            execution_control,
            25,
        )
        .await
    }

    /// Materialize a runtime ReAct agent with execution-control middleware and
    /// a service-selected ReAct iteration budget.
    pub(crate) async fn materialize_runtime_react_agent_from_context_snapshot_with_execution_control_and_max_iters(
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

    /// Materialize a runtime ReAct agent with a service-selected execution policy.
    ///
    /// This helper is intentionally named as materialization rather than
    /// construction. Runtime-host has already accepted the typed construction
    /// command, applied protocol decorators, and selected the execution policy.
    pub(crate) async fn materialize_runtime_react_agent_from_context_snapshot_with_execution_policy(
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
