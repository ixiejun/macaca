//! Agent execution service adapter for PlanLoop / WorkerLoop delegated runs.
//!
//! All LLM/agent construction flows through `service.agent_execution` via
//! `AgentExecutionCommand`, preserving a single auditable execution path.

use std::sync::Arc;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationId, KernelServiceId, ServiceBusSource, TaskId, TraceContext,
    AGENT_EXECUTION_SERVICE_ID,
};

use super::planner_helpers::{
    executor_task_completed, executor_task_failed, executor_task_started,
    update_agent_activity_by_name,
};
use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlannerFrameworkCallKind {
    DecomposeGoal,
    Review,
    FollowUp,
    GoalEvaluation,
}

impl PlannerFrameworkCallKind {
    fn timeout(self) -> std::time::Duration {
        match self {
            Self::DecomposeGoal => std::time::Duration::from_secs(90),
            Self::Review | Self::FollowUp => std::time::Duration::from_secs(5 * 60),
            Self::GoalEvaluation => std::time::Duration::from_secs(10 * 60),
        }
    }

    pub(crate) fn goal_context(
        self,
        task_id: macaca_proto::TaskId,
    ) -> Option<macaca_proto::TaskId> {
        match self {
            Self::DecomposeGoal | Self::FollowUp | Self::GoalEvaluation => Some(task_id),
            Self::Review => None,
        }
    }

    pub(crate) fn success_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Planner decomposition completed",
            Self::Review => "Review completed",
            Self::FollowUp => "Follow-up tasks created",
            Self::GoalEvaluation => "Goal evaluation completed",
        }
    }

    pub(crate) fn reply_error_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Planner decomposition failed",
            Self::Review => "Review failed",
            Self::FollowUp => "Follow-up task creation failed",
            Self::GoalEvaluation => "Goal evaluation failed",
        }
    }

    pub(crate) fn build_error_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Failed to build planner agent",
            Self::Review => "Failed to build planner agent for review",
            Self::FollowUp => "Failed to build planner agent for follow-up",
            Self::GoalEvaluation => "Failed to build planner agent for goal evaluation",
        }
    }

    pub(crate) fn missing_executor_log(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "No executor found for planner decomposition",
            Self::Review => "No executor found for planner review",
            Self::FollowUp => "No executor found for planner follow-up",
            Self::GoalEvaluation => "No executor found for planner goal evaluation",
        }
    }
}

pub(crate) async fn run_loop_agent_execution_service_call(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
    session_id: Option<String>,
    task_id: TaskId,
    prompt: String,
    intent: AgentExecutionIntent,
    source: &'static str,
    timeout: std::time::Duration,
    delegated_context: serde_json::Value,
) -> Result<String, String> {
    let resolved_session_id = session_id.unwrap_or_else(|| format!("loop-task:{}", task_id.0));
    let mut trace = TraceContext::new(format!(
        "{source}:{}:{}:{}",
        app_id.0, agent_name, task_id.0
    ));
    trace.session_id = Some(resolved_session_id.clone());
    trace.task_id = Some(task_id.0.to_string());
    trace.agent = Some(agent_name.to_string());

    let mut command = AgentExecutionCommand::new(
        *app_id,
        resolved_session_id,
        agent_name,
        intent,
        prompt,
        trace,
    )
    .map_err(|error| error.to_string())?
    .with_delegated_context(delegated_context);
    command.task_id = Some(task_id);
    command
        .metadata
        .insert("entrypoint".into(), source.to_string());

    let service_command = command
        .into_service_command()
        .map_err(|error| error.to_string())?;
    let reply = tokio::time::timeout(
        timeout,
        state.service_runtime.call(
            &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
            ServiceBusSource::new(source),
            service_command,
        ),
    )
    .await
    .map_err(|_| {
        format!(
            "agent execution service timed out after {}s",
            timeout.as_secs()
        )
    })?
    .map_err(|error| error.to_string())?;

    let output = reply
        .output
        .ok_or_else(|| "agent execution service returned no output".to_string())?;
    let result: AgentExecutionResult =
        serde_json::from_value(output).map_err(|error| error.to_string())?;

    match result.status {
        AgentExecutionStatus::Completed => Ok(result
            .output
            .get("output")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| result.output.to_string())),
        status => Err(result
            .output
            .get("error")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("agent execution service returned {}", status.as_str()))),
    }
}

pub(crate) async fn run_planner_framework_call(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    plan_agent_name: &str,
    session_id: Option<String>,
    task_id: macaca_proto::TaskId,
    prompt: String,
    activity_context: String,
    kind: PlannerFrameworkCallKind,
) -> Result<String, String> {
    let Some(executor) = state.executor_registry.get(app_id).await else {
        let error = kind.missing_executor_log().to_string();
        tracing::error!("{}", error);
        return Err(error);
    };

    update_agent_activity_by_name(
        state,
        plan_agent_name,
        macaca_proto::AgentActivity::Working {
            context: activity_context,
        },
    )
    .await;
    executor.broadcast_event(executor_task_started(task_id, plan_agent_name));

    let intent = match kind {
        PlannerFrameworkCallKind::Review => AgentExecutionIntent::Reviewer,
        PlannerFrameworkCallKind::DecomposeGoal => AgentExecutionIntent::Planner,
        PlannerFrameworkCallKind::FollowUp | PlannerFrameworkCallKind::GoalEvaluation => {
            AgentExecutionIntent::GoalWorker
        }
    };

    let result = run_loop_agent_execution_service_call(
        state,
        app_id,
        plan_agent_name,
        session_id,
        task_id,
        prompt,
        intent,
        "macaca.web.plan_loop",
        kind.timeout(),
        serde_json::json!({
            "planner_call_kind": format!("{kind:?}"),
            "goal_context": kind.goal_context(task_id).map(|id| id.0.to_string()),
        }),
    )
    .await;
    match &result {
        Ok(output) => {
            executor.broadcast_event(executor_task_completed(
                task_id,
                plan_agent_name,
                output.clone(),
            ));
            tracing::info!(
                "{}: {}",
                kind.success_log_prefix(),
                output.chars().take(100).collect::<String>()
            );
        }
        Err(error) => {
            executor.broadcast_event(executor_task_failed(
                task_id,
                plan_agent_name,
                error.clone(),
            ));
            tracing::error!("{}: {}", kind.reply_error_log_prefix(), error);
        }
    }

    update_agent_activity_by_name(state, plan_agent_name, macaca_proto::AgentActivity::Idle).await;
    result
}

pub(crate) async fn list_goal_todos_for_scope(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
) -> Vec<macaca_proto::TodoItem> {
    let todos = match session_id {
        Some(sid) => {
            state
                .persist
                .todo_store
                .list_all_todos_for_session(app_id, sid)
                .await
        }
        None => state.persist.todo_store.list_all_todos(app_id).await,
    };
    todos
        .into_iter()
        .filter(|task| task.parent_task == Some(goal_id))
        .collect()
}
