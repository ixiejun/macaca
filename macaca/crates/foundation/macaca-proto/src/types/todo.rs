//! Todo board graph types: items, goals, ownership, and agent task references.
//!
//! Supports hierarchical task decomposition with traceable parent/child links
//! for audit and observability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identity::{ApplicationId, TaskId};

// ── Todo System Types ──

/// Status of a todo item in the Task Board system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    /// Waiting to be claimed by an agent
    Pending,
    /// Agent has acknowledged the task
    Assigned,
    /// Agent is actively working on it
    InProgress,
    /// Agent finished, awaiting Plan Agent verification
    PendingReview,
    /// Verification failed, agent should retry with suggestions
    NeedsOptimization,
    /// Plan Agent verified – done
    Completed,
    /// Blocked by dependency tasks
    Blocked,
    /// Cancelled by Plan Agent
    Cancelled,
    /// Exceeded max attempts, needs human intervention
    Failed,
}

/// Service-owned classification for a task graph entry.
///
/// The value intentionally describes the owning Macaca service boundary instead
/// of an application, workflow, model, driver, provider, or business domain.
/// Application-execution projections use this marker to decide which tasks are
/// authoritative terminal facts for a run, while compatibility and diagnostic
/// tasks remain visible for audit without being allowed to fail an unrelated
/// execution.  The default keeps legacy persisted tasks out of the
/// application-execution terminal path unless a service explicitly marks them
/// as authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskGraphOwner {
    /// Task entries that are authoritative for `service.application_execution`
    /// run completion and failure projection.
    #[serde(alias = "ApplicationExecution")]
    ApplicationExecution,
    /// Existing task-service, scheduler, chat, or goal-loop entries whose
    /// lifecycle is visible on the board but is not an application-execution
    /// terminal source unless a higher-level service explicitly binds it.
    #[default]
    #[serde(alias = "TaskServiceNative")]
    TaskServiceNative,
    /// Compatibility fallback entries created while migrating legacy planner
    /// and Web loop behavior behind the Task Service boundary.
    #[serde(alias = "TaskServiceCompatibility")]
    TaskServiceCompatibility,
    /// Diagnostic entries that explain observations or failures but should
    /// never drive terminal execution state.
    #[serde(alias = "DiagnosticOnly")]
    DiagnosticOnly,
}

impl TaskGraphOwner {
    /// Return true when this graph owner may drive application-execution
    /// terminal aggregation.
    pub fn is_application_execution_authoritative(self) -> bool {
        matches!(self, Self::ApplicationExecution)
    }

    /// Stable label used in service metadata, trace events, and sanitized logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationExecution => "application_execution",
            Self::TaskServiceNative => "task_service_native",
            Self::TaskServiceCompatibility => "task_service_compatibility",
            Self::DiagnosticOnly => "diagnostic_only",
        }
    }
}

/// A single work item on an agent's task board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: TaskId,
    pub application_id: ApplicationId,
    /// Session this task belongs to (None = global / cross-session)
    #[serde(default)]
    pub session_id: Option<String>,
    /// Which agent this task is assigned to
    pub assigned_agent: String,
    /// Who created it (usually the plan agent / coordinator)
    pub created_by: String,
    /// Service boundary that owns this task entry for terminal aggregation.
    ///
    /// This field is a generic service classification.  It prevents a
    /// compatibility fallback task or diagnostic task from being interpreted as
    /// the authoritative terminal state of an application-execution run.  It is
    /// never allowed to encode application names, workflow names, provider
    /// names, programming languages, or product-domain semantics.
    #[serde(default)]
    pub graph_owner: TaskGraphOwner,
    /// Stable identity of the task graph that this task belongs to.
    ///
    /// The value is a service-owned execution graph identifier.  It lets the
    /// Task Service admit many tasks into the same authoritative graph while
    /// rejecting a second authoritative graph for the same application
    /// execution session.  The identifier must not contain workflow names,
    /// application names, provider names, programming languages, or business
    /// semantics; callers should use run/session scoped opaque ids.
    #[serde(default)]
    pub graph_id: Option<String>,

    // ── content ──
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    /// Extra context from parent goal or prior tasks
    pub context: Option<String>,

    // ── lifecycle ──
    pub status: TodoStatus,
    pub priority: u8,
    /// Execution order within this agent+session scope (1-based, ascending).
    /// Lower numbers execute first. 0 means unassigned (legacy data).
    #[serde(default)]
    pub sequence_number: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,

    // ── dependencies ──
    pub depends_on: Vec<TaskId>,
    pub parent_task: Option<TaskId>,

    // ── execution records ──
    /// Progress notes recorded during task execution
    #[serde(default)]
    pub progress_notes: Vec<String>,
    /// Agent's summary when submitting for review
    pub completion_summary: Option<String>,
    /// Plan Agent's feedback after review
    pub review_feedback: Option<String>,
    /// Suggestions when status = NeedsOptimization
    pub optimization_suggestions: Option<String>,
    pub attempt_count: u32,
    pub max_attempts: u32,
}

impl TodoItem {
    pub fn new(
        application_id: ApplicationId,
        session_id: Option<String>,
        assigned_agent: impl Into<String>,
        created_by: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        priority: u8,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            application_id,
            session_id,
            assigned_agent: assigned_agent.into(),
            created_by: created_by.into(),
            graph_owner: TaskGraphOwner::TaskServiceNative,
            graph_id: None,
            title: title.into(),
            description: description.into(),
            acceptance_criteria: Vec::new(),
            context: None,
            status: TodoStatus::Pending,
            priority,
            sequence_number: 0,
            created_at: now,
            updated_at: now,
            deadline: None,
            depends_on: Vec::new(),
            parent_task: None,
            progress_notes: Vec::new(),
            completion_summary: None,
            review_feedback: None,
            optimization_suggestions: None,
            attempt_count: 0,
            max_attempts: 3,
        }
    }
}

/// Result of a Plan Agent reviewing a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoReviewResult {
    pub passed: bool,
    pub feedback: String,
    /// Per-criterion verification: (criterion text, passed?)
    pub verified_criteria: Vec<(String, bool)>,
}

/// High-level goal submitted to an application's task space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoGoal {
    pub id: TaskId,
    pub application_id: ApplicationId,
    /// Session this goal belongs to (None = global / cross-session)
    #[serde(default)]
    pub session_id: Option<String>,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub status: TodoGoalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoGoalStatus {
    Pending,
    /// Planner is decomposing the goal into subtasks.
    Decomposing,
    InProgress,
    /// GoalEvaluator is assessing whether the goal is satisfied.
    Evaluating,
    Completed,
    /// Goal was cancelled (e.g. session cleanup).
    Cancelled,
    /// Goal decomposition or evaluation failed.
    Failed,
}

/// Reference to tasks owned by another agent, used for cross-agent dependency declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTaskRef {
    /// Depend on ALL tasks assigned to the named agent.
    AllTasks { agent: String },
    /// Depend on a specific task (matched by title) assigned to the named agent.
    SpecificTask { agent: String, title: String },
}

impl TodoGoal {
    pub fn new(
        application_id: ApplicationId,
        session_id: Option<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: TaskId::new(),
            application_id,
            session_id,
            description: description.into(),
            created_at: Utc::now(),
            status: TodoGoalStatus::Pending,
        }
    }
}
