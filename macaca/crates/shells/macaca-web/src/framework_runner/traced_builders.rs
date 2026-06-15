//! Public traced agent builders for executor-backed planner/worker execution.

use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;
use super::request_composition;
use super::FrameworkRunner;
use crate::state::AppState;
use macaca_host_composition::framework::agent::HookedAgent;
use macaca_host_composition::framework::construction::{
    AgentBuildIntent, AgentToolConfig, TracedAgentFactory,
};
use macaca_host_composition::framework::react_agent::ReActAgent;
use macaca_proto::ApplicationId;
use std::sync::Arc;

impl FrameworkRunner {
    /// Build a traced `ReActAgent` without goal context.
    pub async fn build_traced_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
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
        executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
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
    /// consumers so they do not need to know shell-local builder naming.
    pub async fn build_for_intent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
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
