//! Plan domain types — subtask and plan lifecycle value objects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// SubTaskState
// ---------------------------------------------------------------------------

/// Lifecycle state for a single subtask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskState {
    Todo,
    InProgress,
    Done,
    Abandoned,
}

// ---------------------------------------------------------------------------
// SubTask
// ---------------------------------------------------------------------------

/// A single atomic unit of work within a `Plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Short name for this subtask.
    pub name: String,
    /// Detailed description of what needs to be done.
    pub description: String,
    /// What a successful outcome looks like.
    pub expected_outcome: String,
    /// Recorded outcome after the subtask is finished.
    pub outcome: Option<String>,
    /// Current lifecycle state.
    pub state: SubTaskState,
    /// When this subtask was created.
    pub created_at: DateTime<Utc>,
    /// When this subtask was finished or abandoned.
    pub finished_at: Option<DateTime<Utc>>,
}

impl SubTask {
    /// Construct a new subtask in `Todo` state (crate-internal factory for `Plan::add_subtask`).
    pub(crate) fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        expected_outcome: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            expected_outcome: expected_outcome.into(),
            outcome: None,
            state: SubTaskState::Todo,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    /// Returns true when this subtask is in a terminal state (`Done` or `Abandoned`).
    pub(crate) fn is_finished(&self) -> bool {
        matches!(self.state, SubTaskState::Done | SubTaskState::Abandoned)
    }
}

// ---------------------------------------------------------------------------

// PlanState
// ---------------------------------------------------------------------------

/// Lifecycle state for an entire plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Todo,
    InProgress,
    Done,
    Abandoned,
}

// ---------------------------------------------------------------------------
