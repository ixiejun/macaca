//! Agent lifecycle, manifest, fork, permission, and runtime status types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identity::AgentId;

// ── Agent Types ──

/// Lifecycle state of an agent (static).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Created,
    Running,
    Suspended,
    Terminated,
}

/// Runtime activity status of an agent (dynamic).
/// Simplified to 4 core states: IDLE, WORKING, ERROR, THINKING.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentActivity {
    /// Agent is idle, waiting for tasks.
    Idle,
    /// Agent is actively working (executing tools, processing).
    Working {
        /// Brief description of what's being worked on.
        context: String,
    },
    /// Agent encountered an error.
    Error {
        /// Error description.
        message: String,
    },
    /// Agent is thinking/processing (LLM call in progress).
    Thinking {
        /// Brief description of what's being processed.
        context: String,
    },
}

impl Default for AgentActivity {
    fn default() -> Self {
        Self::Idle
    }
}

/// Lifecycle state of a forked (child) agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkState {
    /// Fork created, waiting to start execution.
    Pending,
    /// Fork is actively executing.
    Running,
    /// Fork delegated a task and is waiting for hook callback.
    WaitingForHook,
    /// Fork's delegated task completed successfully.
    Completed,
    /// Fork's delegated task failed.
    Failed { error: String },
    /// Fork completed and merged back to parent.
    Merged,
    /// Fork was cancelled.
    Cancelled,
}

/// Criteria for accepting a fork's result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    /// Human-readable description of success criteria.
    pub description: String,
    /// Required artifacts (file paths, URLs, etc.).
    pub required_artifacts: Vec<String>,
    /// Whether to auto-accept on success without validation.
    pub auto_accept: bool,
}

impl Default for AcceptanceCriteria {
    fn default() -> Self {
        Self {
            description: "Task completed successfully".into(),
            required_artifacts: vec![],
            auto_accept: false,
        }
    }
}

/// Result of validating a fork's output against acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationResult {
    /// Validation passed.
    Accepted,
    /// Validation failed.
    Rejected { reason: String },
    /// Unable to validate (e.g., timeout, missing artifacts).
    Unavailable { reason: String },
}

/// Runtime status of an agent, combining lifecycle state and current activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeStatus {
    /// Agent's unique ID.
    pub agent_id: AgentId,
    /// Agent's name.
    pub name: String,
    /// Lifecycle state (Created, Running, etc.).
    pub state: AgentState,
    /// Current activity (what the agent is doing right now).
    pub activity: AgentActivity,
    /// Timestamp of last status update.
    pub updated_at: DateTime<Utc>,
    /// Current task description (if any).
    pub current_task: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// System agent — full access (kernel mode)
    System,
    /// User agent — restricted access (user mode)
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub allowed_tools: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub network_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub permission: Permission,
    pub state: AgentState,
    pub created_at: DateTime<Utc>,
    /// Preferred LLM model for this agent (e.g., "", "gpt-4o").
    /// If empty, uses the application's default model.
    #[serde(default)]
    pub model: String,
}
