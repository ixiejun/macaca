//! Goal and board query command handlers.

use std::sync::Arc;

use tracing::{info, warn};

use macaca_proto::{
    diagnose_session_claims, AgentTaskBoardResult, SessionClaimDiagnostics, TaskBoardQueryResult,
    TaskGoalsResult, TaskProgressSummary, TodoGoal, TodoGoalStatus,
};

use crate::commands::{
    CompleteGoalCommand, CreateGoalCommand, QueryAgentTodosCommand, QueryTaskBoardCommand,
    QueryTaskClaimDiagnosticsCommand, QueryTaskGoalsCommand, QueryTaskProgressCommand,
};
use crate::events::{TaskServiceEvent, TaskServiceEventType};
use crate::todo_board::TaskSpace;

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    pub async fn create_goal(&self, command: CreateGoalCommand) -> Result<TodoGoal, String> {
        let description = command.description.trim().to_string();
        if description.is_empty() {
            return Err("task service goal description cannot be blank".into());
        }

        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let goal = space.push_goal(description.clone()).await;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            None,
            Some(goal.id),
            TaskServiceEventType::GoalReady,
            command.trace.clone(),
            serde_json::json!({
                "goal_id": goal.id.to_string(),
                "description": description,
                "status": format!("{:?}", goal.status),
            }),
        ))
        .await;

        if let Err(error) = self
            .execution
            .decompose_goal(&goal, &space, command.trace.clone())
            .await
        {
            warn!(goal_id = %goal.id, error = %error, "task service goal decomposition hook failed");
        }

        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(goal)
    }

    /// Mark a high-level goal as completed through the Task Service boundary.
    ///
    /// This owns the durable state transition that shell event consumers used
    /// to write through `TaskSpace`. The command preserves the goal's stored
    /// session scope while letting callers pass an optional trace scope for
    /// audit correlation.
    pub async fn complete_goal(&self, command: CompleteGoalCommand) -> Result<bool, String> {
        let goals = self.store.list_goals(&command.app_id).await;
        let Some(mut goal) = goals.into_iter().find(|goal| goal.id == command.goal_id) else {
            warn!(
                app_id = %command.app_id.0,
                session_id = ?command.session_id,
                goal_id = %command.goal_id,
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service goal completion skipped because goal was not found"
            );
            return Ok(false);
        };

        goal.status = TodoGoalStatus::Completed;
        self.store.save_goal(&goal).await;
        info!(
            app_id = %command.app_id.0,
            session_id = ?goal.session_id,
            goal_id = %goal.id,
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service goal completed"
        );
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            goal.session_id.clone(),
            None,
            Some(goal.id),
            TaskServiceEventType::GoalCompleted,
            command.trace.clone(),
            serde_json::json!({
                "goal_id": goal.id.to_string(),
                "status": format!("{:?}", goal.status),
            }),
        ))
        .await;
        self.refresh_snapshot(&command.app_id, goal.session_id.as_deref())
            .await;
        Ok(true)
    }

    pub async fn query_task_board(
        &self,
        command: QueryTaskBoardCommand,
    ) -> Result<TaskBoardQueryResult, String> {
        let mut todos = self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await;
        todos.sort_by_key(|todo| todo.sequence_number);
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            Some(command.session_id.clone()),
            None,
            None,
            TaskServiceEventType::TaskSnapshotRequested,
            command.trace.clone(),
            serde_json::json!({
                "count": todos.len(),
                "session_id": command.session_id,
            }),
        ))
        .await;
        let count = todos.len();
        Ok(TaskBoardQueryResult { todos, count })
    }

    pub async fn query_progress(
        &self,
        command: QueryTaskProgressCommand,
    ) -> Result<TaskProgressSummary, String> {
        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let progress = space.overall_progress().await;
        let result = TaskProgressSummary {
            total: progress.total,
            pending: progress.pending,
            assigned: progress.assigned,
            in_progress: progress.in_progress,
            pending_review: progress.pending_review,
            needs_optimization: progress.needs_optimization,
            completed: progress.completed,
            blocked: progress.blocked,
            failed: progress.failed,
            cancelled: progress.cancelled,
            all_done: space.all_tasks_done().await,
        };
        info!(
            app_id = %command.app_id.0,
            session_id = ?command.session_id,
            total = result.total,
            all_done = result.all_done,
            "task service progress queried"
        );
        Ok(result)
    }

    pub async fn query_agent_todos(
        &self,
        command: QueryAgentTodosCommand,
    ) -> Result<AgentTaskBoardResult, String> {
        if command.agent_name.trim().is_empty() {
            return Err("task service agent todo query requires non-empty agent_name".into());
        }
        let mut todos = self
            .store
            .list_agent_todos(&command.app_id, &command.session_id, &command.agent_name)
            .await;
        todos.sort_by_key(|todo| todo.sequence_number);
        let count = todos.len();
        info!(
            app_id = %command.app_id.0,
            session_id = ?command.session_id,
            agent = %command.agent_name,
            count,
            "task service agent board queried"
        );
        Ok(AgentTaskBoardResult {
            agent: command.agent_name,
            todos,
            count,
        })
    }

    pub async fn query_goals(
        &self,
        command: QueryTaskGoalsCommand,
    ) -> Result<TaskGoalsResult, String> {
        let mut goals = match command.session_id.as_deref() {
            Some(session_id) => {
                self.store
                    .list_goals_for_session(&command.app_id, session_id)
                    .await
            }
            None => self.store.list_goals(&command.app_id).await,
        };
        goals.sort_by_key(|goal| goal.created_at);
        let count = goals.len();
        info!(
            app_id = %command.app_id.0,
            session_id = ?command.session_id,
            count,
            "task service goals queried"
        );
        Ok(TaskGoalsResult { goals, count })
    }

    pub async fn query_claim_diagnostics(
        &self,
        command: QueryTaskClaimDiagnosticsCommand,
    ) -> Result<SessionClaimDiagnostics, String> {
        if command.session_id.trim().is_empty() {
            return Err("task service claim diagnostics requires non-empty session_id".into());
        }
        let todos = self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await;
        let diagnostics = diagnose_session_claims(&todos);
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            agents = diagnostics.agents.len(),
            "task service claim diagnostics queried"
        );
        Ok(diagnostics)
    }
}
