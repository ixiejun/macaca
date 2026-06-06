//! aos-task: task decomposition, scheduling, and lifecycle.

pub mod a2a_task;
pub mod claim_diagnostics;
pub mod commands;
pub mod decompose;
pub mod dependency;
pub mod events;
pub mod lifecycle;
pub mod plan_loop;
pub mod queue;
pub mod runtime;
pub mod scheduler;
pub mod service_adapter;
pub mod todo_board;
pub mod todo_store;
pub mod tracker;
pub mod worker_loop;

pub use a2a_task::{A2APaymentReferences, A2ATaskRequestContext, A2ATaskTraceContext};
pub use claim_diagnostics::{diagnose_session_claims, SessionClaimDiagnostics};
pub use commands::{
    ClaimTaskCommand, CreateGoalCommand, CreateTaskAssignmentCommand, QueryTaskBoardCommand,
    ResumeCoordinatorCommand, ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand,
    TaskServiceSnapshotCommand,
};
pub use decompose::{
    build_decomposition_prompt, DecomposedTask, LlmDecomposer, SimpleDecomposer, TaskDecomposer,
};
pub use dependency::{DefaultTaskDependencyResolver, TaskDependencyResolver};
pub use events::{
    TaskServiceEvent, TaskServiceEventType, TaskServiceGoalSnapshot, TaskServiceSnapshot,
    TaskServiceTaskSnapshot,
};
pub use lifecycle::{DefaultTodoLifecyclePolicy, TodoLifecyclePolicy};
pub use plan_loop::{
    GoalEvaluation, GoalEvaluator, PlanEvent, PlanLoop, PlanLoopConfig, PlanLoopWaker, TaskSummary,
};
pub use queue::TaskQueue;
pub use runtime::{
    InMemoryTaskServiceEventSink, NoopTaskServiceExecutionStrategy, TaskServiceEventSink,
    TaskServiceExecutionStrategy, TaskServiceRuntime,
};
pub use scheduler::{ScheduleAction, ScheduleEntry, ScheduleEvent, SchedulerConfig, TaskScheduler};
pub use service_adapter::task_service_descriptor;
pub use todo_board::{ProgressSummary, TaskBoard, TaskSpace};
pub use todo_store::TodoStore;
pub use tracker::TaskTracker;
pub use worker_loop::{WorkerEvent, WorkerLoop, WorkerLoopConfig, WorkerLoopWaker};
