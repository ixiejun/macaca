//! Plan notebook — agent self-planning with sequential subtask execution.
//!
//! `PlanNotebook` lets an agent decompose a complex goal into ordered subtasks,
//! track progress through them, and receive hint messages guiding its next action.
//!
//! Invariant: at most one subtask can be `InProgress` at any time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::message::Msg;
use crate::state::{StateError, StateModule};

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
// StateModule for PlanNotebook
// ---------------------------------------------------------------------------

impl StateModule for PlanNotebook {
    fn state_dict(&self) -> Value {
        serde_json::json!({
            "current_plan": self.current_plan,
            "historical_plans": self.historical_plans,
        })
    }

    fn load_state_dict(&mut self, state: Value) -> Result<(), StateError> {
        self.current_plan = match state.get("current_plan") {
            Some(v) if !v.is_null() => {
                let plan: Plan = serde_json::from_value(v.clone())
                    .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
                Some(plan)
            }
            _ => None,
        };

        self.historical_plans = match state.get("historical_plans") {
            Some(v) if !v.is_null() => serde_json::from_value(v.clone())
                .map_err(|e| StateError::DeserializeFailed(e.to_string()))?,
            _ => Vec::new(),
        };

        Ok(())
    }

    fn module_name(&self) -> &str {
        "plan_notebook"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 1. Create plan + subtasks
    // -----------------------------------------------------------------------
    #[test]
    fn test_create_plan() {
        let mut nb = PlanNotebook::new();
        let plan = nb.create_plan("Fix bug", "Fix the login bug", "Login works");
        assert_eq!(plan.name, "Fix bug");
        assert_eq!(plan.state, PlanState::Todo);
        assert!(plan.subtasks.is_empty());

        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Reproduce", "Reproduce the bug", "Bug reproduced");
        plan.add_subtask("Fix", "Apply the fix", "Bug fixed");

        assert_eq!(nb.current_plan().unwrap().subtasks.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 2. Sequential subtasks — only one InProgress at a time
    // -----------------------------------------------------------------------
    #[test]
    fn test_subtask_sequential() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.add_subtask("Task B", "Do B", "B done");

        plan.start_subtask(0).unwrap();
        // Starting the second while first is in-progress must fail.
        let err = plan.start_subtask(1);
        assert!(matches!(err, Err(PlanError::AnotherInProgress)));
    }

    // -----------------------------------------------------------------------
    // 3. Finish subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_finish_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.add_subtask("Task B", "Do B", "B done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "A completed").unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[0].outcome.as_deref(), Some("A completed"));
        // Auto-started next subtask.
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 4. Finish entire plan — archived to historical
    // -----------------------------------------------------------------------
    #[test]
    fn test_finish_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        nb.finish_plan("All done").unwrap();

        assert!(nb.current_plan().is_none());
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].state, PlanState::Done);
        assert_eq!(
            nb.historical_plans()[0].outcome.as_deref(),
            Some("All done")
        );
    }

    // -----------------------------------------------------------------------
    // 5. Abandon subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_abandon_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.start_subtask(0).unwrap();
        plan.abandon_subtask(0).unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Abandoned);
        assert!(plan.subtasks[0].finished_at.is_some());

        // Cannot abandon again (already terminal).
        let err = plan.abandon_subtask(0);
        assert!(matches!(err, Err(PlanError::InvalidTransition { .. })));
    }

    // -----------------------------------------------------------------------
    // 6. Recover historical plan
    // -----------------------------------------------------------------------
    #[test]
    fn test_recover_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Old Plan", "old desc", "old outcome");
        nb.finish_plan("done").unwrap();

        nb.create_plan("New Plan", "new desc", "new outcome");

        // Recover the old plan (index 0 in historical).
        nb.recover_plan(0).unwrap();

        assert_eq!(nb.current_plan().unwrap().name, "Old Plan");
        // "New Plan" should now be in historical.
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].name, "New Plan");
    }

    // -----------------------------------------------------------------------
    // 7. Hint — no plan
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_no_plan() {
        let nb = PlanNotebook::new();
        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("no active plan"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 8. Hint — subtask in progress
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_in_progress() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A carefully", "A is done");
        plan.start_subtask(0).unwrap();

        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("Task A"), "got: {text}");
        assert!(text.contains("Do A carefully"), "got: {text}");
        assert!(text.contains("A is done"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 9. Hint — all subtasks done
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_all_done() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task A", "Do A", "A done");
        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "completed").unwrap();

        let hint = nb.hint().unwrap();
        let text = hint.get_text();
        assert!(text.contains("All subtasks complete"), "got: {text}");
    }

    // -----------------------------------------------------------------------
    // 10. StateModule round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn test_state_roundtrip() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan A", "desc A", "outcome A");
        nb.finish_plan("A done").unwrap();
        nb.create_plan("Plan B", "desc B", "outcome B");
        {
            let plan = nb.current_plan_mut().unwrap();
            plan.add_subtask("Task 1", "Do 1", "1 done");
            plan.start_subtask(0).unwrap();
        }

        let state = nb.state_dict();

        let mut restored = PlanNotebook::new();
        restored.load_state_dict(state).unwrap();

        // Current plan preserved.
        let cur = restored.current_plan().unwrap();
        assert_eq!(cur.name, "Plan B");
        assert_eq!(cur.subtasks.len(), 1);
        assert_eq!(cur.subtasks[0].state, SubTaskState::InProgress);

        // Historical plan preserved.
        assert_eq!(restored.historical_plans().len(), 1);
        assert_eq!(restored.historical_plans()[0].name, "Plan A");
        assert_eq!(restored.historical_plans()[0].state, PlanState::Done);
    }

    // -----------------------------------------------------------------------
    // 11. Single active invariant — starting second while first runs fails
    // -----------------------------------------------------------------------
    #[test]
    fn test_single_active_invariant() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        // Trying to start B while A is in progress.
        assert!(matches!(
            plan.start_subtask(1),
            Err(PlanError::AnotherInProgress)
        ));
        assert!(matches!(
            plan.start_subtask(2),
            Err(PlanError::AnotherInProgress)
        ));
        // Only A is InProgress.
        assert_eq!(
            plan.subtasks
                .iter()
                .filter(|s| s.state == SubTaskState::InProgress)
                .count(),
            1
        );
    }

    // -----------------------------------------------------------------------
    // 12. Auto-advance after finishing current subtask
    // -----------------------------------------------------------------------
    #[test]
    fn test_auto_advance_subtask() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "completed A").unwrap();

        // B should be auto-advanced to InProgress.
        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
        assert_eq!(plan.subtasks[2].state, SubTaskState::Todo);

        plan.finish_subtask(1, "completed B").unwrap();
        // C auto-advanced.
        assert_eq!(plan.subtasks[2].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 13. All subtasks done completes plan flow
    // -----------------------------------------------------------------------
    #[test]
    fn test_all_subtasks_done_completes_plan() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "done A").unwrap();
        plan.finish_subtask(1, "done B").unwrap(); // B was auto-started

        assert!(plan.all_subtasks_finished());
        assert_eq!(plan.subtasks[0].state, SubTaskState::Done);
        assert_eq!(plan.subtasks[1].state, SubTaskState::Done);

        // Hint should say all done.
        let hint_text = nb.hint().unwrap().get_text();
        assert!(
            hint_text.contains("All subtasks complete"),
            "got: {hint_text}"
        );
    }

    // -----------------------------------------------------------------------
    // 14. Illegal state transition: Done → InProgress
    // -----------------------------------------------------------------------
    #[test]
    fn test_illegal_state_transition() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");

        plan.start_subtask(0).unwrap();
        plan.finish_subtask(0, "done").unwrap();

        // A is now Done, no other subtask running. Trying to start it again should fail.
        let err = plan.start_subtask(0);
        assert!(matches!(err, Err(PlanError::InvalidTransition { .. })));
    }

    // -----------------------------------------------------------------------
    // 15. Empty plan behavior
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_plan_behavior() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Empty Plan", "no subtasks", "nothing");
        let plan = nb.current_plan().unwrap();

        // No subtasks → all_subtasks_finished is false (empty check).
        assert!(!plan.all_subtasks_finished());
        assert!(plan.current_subtask().is_none());
        assert!(plan.next_todo_subtask().is_none());

        // Hint should suggest adding subtasks.
        let hint_text = nb.hint().unwrap().get_text();
        assert!(hint_text.contains("Add subtasks"), "got: {hint_text}");
    }

    // -----------------------------------------------------------------------
    // 16. Abandon subtask, plan continues with others
    // -----------------------------------------------------------------------
    #[test]
    fn test_abandon_subtask_plan_continues() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("A", "do a", "a done");
        plan.add_subtask("B", "do b", "b done");
        plan.add_subtask("C", "do c", "c done");

        plan.start_subtask(0).unwrap();
        plan.abandon_subtask(0).unwrap();

        assert_eq!(plan.subtasks[0].state, SubTaskState::Abandoned);
        // B and C are still Todo — can start B.
        plan.start_subtask(1).unwrap();
        assert_eq!(plan.subtasks[1].state, SubTaskState::InProgress);
    }

    // -----------------------------------------------------------------------
    // 17. Plan history archive — new plan archives old
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_history_archive() {
        let mut nb = PlanNotebook::new();
        nb.create_plan("Plan 1", "first", "first outcome");
        // Creating a second plan archives the first.
        nb.create_plan("Plan 2", "second", "second outcome");

        assert_eq!(nb.current_plan().unwrap().name, "Plan 2");
        assert_eq!(nb.historical_plans().len(), 1);
        assert_eq!(nb.historical_plans()[0].name, "Plan 1");

        // Creating a third archives the second.
        nb.create_plan("Plan 3", "third", "third outcome");
        assert_eq!(nb.current_plan().unwrap().name, "Plan 3");
        assert_eq!(nb.historical_plans().len(), 2);
        assert_eq!(nb.historical_plans()[0].name, "Plan 1");
        assert_eq!(nb.historical_plans()[1].name, "Plan 2");
    }

    // -----------------------------------------------------------------------
    // 18. Hint output varies by state
    // -----------------------------------------------------------------------
    #[test]
    fn test_hint_output_varies_by_state() {
        let mut nb = PlanNotebook::new();

        // No plan.
        let h1 = nb.hint().unwrap().get_text();
        assert!(h1.contains("no active plan"), "got: {h1}");

        // Plan with subtasks but none started (Todo state).
        nb.create_plan("Plan", "desc", "outcome");
        let plan = nb.current_plan_mut().unwrap();
        plan.add_subtask("Task X", "do X", "X done");
        let h2 = nb.hint().unwrap().get_text();
        assert!(h2.contains("Start the first one"), "got: {h2}");

        // InProgress.
        nb.current_plan_mut().unwrap().start_subtask(0).unwrap();
        let h3 = nb.hint().unwrap().get_text();
        assert!(h3.contains("Task X"), "got: {h3}");
        assert!(h3.contains("Working on subtask"), "got: {h3}");

        // Done.
        nb.current_plan_mut()
            .unwrap()
            .finish_subtask(0, "ok")
            .unwrap();
        let h4 = nb.hint().unwrap().get_text();
        assert!(h4.contains("All subtasks complete"), "got: {h4}");

        // All hints are different.
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h3, h4);
    }

    // -----------------------------------------------------------------------
    // 19. PlanNotebook state_dict/load_state_dict with historical plans
    // -----------------------------------------------------------------------
    #[test]
    fn test_plan_state_module_roundtrip() {
        let mut nb = PlanNotebook::new();

        // Create & finish plan 1.
        nb.create_plan("Plan Alpha", "alpha desc", "alpha outcome");
        {
            let p = nb.current_plan_mut().unwrap();
            p.add_subtask("A1", "desc", "outcome");
            p.start_subtask(0).unwrap();
            p.finish_subtask(0, "done").unwrap();
        }
        nb.finish_plan("Alpha completed").unwrap();

        // Create plan 2 with an in-progress subtask.
        nb.create_plan("Plan Beta", "beta desc", "beta outcome");
        {
            let p = nb.current_plan_mut().unwrap();
            p.add_subtask("B1", "desc b1", "outcome b1");
            p.add_subtask("B2", "desc b2", "outcome b2");
            p.start_subtask(0).unwrap();
        }

        // Save and restore.
        let state = nb.state_dict();
        let mut restored = PlanNotebook::new();
        restored.load_state_dict(state).unwrap();

        // Current plan preserved.
        let cur = restored.current_plan().unwrap();
        assert_eq!(cur.name, "Plan Beta");
        assert_eq!(cur.subtasks.len(), 2);
        assert_eq!(cur.subtasks[0].state, SubTaskState::InProgress);
        assert_eq!(cur.subtasks[1].state, SubTaskState::Todo);

        // Historical plan preserved.
        assert_eq!(restored.historical_plans().len(), 1);
        assert_eq!(restored.historical_plans()[0].name, "Plan Alpha");
        assert_eq!(restored.historical_plans()[0].state, PlanState::Done);
        assert_eq!(
            restored.historical_plans()[0].outcome.as_deref(),
            Some("Alpha completed")
        );
    }
}
