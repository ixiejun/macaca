//! Plan aggregate — ordered subtask list with single-in-progress invariant (**State Machine**).
//!
//! `Plan` is the domain aggregate root: it owns an ordered `Vec<SubTask>` and enforces
//! the invariant that at most one subtask may be `InProgress` at any time. State
//! transitions are explicit methods that return `PlanError` on invalid moves, making
//! the lifecycle auditable via `tracing` hooks at each transition boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::PlanError;
use super::types::{PlanState, SubTask, SubTaskState};

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
        let name = name.into();
        tracing::debug!(
            target = "macaca_framework::plan::aggregate",
            plan_name = %name,
            "created new plan aggregate in Todo state"
        );
        Self {
            id: Uuid::new_v4().to_string(),
            name,
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
        let name = name.into();
        tracing::debug!(
            target = "macaca_framework::plan::aggregate",
            plan_id = %self.id,
            subtask_name = %name,
            subtask_count_after = self.subtasks.len() + 1,
            "appended subtask to plan"
        );
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
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                subtask_count = self.subtasks.len(),
                "start_subtask rejected: index out of bounds"
            );
            return Err(PlanError::IndexOutOfBounds(index));
        }

        // Enforce single-in-progress invariant.
        let already_running = self
            .subtasks
            .iter()
            .enumerate()
            .any(|(i, s)| i != index && s.state == SubTaskState::InProgress);
        if already_running {
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                "start_subtask rejected: another subtask already InProgress"
            );
            return Err(PlanError::AnotherInProgress);
        }

        let subtask = &self.subtasks[index];
        if subtask.state != SubTaskState::Todo {
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                from = ?subtask.state,
                to = ?SubTaskState::InProgress,
                "start_subtask rejected: invalid state transition"
            );
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
        tracing::debug!(
            target = "macaca_framework::plan::aggregate",
            plan_id = %self.id,
            index = index,
            subtask_name = %self.subtasks[index].name,
            plan_state = ?self.state,
            "subtask transitioned to InProgress"
        );
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
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                "finish_subtask rejected: index out of bounds"
            );
            return Err(PlanError::IndexOutOfBounds(index));
        }

        let subtask = &self.subtasks[index];
        if subtask.state != SubTaskState::InProgress {
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                from = ?subtask.state,
                to = ?SubTaskState::Done,
                "finish_subtask rejected: invalid state transition"
            );
            return Err(PlanError::InvalidTransition {
                from: subtask.state,
                to: SubTaskState::Done,
            });
        }

        self.subtasks[index].state = SubTaskState::Done;
        self.subtasks[index].outcome = Some(outcome.into());
        self.subtasks[index].finished_at = Some(Utc::now());

        // Auto-start the next Todo subtask.
        let auto_started = self.next_todo_subtask();
        if let Some(next_idx) = auto_started {
            self.subtasks[next_idx].state = SubTaskState::InProgress;
        }

        tracing::debug!(
            target = "macaca_framework::plan::aggregate",
            plan_id = %self.id,
            index = index,
            subtask_name = %self.subtasks[index].name,
            auto_started_next = auto_started.is_some(),
            "subtask transitioned to Done"
        );
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
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                "abandon_subtask rejected: index out of bounds"
            );
            return Err(PlanError::IndexOutOfBounds(index));
        }

        let subtask = &self.subtasks[index];
        if subtask.is_finished() {
            tracing::warn!(
                target = "macaca_framework::plan::aggregate",
                plan_id = %self.id,
                index = index,
                from = ?subtask.state,
                to = ?SubTaskState::Abandoned,
                "abandon_subtask rejected: subtask already terminal"
            );
            return Err(PlanError::InvalidTransition {
                from: subtask.state,
                to: SubTaskState::Abandoned,
            });
        }

        self.subtasks[index].state = SubTaskState::Abandoned;
        self.subtasks[index].finished_at = Some(Utc::now());
        tracing::debug!(
            target = "macaca_framework::plan::aggregate",
            plan_id = %self.id,
            index = index,
            subtask_name = %self.subtasks[index].name,
            "subtask transitioned to Abandoned"
        );
        Ok(())
    }
}
