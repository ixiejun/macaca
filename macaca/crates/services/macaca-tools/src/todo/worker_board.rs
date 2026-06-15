//! Worker-agent TaskBoard tools (pull-based task lifecycle).
//!
//! Worker tools: `claim_task`, `start_task`, `update_task_progress`,
//! `submit_task_for_review`, `list_my_tasks`.
//!
//! **Command pattern**: each struct implements [`Tool`] and delegates to [`TaskBoard`].

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_task::TaskBoard;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolCommand};

/// Claim the highest-priority pending task from the agent's board.
pub struct ClaimTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ClaimTaskTool {
    fn name(&self) -> &str {
        "claim_task"
    }
    fn description(&self) -> &str {
        "Claim the highest-priority pending task from your task board. Returns the task details or null if no tasks available."
    }
    fn tool_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn invoke(&self, _command: ToolCommand) -> MacacaResult<Value> {
        match self.board.claim_next_task().await {
            Some(task) => Ok(json!({
                "task_id": task.id.to_string(),
                "title": task.title,
                "description": task.description,
                "acceptance_criteria": task.acceptance_criteria,
                "priority": task.priority,
                "context": task.context,
                "optimization_suggestions": task.optimization_suggestions,
                "attempt": task.attempt_count,
            })),
            None => {
                Ok(json!({ "status": "no_tasks", "message": "No pending tasks on your board" }))
            }
        }
    }
}

/// Mark a claimed task as in-progress.
pub struct StartTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for StartTaskTool {
    fn name(&self) -> &str {
        "start_task"
    }
    fn description(&self) -> &str {
        "Mark a claimed task as in-progress. Call this after claim_task before starting work."
    }
    fn tool_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "task_id": { "type": "string", "description": "Task ID to start" } },
            "required": ["task_id"]
        })
    }
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let ok = self.board.mark_task_in_progress(&task_id).await;
        Ok(json!({ "success": ok, "task_id": task_id_str }))
    }
}

/// Update progress on the current in-progress task.
pub struct UpdateTaskProgressTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for UpdateTaskProgressTool {
    fn name(&self) -> &str {
        "update_task_progress"
    }
    fn description(&self) -> &str {
        "Update progress on the current in-progress task."
    }
    fn tool_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID" },
                "message": { "type": "string", "description": "Progress update message" }
            },
            "required": ["task_id", "message"]
        })
    }
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let message = input["message"].as_str().unwrap_or_default().to_string();
        let ok = self.board.update_progress(&task_id, message).await;
        Ok(json!({ "success": ok }))
    }
}

/// Submit a completed task for Plan Agent review.
pub struct SubmitTaskForReviewTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for SubmitTaskForReviewTool {
    fn name(&self) -> &str {
        "submit_task_for_review"
    }
    fn description(&self) -> &str {
        "Submit a completed task for review by the Plan Agent. Include a summary of what was done."
    }
    fn tool_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to submit" },
                "summary": { "type": "string", "description": "Summary of completed work" }
            },
            "required": ["task_id", "summary"]
        })
    }
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let summary = input["summary"].as_str().unwrap_or_default().to_string();
        let ok = self.board.submit_task_for_review(&task_id, summary).await;
        Ok(json!({ "success": ok, "status": if ok { "pending_review" } else { "error" } }))
    }
}

/// List all tasks on the agent's board.
pub struct ListMyTasksTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ListMyTasksTool {
    fn name(&self) -> &str {
        "list_my_tasks"
    }
    fn description(&self) -> &str {
        "List all tasks on your task board with their statuses."
    }
    fn tool_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn invoke(&self, _command: ToolCommand) -> MacacaResult<Value> {
        let tasks = self.board.list_all().await;
        let items: Vec<Value> = tasks
            .iter()
            .map(|t| {
                json!({
                    "task_id": t.id.to_string(),
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "attempt_count": t.attempt_count,
                    "optimization_suggestions": t.optimization_suggestions,
                })
            })
            .collect();
        Ok(json!({ "agent": self.board.agent_name(), "tasks": items, "count": items.len() }))
    }
}
