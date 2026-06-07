//! Public traced agent builders for executor-backed planner/worker execution.

use std::sync::Arc;
use macaca_framework::agent::HookedAgent;
use macaca_framework::construction::{AgentBuildIntent, AgentToolConfig, TracedAgentFactory};
use macaca_framework::react_agent::ReActAgent;
use macaca_proto::ApplicationId;
use crate::state::AppState;
use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;
use super::request_composition;
use super::FrameworkRunner;

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
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
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
        let request = request_composition::build_request(
            state, app_id, agent_name, session_id, task_id, goal_id, intent, tools,
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Executor { executor },
        };
        factory.build(request).await
    }
}
