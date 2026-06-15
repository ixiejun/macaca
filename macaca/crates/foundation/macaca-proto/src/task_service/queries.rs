//! Task Service query command and result DTOs.

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, TodoGoal, TodoItem, TraceContext};

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

/// Command to query aggregate task progress for an app or one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTaskProgressCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub trace: Option<TraceContext>,
}

impl QueryTaskProgressCommand {
    /// Create an app or session scoped task progress query.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.map(|value| value.trim().to_string()),
            trace,
        }
    }
}

/// Command to query one agent's task board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryAgentTodosCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub agent_name: String,
    pub trace: Option<TraceContext>,
}

impl QueryAgentTodosCommand {
    /// Create an agent-board query without embedding application-specific semantics.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.map(|value| value.trim().to_string()),
            agent_name: agent_name.into().trim().to_string(),
            trace,
        }
    }
}

/// Command to query task goals for an app or one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTaskGoalsCommand {
    pub app_id: ApplicationId,
    pub session_id: Option<String>,
    pub trace: Option<TraceContext>,
}

impl QueryTaskGoalsCommand {
    /// Create a goals query with optional session scope.
    pub fn new(
        app_id: ApplicationId,
        session_id: Option<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            app_id,
            session_id: session_id.map(|value| value.trim().to_string()),
            trace,
        }
    }
}

/// Command to query why a session's pending tasks can or cannot be claimed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTaskClaimDiagnosticsCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
    pub trace: Option<TraceContext>,
}

impl QueryTaskClaimDiagnosticsCommand {
    /// Create a session-scoped claim diagnostics query.
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

/// Replay-safe task board result preserving shell response contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoardQueryResult {
    pub todos: Vec<TodoItem>,
    pub count: usize,
}

/// Aggregate task progress result used by shell diagnostics and task loops.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgressSummary {
    pub total: usize,
    pub pending: usize,
    pub assigned: usize,
    pub in_progress: usize,
    pub pending_review: usize,
    pub needs_optimization: usize,
    pub completed: usize,
    pub blocked: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub all_done: bool,
}

/// Agent-scoped task board result preserving the current Web response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskBoardResult {
    pub agent: String,
    pub todos: Vec<TodoItem>,
    pub count: usize,
}

/// Goal list result preserving the current Web response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGoalsResult {
    pub goals: Vec<TodoGoal>,
    pub count: usize,
}
