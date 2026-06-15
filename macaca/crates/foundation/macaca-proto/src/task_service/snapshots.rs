//! Task Service snapshot DTOs.

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, TaskGraphOwner, TaskId, TodoGoalStatus, TodoStatus, TraceContext};

/// Command to inspect the deterministic task service snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceSnapshotCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub trace: Option<TraceContext>,
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
    /// Service-owned graph classification used by projections and audit tools.
    pub graph_owner: TaskGraphOwner,
    /// Opaque service-owned graph identity shared by tasks in the same graph.
    pub graph_id: Option<String>,
}

/// Deterministic Task Service snapshot ordered by stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskServiceSnapshot {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub goals: Vec<TaskServiceGoalSnapshot>,
    pub tasks: Vec<TaskServiceTaskSnapshot>,
}
