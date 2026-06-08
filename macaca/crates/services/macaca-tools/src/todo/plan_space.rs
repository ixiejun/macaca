//! Plan-space tools: review, goal creation, reassignment, progress audit.
//!
//! **Observer pattern**: [`ReviewTodoTool`] and [`CreateGoalTool`] invoke optional callbacks
//! for trace/analytics without coupling to the web shell.
//! **Command pattern**: each tool maps one LLM-facing action to [`TaskSpace`] operations.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_task::TaskSpace;
use serde_json::{json, Value};

use crate::tool::Tool;

use super::callbacks::{OnGoalCreated, OnGoalRecorded, OnTodoReviewed};

/// Review a task submitted by an agent.
pub struct ReviewTodoTool {
    pub space: Arc<TaskSpace>,
    #[allow(clippy::type_complexity)]
    pub on_reviewed: Option<OnTodoReviewed>,
}

#[async_trait]
impl Tool for ReviewTodoTool {
    fn name(&self) -> &str {
        "review_todo"
    }
    fn description(&self) -> &str {
        "Review a submitted task. task_id must be a UUID from create_todo or list_agent_todos, not a title/slug."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "UUID string returned by create_todo, claim_task, or list_agent_todos — not a title or slug"
                },
                "agent": { "type": "string", "description": "Agent who owns the task" },
                "passed": { "type": "boolean", "description": "Whether the task passes review" },
                "feedback": { "type": "string", "description": "Review feedback or optimization suggestions" }
            },
            "required": ["task_id", "agent", "passed", "feedback"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let agent = input["agent"].as_str().unwrap_or_default();
        let passed = input["passed"].as_bool().unwrap_or(false);
        let feedback = input["feedback"].as_str().unwrap_or_default().to_string();

        let result = macaca_proto::TodoReviewResult {
            passed,
            feedback: feedback.clone(),
            verified_criteria: vec![],
        };
        let ok = self
            .space
            .apply_review_result(&task_id, agent, result)
            .await;
        if ok {
            tracing::info!(
                task_id = %task_id,
                agent = %agent,
                passed = passed,
                "Applied review_todo result"
            );
            if let Some(ref cb) = self.on_reviewed {
                cb(task_id, agent.to_string(), passed);
            }
        }
        Ok(json!({
            "success": ok,
            "task_id": task_id_str,
            "passed": passed,
            "new_status": if passed { "completed" } else { "needs_optimization" },
        }))
    }
}

/// Create a high-level goal for the Plan Agent to decompose into tasks.
pub struct CreateGoalTool {
    pub space: Arc<TaskSpace>,
    /// Optional callback to trigger PlanLoop startup after goal creation.
    pub on_created: Option<OnGoalCreated>,
    /// Optional hook after the goal row exists (tracing).
    pub on_goal_recorded: Option<OnGoalRecorded>,
}

#[async_trait]
impl Tool for CreateGoalTool {
    fn name(&self) -> &str {
        "create_goal"
    }
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
        let description = input["description"].as_str().ok_or_else(|| {
            macaca_proto::MacacaError::Task("Missing 'description' parameter".into())
        })?;

        let goal = self.space.push_goal(description).await;
        tracing::info!(goal_id = %goal.id, "Created project goal via create_goal tool");

        if let Some(ref cb) = self.on_goal_recorded {
            cb(goal.clone());
        }

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
    fn name(&self) -> &str {
        "reassign_task"
    }
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
        let task_id_str = input["task_id"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'task_id'".into()))?;
        let uuid = uuid::Uuid::parse_str(task_id_str).map_err(|_| {
            macaca_proto::MacacaError::Task(format!("Invalid task_id: {}", task_id_str))
        })?;
        let task_id = macaca_proto::TaskId(uuid);
        let current_agent = input["current_agent"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'current_agent'".into()))?;
        let new_agent = input["new_agent"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'new_agent'".into()))?;

        let success = self
            .space
            .reassign_task(&task_id, current_agent, new_agent)
            .await;

        if success {
            tracing::info!(
                task_id = %task_id_str,
                from = %current_agent,
                to = %new_agent,
                "Reassigned task between agents"
            );
            Ok(json!({
                "task_id": task_id_str,
                "reassigned_from": current_agent,
                "reassigned_to": new_agent,
                "new_status": "pending"
            }))
        } else {
            Err(macaca_proto::MacacaError::NotFound(format!(
                "Task {} not found on agent {}'s board",
                task_id_str, current_agent
            )))
        }
    }
}

/// Check overall progress of all tasks in the application.
pub struct CheckTodoProgressTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for CheckTodoProgressTool {
    fn name(&self) -> &str {
        "check_todo_progress"
    }
    fn description(&self) -> &str {
        "Check the overall progress of all tasks across all agents. When pending_review > 0, the response includes `pending_review_tasks` with `task_id` (UUID) for each task — use these with `review_todo`."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let p = self.space.overall_progress().await;
        let reviews = self.space.pending_reviews().await;
        let pending_review_tasks: Vec<Value> = reviews
            .into_iter()
            .take(50)
            .map(|t| {
                json!({
                    "task_id": t.id.to_string(),
                    "title": t.title,
                    "assigned_agent": t.assigned_agent,
                    "session_id": t.session_id,
                })
            })
            .collect();
        Ok(json!({
            "total": p.total,
            "pending": p.pending,
            "assigned": p.assigned,
            "in_progress": p.in_progress,
            "pending_review": p.pending_review,
            "pending_review_tasks": pending_review_tasks,
            "needs_optimization": p.needs_optimization,
            "completed": p.completed,
            "blocked": p.blocked,
            "failed": p.failed,
            "cancelled": p.cancelled,
            "all_done": p.completed + p.cancelled + p.failed == p.total && p.total > 0,
        }))
    }
}
