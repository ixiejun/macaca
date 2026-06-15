//! Task Service lifecycle command DTOs.

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, TaskGraphOwner, TaskId, TodoReviewResult, TraceContext};

/// Command to submit a high-level goal into the task system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGoalCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub description: String,
    pub trace: Option<TraceContext>,
}

impl CreateGoalCommand {
    /// Create a goal command after trimming the description and scope fields.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        description: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.map(|value| value.trim().to_string()),
            description: description.into().trim().to_string(),
            trace,
        }
    }
}

/// Command to mark a high-level goal as completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteGoalCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub goal_id: TaskId,
    pub trace: Option<TraceContext>,
}

impl CompleteGoalCommand {
    /// Create a goal-completion command with optional session scope.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        goal_id: TaskId,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.map(|value| value.trim().to_string()),
            goal_id,
            trace,
        }
    }
}

/// Command to create one explicit task assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTaskAssignmentCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub created_by: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub priority: u8,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub parent_task: Option<TaskId>,
    #[serde(default = "default_assignment_graph_owner")]
    pub graph_owner: TaskGraphOwner,
    #[serde(default)]
    pub graph_id: Option<String>,
    pub trace: Option<TraceContext>,
}

fn default_assignment_graph_owner() -> TaskGraphOwner {
    TaskGraphOwner::ApplicationExecution
}

/// Command to request task claim orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimTaskCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
    pub task_id: TaskId,
    pub trace: Option<TraceContext>,
}

/// Command to mark a task as started by an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartTaskCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
    pub task_id: TaskId,
    pub trace: Option<TraceContext>,
}

/// Command to submit a task for review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitReviewCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
    pub task_id: TaskId,
    pub summary: String,
    pub trace: Option<TraceContext>,
}

/// Command to mark a task as failed by an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailTaskCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
    pub task_id: TaskId,
    pub error: String,
    pub trace: Option<TraceContext>,
}

/// Command to apply a review result to a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTaskCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub task_id: TaskId,
    pub result: TodoReviewResult,
    pub trace: Option<TraceContext>,
}

/// Command to request a coordinator resume after task completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCoordinatorCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub goal_id: Option<TaskId>,
    pub reason: String,
    pub trace: Option<TraceContext>,
}
