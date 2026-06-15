//! Task request/result types, priority ordering, and execution status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identity::{AgentId, TaskId};
use super::llm::TokenUsage;

// ── Task Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub description: String,
    pub priority: TaskPriority,
    pub requester: String,
}

impl TaskRequest {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            priority: TaskPriority::Normal,
            requester: String::from("user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assigned_agent: Option<AgentId>,
    pub subtasks: Vec<TaskId>,
    pub parent: Option<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub output: String,
    /// Structured failure message when `success` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub artifacts: Vec<String>,
    pub completed_at: DateTime<Utc>,
    /// Optional LLM token accounting captured by the execution service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<TokenUsage>,
}

/// Provider-neutral description of an execution-capable application agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier.
    pub id: String,
    /// Stable agent name exposed by application metadata.
    pub name: String,
    /// Provider-neutral capability labels used by routing policies.
    pub capabilities: Vec<String>,
    /// Current running-task count reported by the execution service.
    pub current_load: u32,
    /// Maximum concurrent work accepted by this agent.
    pub max_load: u32,
    /// Whether the agent can currently accept work.
    pub available: bool,
}
