//! Plan notebook — agent self-planning with sequential subtask execution.
//!
//! `PlanNotebook` lets an agent decompose a complex goal into ordered subtasks,
//! track progress through them, and receive hint messages guiding its next action.
//!
//! Invariant: at most one subtask can be `InProgress` at any time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::Msg;

// ---------------------------------------------------------------------------
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
    fn new(
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

    /// Returns true if this subtask is in a terminal state.
    fn is_finished(&self) -> bool {
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
// PlanError
// ---------------------------------------------------------------------------

/// Errors returned by `Plan` and `PlanNotebook` operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlanError {
    /// Another subtask is already `InProgress`; finish it before starting another.
    #[error("Another subtask is already in progress")]
    AnotherInProgress,

    /// The given subtask index does not exist.
    #[error("Subtask index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    /// The requested state transition is not allowed.
    #[error("Invalid state transition: {from:?} → {to:?}")]
    InvalidTransition {
        from: SubTaskState,
        to: SubTaskState,
    },

    /// There is no active plan to operate on.
    #[error("No active plan")]
    NoPlan,

    /// The given historical plan index does not exist.
    #[error("Historical plan index out of bounds: {0}")]
    HistoricalIndexOutOfBounds(usize),
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

/// A goal decomposed into an ordered list of subtasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Unique plan identifier.
    pub id: String,
    /// Short name for this plan.
    pub name: String,
    /// Detailed description of the overall goal.
    pub description: String,
    /// What a successful plan outcome looks like.
    pub expected_outcome: String,
    /// Ordered list of subtasks.
    pub subtasks: Vec<SubTask>,
    /// Current lifecycle state of this plan.
    pub state: PlanState,
    /// Recorded outcome after the plan is finished.
    pub outcome: Option<String>,
    /// When this plan was created.
    pub created_at: DateTime<Utc>,
    /// When this plan was finished or abandoned.
    pub finished_at: Option<DateTime<Utc>>,
}

impl Plan {
    /// Create a new plan in `Todo` state with no subtasks.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        expected_outcome: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            expected_outcome: expected_outcome.into(),
            subtasks: Vec::new(),
            state: PlanState::Todo,
            outcome: None,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    /// Return the currently active subtask (`InProgress`), if any.
    pub fn current_subtask(&self) -> Option<&SubTask> {
        self.subtasks
            .iter()
            .find(|s| s.state == SubTaskState::InProgress)
    }

    /// Return the index of the first `Todo` subtask, if any.
    pub fn next_todo_subtask(&self) -> Option<usize> {
        self.subtasks
            .iter()
            .position(|s| s.state == SubTaskState::Todo)
    }

    /// Returns `true` when every subtask is in a terminal state (`Done` or `Abandoned`).
    pub fn all_subtasks_finished(&self) -> bool {
        !self.subtasks.is_empty() && self.subtasks.iter().all(|s| s.is_finished())
    }

    /// Append a new `Todo` subtask to the end of the list.
    pub fn add_subtask(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        expected_outcome: impl Into<String>,
    ) {
        self.subtasks
            .push(SubTask::new(name, description, expected_outcome));
    }

    /// Transition subtask at `index` to `InProgress`.
    ///
    /// # Errors
    ///
    /// - `IndexOutOfBounds` if `index` is invalid.
    /// - `InvalidTransition` if the subtask is not in `Todo` state.
    /// - `AnotherInProgress` if a different subtask is already running.
    pub fn start_subtask(&mut self, index: usize) -> Result<(), PlanError> {
        if index >= self.subtasks.len() {
            return Err(PlanError::IndexOutOfBounds(index));
        }

        // Enforce single-in-progress invariant.
        let already_running = self
            .subtasks
            .iter()
            .enumerate()
            .any(|(i, s)| i != index && s.state == SubTaskState::InProgress);
        if already_running {
            return Err(PlanError::AnotherInProgress);
        }

        let subtask = &self.subtasks[index];
        if subtask.state != SubTaskState::Todo {
            return Err(PlanError::InvalidTransition {
                from: subtask.state,
                to: SubTaskState::InProgress,
            });
        }

        self.subtasks[index].state = SubTaskState::InProgress;
        // Promote plan state to InProgress on first subtask start.
        if self.state == PlanState::Todo {
            self.state = PlanState::InProgress;
        }
        Ok(())
    }

    /// Transition subtask at `index` to `Done`, recording its `outcome`.
    ///
    /// After finishing, automatically starts the next `Todo` subtask (if any).
    ///
    /// # Errors
    ///
    /// - `IndexOutOfBounds` if `index` is invalid.
    /// - `InvalidTransition` if the subtask is not `InProgress`.
    pub fn finish_subtask(
        &mut self,
        index: usize,
        outcome: impl Into<String>,
    ) -> Result<(), PlanError> {
        if index >= self.subtasks.len() {
            return Err(PlanError::IndexOutOfBounds(index));
        }

        let subtask = &self.subtasks[index];
        if subtask.state != SubTaskState::InProgress {
            return Err(PlanError::InvalidTransition {
                from: subtask.state,
                to: SubTaskState::Done,
            });
        }

        self.subtasks[index].state = SubTaskState::Done;
        self.subtasks[index].outcome = Some(outcome.into());
        self.subtasks[index].finished_at = Some(Utc::now());

        // Auto-start the next Todo subtask.
        if let Some(next_idx) = self.next_todo_subtask() {
            self.subtasks[next_idx].state = SubTaskState::InProgress;
        }

        Ok(())
    }

    /// Transition subtask at `index` to `Abandoned`.
    ///
    /// # Errors
    ///
    /// - `IndexOutOfBounds` if `index` is invalid.
    /// - `InvalidTransition` if the subtask is already in a terminal state.
    pub fn abandon_subtask(&mut self, index: usize) -> Result<(), PlanError> {
        if index >= self.subtasks.len() {
            return Err(PlanError::IndexOutOfBounds(index));
        }

        let subtask = &self.subtasks[index];
        if subtask.is_finished() {
            return Err(PlanError::InvalidTransition {
                from: subtask.state,
                to: SubTaskState::Abandoned,
            });
        }

        self.subtasks[index].state = SubTaskState::Abandoned;
        self.subtasks[index].finished_at = Some(Utc::now());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PlanNotebook
// ---------------------------------------------------------------------------

/// Agent-level plan manager: one active plan at a time, with history.
///
/// When an agent starts a new plan, any existing active plan is archived
/// (moved to `historical_plans`). Historical plans can be recovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNotebook {
    current_plan: Option<Plan>,
    historical_plans: Vec<Plan>,
}

impl PlanNotebook {
    /// Create an empty notebook with no plans.
    pub fn new() -> Self {
        Self {
            current_plan: None,
            historical_plans: Vec::new(),
        }
    }

    /// Create a new plan, archiving any existing active plan first.
    ///
    /// Returns a reference to the newly created plan.
    pub fn create_plan(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        expected_outcome: impl Into<String>,
    ) -> &Plan {
        // Archive existing plan.
        if let Some(old) = self.current_plan.take() {
            self.historical_plans.push(old);
        }
        self.current_plan = Some(Plan::new(name, description, expected_outcome));
        self.current_plan.as_ref().unwrap()
    }

    /// Return a shared reference to the active plan, if any.
    pub fn current_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    /// Return a mutable reference to the active plan, if any.
    pub fn current_plan_mut(&mut self) -> Option<&mut Plan> {
        self.current_plan.as_mut()
    }

    /// Mark the active plan as `Done` and archive it.
    ///
    /// # Errors
    ///
    /// Returns `PlanError::NoPlan` if there is no active plan.
    pub fn finish_plan(&mut self, outcome: impl Into<String>) -> Result<(), PlanError> {
        let plan = self.current_plan.as_mut().ok_or(PlanError::NoPlan)?;
        plan.state = PlanState::Done;
        plan.outcome = Some(outcome.into());
        plan.finished_at = Some(Utc::now());

        let finished = self.current_plan.take().unwrap();
        self.historical_plans.push(finished);
        Ok(())
    }

    /// Mark the active plan as `Abandoned` and archive it.
    ///
    /// # Errors
    ///
    /// Returns `PlanError::NoPlan` if there is no active plan.
    pub fn abandon_plan(&mut self) -> Result<(), PlanError> {
        let plan = self.current_plan.as_mut().ok_or(PlanError::NoPlan)?;
        plan.state = PlanState::Abandoned;
        plan.finished_at = Some(Utc::now());

        let abandoned = self.current_plan.take().unwrap();
        self.historical_plans.push(abandoned);
        Ok(())
    }

    /// Return a slice of all archived plans, oldest first.
    pub fn historical_plans(&self) -> &[Plan] {
        &self.historical_plans
    }

    /// Move historical plan at `index` back to the active slot.
    ///
    /// Any currently active plan is archived first.
    ///
    /// # Errors
    ///
    /// Returns `PlanError::HistoricalIndexOutOfBounds` if `index` is invalid.
    pub fn recover_plan(&mut self, index: usize) -> Result<(), PlanError> {
        if index >= self.historical_plans.len() {
            return Err(PlanError::HistoricalIndexOutOfBounds(index));
        }

        // Archive the current plan if present.
        if let Some(cur) = self.current_plan.take() {
            self.historical_plans.push(cur);
        }

        let recovered = self.historical_plans.remove(index);
        self.current_plan = Some(recovered);
        Ok(())
    }

    /// Generate a hint `Msg` guiding the agent toward its next action.
    ///
    /// Returns `None` only when there is no active plan and no hint is needed
    /// (callers should still consider the no-plan case via the `Some` variant).
    pub fn hint(&self) -> Option<Msg> {
        let text = match &self.current_plan {
            None => {
                "<system-hint>You have no active plan. Consider creating one with create_plan if the task is complex.</system-hint>".to_string()
            }
            Some(plan) => {
                if plan.subtasks.is_empty() {
                    format!(
                        "<system-hint>Plan '{}' created. Add subtasks and start the first one.</system-hint>",
                        plan.name
                    )
                } else if let Some(in_progress) = plan.current_subtask() {
                    format!(
                        "<system-hint>Working on subtask '{}': {}. Expected: {}</system-hint>",
                        in_progress.name,
                        in_progress.description,
                        in_progress.expected_outcome,
                    )
                } else if plan.all_subtasks_finished() {
                    "<system-hint>All subtasks complete. Consider finishing the plan with finish_plan.</system-hint>".to_string()
                } else {
                    // Has subtasks but none are InProgress (all Todo).
                    let n = plan.subtasks.iter().filter(|s| s.state == SubTaskState::Todo).count();
                    format!(
                        "<system-hint>Plan '{}' has {} subtasks. Start the first one.</system-hint>",
                        plan.name, n
                    )
                }
            }
        };

        Some(Msg::system(text))
    }
}

impl Default for PlanNotebook {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
