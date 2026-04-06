//! Todo task board tools for Worker Agents and Plan Agents.
//!
//! Worker tools: claim_task, start_task, update_task_progress, submit_task_for_review, list_my_tasks
//! Plan tools:   create_todo, review_todo, check_todo_progress

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_task::{TaskBoard, TaskSpace};
use serde_json::{json, Value};

use crate::tool::Tool;

// ─────────────────────────────────────────────────────────────────────────────
// Worker Agent Tools
// ─────────────────────────────────────────────────────────────────────────────

/// Claim the highest-priority pending task from the agent's board.
pub struct ClaimTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ClaimTaskTool {
    fn name(&self) -> &str { "claim_task" }
    fn description(&self) -> &str {
        "Claim the highest-priority pending task from your task board. Returns the task details or null if no tasks available."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        match self.board.claim_next().await {
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
            None => Ok(json!({ "status": "no_tasks", "message": "No pending tasks on your board" })),
        }
    }
}

/// Mark a claimed task as in-progress.
pub struct StartTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for StartTaskTool {
    fn name(&self) -> &str { "start_task" }
    fn description(&self) -> &str {
        "Mark a claimed task as in-progress. Call this after claim_task before starting work."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "task_id": { "type": "string", "description": "Task ID to start" } },
            "required": ["task_id"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(uuid::Uuid::parse_str(task_id_str).map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?);
        let ok = self.board.start_task(&task_id).await;
        Ok(json!({ "success": ok, "task_id": task_id_str }))
    }
}

/// Update progress on the current in-progress task.
pub struct UpdateTaskProgressTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for UpdateTaskProgressTool {
    fn name(&self) -> &str { "update_task_progress" }
    fn description(&self) -> &str {
        "Update progress on the current in-progress task."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID" },
                "message": { "type": "string", "description": "Progress update message" }
            },
            "required": ["task_id", "message"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(uuid::Uuid::parse_str(task_id_str).map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?);
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
    fn name(&self) -> &str { "submit_task_for_review" }
    fn description(&self) -> &str {
        "Submit a completed task for review by the Plan Agent. Include a summary of what was done."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to submit" },
                "summary": { "type": "string", "description": "Summary of completed work" }
            },
            "required": ["task_id", "summary"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(uuid::Uuid::parse_str(task_id_str).map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?);
        let summary = input["summary"].as_str().unwrap_or_default().to_string();
        let ok = self.board.submit_for_review(&task_id, summary).await;
        Ok(json!({ "success": ok, "status": if ok { "pending_review" } else { "error" } }))
    }
}

/// List all tasks on the agent's board.
pub struct ListMyTasksTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ListMyTasksTool {
    fn name(&self) -> &str { "list_my_tasks" }
    fn description(&self) -> &str {
        "List all tasks on your task board with their statuses."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let tasks = self.board.list_all().await;
        let items: Vec<Value> = tasks.iter().map(|t| json!({
            "task_id": t.id.to_string(),
            "title": t.title,
            "status": t.status,
            "priority": t.priority,
            "attempt_count": t.attempt_count,
            "optimization_suggestions": t.optimization_suggestions,
        })).collect();
        Ok(json!({ "agent": self.board.agent_name(), "tasks": items, "count": items.len() }))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plan Agent Tools
// ─────────────────────────────────────────────────────────────────────────────

/// Create a new task and assign it to an agent's board.
pub struct CreateTodoTool {
    pub space: Arc<TaskSpace>,
    pub coordinator_name: String,
}

#[async_trait]
impl Tool for CreateTodoTool {
    fn name(&self) -> &str { "create_todo" }
    fn description(&self) -> &str {
        "Create a new task and assign it to a specific agent's task board."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agent": { "type": "string", "description": "Target agent name (e.g. backend, frontend)" },
                "title": { "type": "string", "description": "Short task title" },
                "description": { "type": "string", "description": "Detailed task description" },
                "acceptance_criteria": {
                    "type": "array", "items": { "type": "string" },
                    "description": "List of criteria that must be met for the task to pass review"
                },
                "priority": { "type": "integer", "description": "Priority 0-10, higher = more urgent", "default": 5 },
                "depends_on": {
                    "type": "array", "items": { "type": "string" },
                    "description": "Task IDs that must complete before this task can start"
                }
            },
            "required": ["agent", "title", "description"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let agent = input["agent"].as_str().unwrap_or("backend");
        let title = input["title"].as_str().unwrap_or_default();
        let description = input["description"].as_str().unwrap_or_default();
        let priority = input["priority"].as_u64().unwrap_or(5) as u8;
        let criteria: Vec<String> = input["acceptance_criteria"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let depends_on: Vec<macaca_proto::TaskId> = input["depends_on"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| {
                let s = v.as_str()?;
                uuid::Uuid::parse_str(s).ok().map(macaca_proto::TaskId)
            }).collect())
            .unwrap_or_default();

        let item = self.space.create_and_assign(
            agent, &self.coordinator_name, title, description,
            criteria, priority, depends_on, None,
        ).await;

        Ok(json!({
            "task_id": item.id.to_string(),
            "agent": agent,
            "status": item.status,
            "priority": priority,
        }))
    }
}

/// Review a task submitted by an agent.
pub struct ReviewTodoTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for ReviewTodoTool {
    fn name(&self) -> &str { "review_todo" }
    fn description(&self) -> &str {
        "Review a task that an agent submitted for review. Pass or fail with feedback."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string" },
                "agent": { "type": "string", "description": "Agent who owns the task" },
                "passed": { "type": "boolean", "description": "Whether the task passes review" },
                "feedback": { "type": "string", "description": "Review feedback or optimization suggestions" }
            },
            "required": ["task_id", "agent", "passed", "feedback"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(uuid::Uuid::parse_str(task_id_str).map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?);
        let agent = input["agent"].as_str().unwrap_or_default();
        let passed = input["passed"].as_bool().unwrap_or(false);
        let feedback = input["feedback"].as_str().unwrap_or_default().to_string();

        let result = macaca_proto::TodoReviewResult {
            passed,
            feedback: feedback.clone(),
            verified_criteria: vec![],
        };
        let ok = self.space.review_task(&task_id, agent, result).await;
        Ok(json!({
            "success": ok,
            "task_id": task_id_str,
            "passed": passed,
            "new_status": if passed { "completed" } else { "needs_optimization" },
        }))
    }
}

/// Callback invoked after a goal is created, allowing the web layer to
/// lazily start the PlanLoop without introducing a circular dependency.
pub type OnGoalCreated = Arc<dyn Fn() + Send + Sync>;

/// Create a high-level goal for the Plan Agent to decompose into tasks.
pub struct CreateGoalTool {
    pub space: Arc<TaskSpace>,
    /// Optional callback to trigger PlanLoop startup after goal creation.
    pub on_created: Option<OnGoalCreated>,
}

#[async_trait]
impl Tool for CreateGoalTool {
    fn name(&self) -> &str { "create_goal" }
    fn description(&self) -> &str {
        "Create a high-level project goal. The Plan Agent will automatically decompose it into concrete tasks and assign them to appropriate agents. Use this for complex multi-step work."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "The goal description"
                }
            },
            "required": ["description"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let description = input["description"].as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'description' parameter".into()))?;

        let goal = self.space.push_goal(description).await;

        // Trigger PlanLoop startup if callback is set
        if let Some(ref cb) = self.on_created {
            cb();
        }

        Ok(json!({
            "goal_id": goal.id.to_string(),
            "status": "pending",
            "message": "Goal created. The Plan Agent will decompose it into tasks."
        }))
    }
}

/// Reassign a task from one agent to another (Plan Agent only).
pub struct ReassignTaskTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for ReassignTaskTool {
    fn name(&self) -> &str { "reassign_task" }
    fn description(&self) -> &str {
        "Reassign a task from one agent to another. The task status is reset to Pending so the new agent can claim it. Use when an agent cannot complete a task or the task was misrouted."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The task ID to reassign" },
                "current_agent": { "type": "string", "description": "The agent currently assigned to the task" },
                "new_agent": { "type": "string", "description": "The agent to reassign the task to" }
            },
            "required": ["task_id", "current_agent", "new_agent"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'task_id'".into()))?;
        let uuid = uuid::Uuid::parse_str(task_id_str)
            .map_err(|_| macaca_proto::MacacaError::Task(format!("Invalid task_id: {}", task_id_str)))?;
        let task_id = macaca_proto::TaskId(uuid);
        let current_agent = input["current_agent"].as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'current_agent'".into()))?;
        let new_agent = input["new_agent"].as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'new_agent'".into()))?;

        let success = self.space.reassign_task(&task_id, current_agent, new_agent).await;

        if success {
            Ok(json!({
                "task_id": task_id_str,
                "reassigned_from": current_agent,
                "reassigned_to": new_agent,
                "new_status": "pending"
            }))
        } else {
            Err(macaca_proto::MacacaError::NotFound(
                format!("Task {} not found on agent {}'s board", task_id_str, current_agent)
            ))
        }
    }
}

/// Check overall progress of all tasks in the application.
pub struct CheckTodoProgressTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for CheckTodoProgressTool {
    fn name(&self) -> &str { "check_todo_progress" }
    fn description(&self) -> &str {
        "Check the overall progress of all tasks across all agents in the application."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let p = self.space.overall_progress().await;
        Ok(json!({
            "total": p.total,
            "pending": p.pending,
            "assigned": p.assigned,
            "in_progress": p.in_progress,
            "pending_review": p.pending_review,
            "needs_optimization": p.needs_optimization,
            "completed": p.completed,
            "blocked": p.blocked,
            "failed": p.failed,
            "cancelled": p.cancelled,
            "all_done": p.completed + p.cancelled + p.failed == p.total && p.total > 0,
        }))
    }
}
