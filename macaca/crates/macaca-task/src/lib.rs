//! aos-task: task decomposition, scheduling, and lifecycle.

pub mod claim_diagnostics;
pub mod decompose;
pub mod plan_loop;
pub mod queue;
pub mod scheduler;
pub mod todo_board;
pub mod todo_store;
pub mod tracker;
pub mod worker_loop;

pub use claim_diagnostics::{diagnose_session_claims, SessionClaimDiagnostics};
pub use decompose::{DecomposedTask, LlmDecomposer, SimpleDecomposer, TaskDecomposer};
pub use plan_loop::{
    GoalEvaluation, GoalEvaluator, PlanEvent, PlanLoop, PlanLoopConfig, PlanLoopWaker, TaskSummary,
};
pub use queue::TaskQueue;
pub use scheduler::{ScheduleAction, ScheduleEntry, ScheduleEvent, SchedulerConfig, TaskScheduler};
pub use todo_board::{ProgressSummary, TaskBoard, TaskSpace};
pub use todo_store::TodoStore;
pub use tracker::TaskTracker;
pub use worker_loop::{WorkerEvent, WorkerLoop, WorkerLoopConfig, WorkerLoopWaker};
