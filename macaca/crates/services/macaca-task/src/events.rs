//! Typed task lifecycle events for the Task Service boundary.
//!
//! These events are emitted by the service runtime and observed by Web, CLI,
//! SSE, EventLog, and future remote adapters. They are intentionally compact,
//! serializable, and trace-aware so they can serve as audit records and UI
//! refresh signals without embedding execution strategy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use macaca_proto::{ApplicationId, TaskId, TodoGoalStatus, TodoStatus, TraceContext};

/// Structured task service lifecycle event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskServiceEvent {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
    pub goal_id: Option<TaskId>,
    pub event_type: TaskServiceEventType,
    pub trace: Option<TraceContext>,
    pub emitted_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

impl TaskServiceEvent {
    /// Build a structured event with the current timestamp.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        task_id: Option<TaskId>,
        goal_id: Option<TaskId>,
        event_type: TaskServiceEventType,
        trace: Option<TraceContext>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            app_id,
            session_id,
            task_id,
            goal_id,
            event_type,
            trace,
            emitted_at: Utc::now(),
            payload,
        }
    }
}

/// Enumerates the task lifecycle milestones that the service may emit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskServiceEventType {
    GoalReady,
    GoalDecomposed,
    TaskCreated,
    TaskClaimed,
    TaskStarted,
    ReviewNeeded,
    ReviewCompleted,
    GoalEvaluating,
    GoalCompleted,
    CoordinatorResumeRequested,
    TaskSnapshotRequested,
    TaskSnapshotEmitted,
    TaskRejected,
    TaskFailed,
}

/// Deterministic snapshot of one task service goal state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceGoalSnapshot {
    pub goal_id: TaskId,
    pub description: String,
    pub status: TodoGoalStatus,
    pub session_id: Option<String>,
}

/// Deterministic snapshot of one task service task state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceTaskSnapshot {
    pub task_id: TaskId,
    pub title: String,
    pub agent_name: String,
    pub status: TodoStatus,
    pub session_id: Option<String>,
}

/// Deterministic Task Service snapshot ordered by stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceSnapshot {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub goals: Vec<TaskServiceGoalSnapshot>,
    pub tasks: Vec<TaskServiceTaskSnapshot>,
}
