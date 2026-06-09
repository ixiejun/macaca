//! PlanNotebook — agent plan manager with history archive (**Memento** + **Facade**).
//!
//! `PlanNotebook` maintains at most one *active* `Plan` while archiving superseded
//! plans into `historical_plans`. It implements `StateModule` so the entire notebook
//! (active + history) can be snapshotted and restored across agent restarts.

use chrono::Utc;
use serde_json::Value;

use crate::message::Msg;
use crate::state::{StateError, StateModule};

use super::aggregate::Plan;
use super::error::PlanError;
use super::types::{PlanState, SubTaskState};

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
        let name = name.into();
        // Archive existing plan before replacing the active slot.
        if let Some(old) = self.current_plan.take() {
            tracing::debug!(
                target = "macaca_framework::plan::notebook",
                archived_plan_id = %old.id,
                archived_plan_name = %old.name,
                "archiving superseded active plan before create_plan"
            );
            self.historical_plans.push(old);
        }
        let plan = Plan::new(name, description, expected_outcome);
        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            plan_id = %plan.id,
            plan_name = %plan.name,
            historical_count = self.historical_plans.len(),
            "created new active plan"
        );
        self.current_plan = Some(plan);
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
        let plan = self.current_plan.as_mut().ok_or_else(|| {
            tracing::warn!(
                target = "macaca_framework::plan::notebook",
                "finish_plan rejected: no active plan"
            );
            PlanError::NoPlan
        })?;
        plan.state = PlanState::Done;
        plan.outcome = Some(outcome.into());
        plan.finished_at = Some(Utc::now());

        let finished = self.current_plan.take().unwrap();
        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            plan_id = %finished.id,
            plan_name = %finished.name,
            historical_count_after = self.historical_plans.len() + 1,
            "finished and archived active plan"
        );
        self.historical_plans.push(finished);
        Ok(())
    }

    /// Mark the active plan as `Abandoned` and archive it.
    ///
    /// # Errors
    ///
    /// Returns `PlanError::NoPlan` if there is no active plan.
    pub fn abandon_plan(&mut self) -> Result<(), PlanError> {
        let plan = self.current_plan.as_mut().ok_or_else(|| {
            tracing::warn!(
                target = "macaca_framework::plan::notebook",
                "abandon_plan rejected: no active plan"
            );
            PlanError::NoPlan
        })?;
        plan.state = PlanState::Abandoned;
        plan.finished_at = Some(Utc::now());

        let abandoned = self.current_plan.take().unwrap();
        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            plan_id = %abandoned.id,
            plan_name = %abandoned.name,
            "abandoned and archived active plan"
        );
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
            tracing::warn!(
                target = "macaca_framework::plan::notebook",
                index = index,
                historical_count = self.historical_plans.len(),
                "recover_plan rejected: historical index out of bounds"
            );
            return Err(PlanError::HistoricalIndexOutOfBounds(index));
        }

        // Archive the current plan if present.
        if let Some(cur) = self.current_plan.take() {
            tracing::debug!(
                target = "macaca_framework::plan::notebook",
                archived_plan_id = %cur.id,
                "archiving current plan before recover_plan"
            );
            self.historical_plans.push(cur);
        }

        let recovered = self.historical_plans.remove(index);
        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            plan_id = %recovered.id,
            plan_name = %recovered.name,
            recovered_from_index = index,
            "recovered historical plan to active slot"
        );
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

        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            has_active_plan = self.current_plan.is_some(),
            hint_chars = text.len(),
            "generated plan hint message"
        );
        Some(Msg::system(text))
    }
}

impl Default for PlanNotebook {
    fn default() -> Self {
        Self::new()
    }
}

impl StateModule for PlanNotebook {
    fn state_dict(&self) -> Value {
        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            has_current = self.current_plan.is_some(),
            historical_count = self.historical_plans.len(),
            "serializing plan notebook state (Memento snapshot)"
        );
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

        tracing::debug!(
            target = "macaca_framework::plan::notebook",
            has_current = self.current_plan.is_some(),
            historical_count = self.historical_plans.len(),
            "restored plan notebook state from Memento snapshot"
        );
        Ok(())
    }

    fn module_name(&self) -> &str {
        "plan_notebook"
    }
}
