//! Cross-crate callback type aliases for todo/plan orchestration hooks.
//!
//! **Callback pattern**: inject `Arc<dyn Fn(...) + Send + Sync>` at tool construction time
//! so `macaca-tools` stays decoupled from shell/runtime crates (PlanLoop startup, run_trace).

use std::sync::Arc;

use macaca_proto::TodoGoal;

/// Fires after a successful `review_task` store update (for run_trace / analytics).
pub type OnTodoReviewed = Arc<dyn Fn(macaca_proto::TaskId, String, bool) + Send + Sync>;

/// Callback invoked after a goal is created, allowing the web layer to
/// lazily start the PlanLoop without introducing a circular dependency.
pub type OnGoalCreated = Arc<dyn Fn() + Send + Sync>;

/// Called synchronously right after the goal is persisted (includes id + session_id).
pub type OnGoalRecorded = Arc<dyn Fn(TodoGoal) + Send + Sync>;
