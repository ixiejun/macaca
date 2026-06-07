//! Typed command contracts for the Task Service boundary.
//!
//! This module defines the command surface used by Web, CLI, and SDK callers
//! when they need task planning, review, claim, resume, or snapshot behavior.
//! The commands are provider-neutral and carry trace/session/task scope so the
//! task service can remain auditable and replaceable.

use serde::{Deserialize, Serialize};

use macaca_proto::{ApplicationId, TaskGraphOwner, TaskId, TodoReviewResult, TraceContext};

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

/// Command to query task board state in a session-scoped way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTaskBoardCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub trace: Option<TraceContext>,
}

impl QueryTaskBoardCommand {
    /// Create a session-scoped task board query.
    pub fn new(
        app_id: ApplicationId,
        session_id: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.into().trim().to_string(),
            trace,
        }
    }
}

/// Command to create one explicit task assignment.
///
/// This command is used when an upstream orchestrator already selected the
/// provider-neutral agent handoff sequence and needs the Task Service to own the
/// durable board record.  It deliberately carries generic task fields only; the
/// service does not inspect workflow names, application names, or business
/// payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTaskAssignmentCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
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
    /// Service-owned graph classification for this task assignment.
    ///
    /// The default is `ApplicationExecution` because this command is the
    /// explicit service boundary used by application execution adapters such as
    /// WASM `agent.delegate`.  Legacy task-board writers that call
    /// `TaskSpace` directly keep the `TaskServiceNative` default on `TodoItem`
    /// unless they deliberately choose another owner.
    #[serde(default = "default_assignment_graph_owner")]
    pub graph_owner: TaskGraphOwner,
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

/// Command to inspect the task service snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceSnapshotCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub trace: Option<TraceContext>,
}
