//! Plan-loop configuration.
//!
//! `PlanLoopConfig` tunes heartbeat cadence and review batching.
//! `TaskSummary` lives in `macaca-proto` because it crosses the task-service
//! event boundary.

use std::time::Duration;

pub use macaca_proto::TaskSummary;

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
