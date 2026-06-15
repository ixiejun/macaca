//! Task Service event DTOs.

use serde::{Deserialize, Serialize};

use crate::TaskId;

/// Summary of a completed task used by goal-evaluation consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub title: String,
    pub agent: String,
    pub status: String,
    pub completion_summary: Option<String>,
}

/// Events emitted by a task-planning loop for downstream adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanEvent {
    /// A new goal is ready for decomposition into executable tasks.
    GoalReady {
        goal_id: TaskId,
        description: String,
        session_id: Option<String>,
    },
    /// A task needs coordinator review.
    ReviewNeeded {
        task_id: TaskId,
        agent: String,
        title: String,
        summary: String,
        criteria: Vec<String>,
        session_id: Option<String>,
    },
    /// All known tasks have reached terminal states.
    AllTasksDone { completed: usize, failed: usize },
    /// A task-service anomaly needs operator-visible reporting.
    AnomalyDetected { message: String },
    /// A goal's tasks are done and the coordinator should evaluate quality.
    EvaluateGoalCompletion {
        goal_id: TaskId,
        goal_description: String,
        completed_count: usize,
        failed_count: usize,
        task_summaries: Vec<TaskSummary>,
        session_id: Option<String>,
    },
    /// A goal has been verified and marked complete.
    GoalCompleted {
        goal_id: TaskId,
        description: String,
    },
}

/// Events emitted by a worker loop for agent-execution adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerEvent {
    /// A task was claimed and needs execution by the assigned agent.
    TaskClaimed {
        task_id: TaskId,
        title: String,
        description: String,
        acceptance_criteria: Vec<String>,
        context: Option<String>,
        optimization_suggestions: Option<String>,
        attempt: u32,
        session_id: Option<String>,
    },
    /// A task needs optimization/retry work from the assigned agent.
    RetryTask {
        task_id: TaskId,
        title: String,
        description: String,
        optimization_suggestions: String,
        attempt: u32,
        session_id: Option<String>,
    },
    /// No work is currently available.
    Idle,
}
