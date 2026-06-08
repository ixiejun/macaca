//! Event payloads emitted by the Plan scheduling loop (Observer pattern).
//!
//! `PlanEvent` variants cross the async channel to Plan Agent consumers.
//! The loop never calls LLM directly — it only signals work that needs attention.

use super::config::TaskSummary;

/// Events emitted by the Plan loop for the Plan Agent to act on.
#[derive(Debug, Clone)]
pub enum PlanEvent {
    /// A new goal is ready for LLM decomposition into tasks.
    GoalReady {
        goal_id: macaca_proto::TaskId,
        description: String,
        session_id: Option<String>,
    },
    /// A task needs Plan Agent review.
    ReviewNeeded {
        task_id: macaca_proto::TaskId,
        agent: String,
        title: String,
        summary: String,
        criteria: Vec<String>,
        session_id: Option<String>,
    },
    /// All tasks are done — decide whether to generate new work.
    AllTasksDone { completed: usize, failed: usize },
    /// An anomaly was detected (failed tasks, timeouts, etc.).
    AnomalyDetected { message: String },
    /// All tasks for a specific goal are done — request quality evaluation.
    EvaluateGoalCompletion {
        goal_id: macaca_proto::TaskId,
        goal_description: String,
        completed_count: usize,
        failed_count: usize,
        task_summaries: Vec<TaskSummary>,
        session_id: Option<String>,
    },
    /// Goal has been fully verified and marked complete.
    GoalCompleted {
        goal_id: macaca_proto::TaskId,
        description: String,
    },
}
