//! Plan Agent scheduling loop for goal-driven task orchestration.
//!
//! Split from monolithic `plan_loop.rs` (P5 iteration 84) using a Facade module tree.
//! Continuously: process goals → review completed tasks → schedule new work.
//! Runs as a background tokio task, tied to the application's lifetime.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_task::plan_loop::*`.
//! - **Observer**: `PlanEvent` channel decouples scheduling from LLM execution.
//! - **Template Method**: `run_with_default_template` orchestrates cycle steps.
//! - **Strategy**: `GoalEvaluator` prompt/build/parse seam for quality checks.
//! - **Value Object**: `TaskSummary`, `PlanLoopConfig`.

mod config;
mod events;
mod goal_evaluator;
mod loop_runner;

#[cfg(test)]
mod loop_runner_tests;
#[cfg(test)]
mod tests;

pub use config::{PlanLoopConfig, TaskSummary};
pub use events::PlanEvent;
pub use goal_evaluator::{GoalEvaluation, GoalEvaluator};
pub use loop_runner::{PlanLoop, PlanLoopWaker};
