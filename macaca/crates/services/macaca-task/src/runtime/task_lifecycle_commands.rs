//! Worker task lifecycle commands (claim, start, review).

use std::sync::Arc;

use macaca_proto::TodoItem;
use tracing::{info, warn};

use crate::commands::{
    ClaimTaskCommand, FailTaskCommand, ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand,
};
use crate::events::{TaskServiceEvent, TaskServiceEventType};
use crate::todo_board::{TaskBoard, TaskSpace};

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    pub async fn claim_task(&self, command: ClaimTaskCommand) -> Result<Option<TodoItem>, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let task = board.claim_task(&command.task_id).await;
        if let Some(task) = &task {
            info!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %task.id,
                graph_owner = %task.graph_owner.as_str(),
                graph_id = task.graph_id.as_deref().unwrap_or("none"),
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service task claimed"
            );
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(task.id),
                task.parent_task,
                TaskServiceEventType::TaskClaimed,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": task.id.to_string(),
                    "agent": task.assigned_agent,
                    "status": format!("{:?}", task.status),
                    "graph_owner": task.graph_owner.as_str(),
                    "graph_id": task.graph_id.as_deref(),
                }),
            ))
            .await;
        } else {
            warn!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                "task service could not claim requested task"
            );
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(task)
    }

    /// Mark a task as started.
    pub async fn start_task(&self, command: StartTaskCommand) -> Result<bool, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let started = board.mark_task_in_progress(&command.task_id).await;
        if started {
            info!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service task started"
            );
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(command.task_id),
                None,
                TaskServiceEventType::TaskStarted,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "agent": command.agent_name,
                }),
            ))
            .await;
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(started)
    }

    /// Submit a task for review.
    pub async fn submit_review(&self, command: SubmitReviewCommand) -> Result<bool, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let submitted = board
            .submit_task_for_review(&command.task_id, command.summary.clone())
            .await;
        if submitted {
            info!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                summary_len = command.summary.len(),
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service task submitted for review"
            );
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(command.task_id),
                None,
                TaskServiceEventType::ReviewNeeded,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "summary": command.summary,
                    "agent": command.agent_name,
                }),
            ))
            .await;
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(submitted)
    }

    /// Mark a task as failed through the Task Service lifecycle boundary.
    pub async fn fail_task(&self, command: FailTaskCommand) -> Result<bool, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let failed = board
            .fail_task(&command.task_id, command.error.clone())
            .await;
        if failed {
            warn!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                error_len = command.error.len(),
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service task failed"
            );
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(command.task_id),
                None,
                TaskServiceEventType::TaskFailed,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "agent": command.agent_name,
                    "error": command.error,
                }),
            ))
            .await;
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(failed)
    }

    /// Apply a review result to a task and emit the outcome event.
    pub async fn review_task(&self, command: ReviewTaskCommand) -> Result<bool, String> {
        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let reviewed = space
            .apply_review_result(
                &command.task_id,
                &command.agent_name,
                command.result.clone(),
            )
            .await;

        if reviewed {
            info!(
                app_id = %command.app_id.0,
                session_id = ?command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                passed = command.result.passed,
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task service task reviewed"
            );
            if !command.result.passed {
                warn!(
                    app_id = %command.app_id.0,
                    session_id = ?command.session_id,
                    agent = %command.agent_name,
                    task_id = %command.task_id,
                    trace_id = command
                        .trace
                        .as_ref()
                        .map(|trace| trace.trace_id.as_str())
                        .unwrap_or("none"),
                    "task service task review failed"
                );
            }
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                command.session_id.clone(),
                Some(command.task_id),
                None,
                TaskServiceEventType::ReviewCompleted,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "passed": command.result.passed,
                    "feedback": command.result.feedback,
                }),
            ))
            .await;
        }

        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(reviewed)
    }
}
