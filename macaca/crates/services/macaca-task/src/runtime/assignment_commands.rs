//! Explicit task assignment command handler.

use std::sync::Arc;

use tracing::info;

use macaca_proto::TodoItem;

use crate::commands::CreateTaskAssignmentCommand;
use crate::events::{TaskServiceEvent, TaskServiceEventType};
use crate::todo_board::TaskSpace;

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    pub async fn create_task_assignment(
        &self,
        command: CreateTaskAssignmentCommand,
    ) -> Result<TodoItem, String> {
        let agent_name = command.agent_name.trim().to_string();
        if agent_name.is_empty() {
            return Err("task service assignment agent cannot be blank".into());
        }
        let title = command.title.trim().to_string();
        if title.is_empty() {
            return Err("task service assignment title cannot be blank".into());
        }
        let graph_id = Self::normalize_assignment_graph_id(&command);
        self.admit_assignment_graph(&command, graph_id.as_deref())
            .await?;
        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let task = space
            .create_task_assignment_with_graph_scope(
                &agent_name,
                command.created_by.trim(),
                title,
                command.description,
                command.acceptance_criteria,
                command.priority,
                command.depends_on,
                command.parent_task,
                command.graph_owner,
                graph_id.clone(),
            )
            .await;
        info!(
            app_id = %command.app_id.0,
            session_id = ?command.session_id,
            agent = %agent_name,
            task_id = %task.id,
            graph_owner = %command.graph_owner.as_str(),
            graph_id = graph_id.as_deref().unwrap_or("none"),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service assignment admitted"
        );
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            Some(task.id),
            task.parent_task,
            TaskServiceEventType::TaskCreated,
            command.trace.clone(),
            serde_json::json!({
                "task_id": task.id.to_string(),
                "agent": task.assigned_agent,
                "status": format!("{:?}", task.status),
                "sequence_number": task.sequence_number,
                "graph_owner": task.graph_owner.as_str(),
                "graph_id": task.graph_id.as_deref(),
            }),
        ))
        .await;
        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(task)
    }
}
