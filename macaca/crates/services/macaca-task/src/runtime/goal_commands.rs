//! Goal and board query command handlers.

use std::sync::Arc;

use tracing::warn;

use macaca_proto::{TodoGoal, TodoItem};

use crate::commands::{CreateGoalCommand, QueryTaskBoardCommand};
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
    pub async fn query_task_board(
        &self,
        command: QueryTaskBoardCommand,
    ) -> Result<Vec<TodoItem>, String> {
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
        Ok(todos)
    }
}
