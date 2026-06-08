//! Plan-loop configuration and task summary value objects.
//!
//! `PlanLoopConfig` tunes heartbeat cadence and review batching.
//! `TaskSummary` is a denormalized read model passed to goal evaluation consumers.

use std::time::Duration;

/// Configuration for the Plan Agent loop.
pub struct PlanLoopConfig {
    /// How often the plan agent checks for work (default: 30s).
    pub check_interval: Duration,
    /// Maximum number of reviews per cycle (default: 10).
    pub max_reviews_per_cycle: usize,
}

impl Default for PlanLoopConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            max_reviews_per_cycle: 10,
        }
    }
}

/// Summary of a completed task for goal evaluation.
#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub title: String,
    pub agent: String,
    pub status: String,
    pub completion_summary: Option<String>,
}
