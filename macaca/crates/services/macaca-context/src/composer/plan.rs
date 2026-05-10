//! [`ContextPlan`]: immutable snapshot produced by the builder step.
//!
//! The composer fixes “included / skipped / reason” before messages reach the engine so audits
//! do not need to re-run providers.

use serde::{Deserialize, Serialize};

use crate::composer::candidate::ContextCandidate;

/// Serializable reason when a candidate is dropped (surfaced on [`crate::report::ContextReport`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPlanDecision {
    pub source_id: String,
    pub reason_code: String,
    pub message: String,
}

/// Full output of one composer run: selected set, skipped set, and plan id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextPlan {
    /// Independent from `ContextReport.request_id`; identifies this composer pass only.
    pub plan_id: String,
    pub selected: Vec<ContextCandidate>,
    pub skipped: Vec<ContextPlanDecision>,
}

impl ContextPlan {
    /// Empty plan: still a structured value even when no provider contributes or everything was cut.
    pub fn empty(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            selected: Vec::new(),
            skipped: Vec::new(),
        }
    }
}
