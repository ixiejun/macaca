//! Re-exported Task Service command contracts.
//!
//! `macaca-proto` owns these provider-neutral DTOs. The task service crate
//! re-exports them so provider-internal modules and downstream callers keep a
//! stable path while the protocol crate remains the canonical source of truth.

pub use macaca_proto::{
    BuildDecompositionPromptCommand, BuildGoalEvaluationPromptCommand, ClaimTaskCommand,
    CompleteGoalCommand, CreateGoalCommand, CreateTaskAssignmentCommand, FailTaskCommand,
    ParseGoalEvaluationCommand, QueryAgentTodosCommand, QueryTaskBoardCommand,
    QueryTaskClaimDiagnosticsCommand, QueryTaskGoalsCommand, QueryTaskProgressCommand,
    ResumeCoordinatorCommand, ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand,
    TaskServiceSnapshotCommand,
};
