//! Neutral harness identifiers for pipeline dry-run stages.
//!
//! These strings are **test-harness fixtures** only — they exercise OS task/board APIs
//! and are not application business logic baked into OS crates.

/// Assignee agent id used across session-claim and depends-on stages.
pub const ASSIGNEE_AGENT: &str = "harness-assignee";
/// Coordinator agent id used when creating task assignments.
pub const COORDINATOR_AGENT: &str = "harness-coordinator";
/// Agent id for the create_todo agentic stage.
pub const PLANNER_AGENT: &str = "harness-planner";
