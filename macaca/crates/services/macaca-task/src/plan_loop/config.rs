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
    /// Minimum wait before re-emitting `ReviewNeeded` for the same task.
    ///
    /// This guards the generic scheduler from a review storm when the delegated
    /// planner returns without persisting `review_todo`, while still allowing
    /// unattended recovery from transient tool-call failures. The default is
    /// intentionally longer than the normal planner review path so a slow but
    /// healthy review delegate is not mistaken for a missing persistence result.
    pub review_retry_backoff: Duration,
    /// Maximum number of `ReviewNeeded` dispatch attempts per PendingReview task.
    ///
    /// The first emission counts as attempt one. The state is cleared as soon as
    /// the task leaves `PendingReview`, so a later optimization cycle receives a
    /// fresh retry budget.
    pub max_review_dispatch_attempts: usize,
}

impl Default for PlanLoopConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            max_reviews_per_cycle: 10,
            review_retry_backoff: Duration::from_secs(120),
            max_review_dispatch_attempts: 3,
        }
    }
}
