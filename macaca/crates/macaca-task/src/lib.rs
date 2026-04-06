//! aos-task: task decomposition, scheduling, and lifecycle.

pub mod decompose;
pub mod plan_loop;
pub mod queue;
pub mod scheduler;
pub mod todo_board;
pub mod todo_store;
pub mod tracker;
pub mod worker_loop;

pub use decompose::{SimpleDecomposer, TaskDecomposer, LlmDecomposer, DecomposedTask};
pub use plan_loop::{PlanLoop, PlanLoopConfig, PlanLoopWaker, PlanEvent, TaskSummary, GoalEvaluator, GoalEvaluation};
pub use queue::TaskQueue;
pub use scheduler::{ScheduleAction, ScheduleEntry, ScheduleEvent, SchedulerConfig, TaskScheduler};
pub use todo_board::{TaskBoard, TaskSpace, ProgressSummary};
pub use todo_store::TodoStore;
pub use tracker::TaskTracker;
pub use worker_loop::{WorkerLoop, WorkerLoopConfig, WorkerLoopWaker, WorkerEvent};
