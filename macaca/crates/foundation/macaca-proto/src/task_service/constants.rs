//! Task Service command names.

/// Stable service id for the generic Task Service.
pub const TASK_SERVICE_ID: &str = "service.task";

/// Command name for querying a session-scoped task board.
pub const TASK_QUERY_COMMAND: &str = "task.query";

/// Command name for querying aggregate task progress.
pub const TASK_PROGRESS_COMMAND: &str = "task.progress";

/// Command name for querying one agent's task board.
pub const TASK_AGENT_TODOS_COMMAND: &str = "task.agent_todos";

/// Command name for querying goals.
pub const TASK_GOALS_COMMAND: &str = "task.goals";

/// Command name for querying session claim diagnostics.
pub const TASK_CLAIM_DIAGNOSTICS_COMMAND: &str = "task.claim_diagnostics";

/// Command name for building a goal decomposition prompt.
pub const TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND: &str = "task.build_decomposition_prompt";

/// Command name for building a goal evaluation prompt.
pub const TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND: &str = "task.build_goal_evaluation_prompt";

/// Command name for parsing a goal evaluation response.
pub const TASK_PARSE_GOAL_EVALUATION_COMMAND: &str = "task.parse_goal_evaluation";

/// Command name for creating a high-level task goal.
pub const TASK_CREATE_GOAL_COMMAND: &str = "task.create_goal";

/// Command name for marking a high-level task goal as completed.
pub const TASK_COMPLETE_GOAL_COMMAND: &str = "task.complete_goal";

/// Command name for creating one explicit task assignment.
pub const TASK_CREATE_ASSIGNMENT_COMMAND: &str = "task.create_assignment";

/// Command name for claiming a task.
pub const TASK_CLAIM_COMMAND: &str = "task.claim";

/// Command name for marking a task as started.
pub const TASK_START_COMMAND: &str = "task.start";

/// Command name for submitting a task for review.
pub const TASK_SUBMIT_REVIEW_COMMAND: &str = "task.submit_review";

/// Command name for marking a task as failed.
pub const TASK_FAIL_COMMAND: &str = "task.fail";

/// Command name for applying a task review result.
pub const TASK_REVIEW_COMMAND: &str = "task.review";

/// Command name for resuming a coordinator after a task milestone.
pub const TASK_RESUME_COORDINATOR_COMMAND: &str = "task.resume_coordinator";

/// Command name for deterministic task service snapshots.
pub const TASK_SNAPSHOT_COMMAND: &str = "service.snapshot";
