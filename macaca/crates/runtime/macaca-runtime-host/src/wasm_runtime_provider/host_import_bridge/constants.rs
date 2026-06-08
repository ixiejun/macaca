//! Provider-neutral constants for WASM host import routing.
//!
//! These identifiers describe OS service contracts, not application names or
//! workflow labels. Guests discover behavior through Application ABI imports;
//! the bridge maps those imports onto stable kernel service ids at runtime.

/// Task Service kernel id used for orchestration lifecycle commands.
pub(super) const TASK_SERVICE_ID: &str = "service.task";

/// Task Service operation that creates a durable goal record.
pub(super) const TASK_CREATE_GOAL_OPERATION: &str = "task.create_goal";

/// Task Service operation that creates an assignment for delegated work.
pub(super) const TASK_CREATE_ASSIGNMENT_OPERATION: &str = "task.create_assignment";

/// Task Service operation that queries board state for an app/session scope.
pub(super) const TASK_QUERY_OPERATION: &str = "task.query";

/// Task Service operation that claims an assignment for an agent executor.
pub(super) const TASK_CLAIM_OPERATION: &str = "task.claim";

/// Task Service operation that marks an assignment as actively running.
pub(super) const TASK_START_OPERATION: &str = "task.start";

/// Task Service operation that submits work for review.
pub(super) const TASK_SUBMIT_REVIEW_OPERATION: &str = "task.submit_review";

/// Task Service operation that records a review verdict.
pub(super) const TASK_REVIEW_OPERATION: &str = "task.review";

/// Audit graph-owner label for application-execution scoped task records.
///
/// This is a provider-neutral trace tag so replay consumers can correlate task
/// lifecycle events with application execution evidence. It is not an
/// application name and must not be used for routing or policy branching.
pub const APPLICATION_EXECUTION_GRAPH_OWNER: &str = "application_execution";
