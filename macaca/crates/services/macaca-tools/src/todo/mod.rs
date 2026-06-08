//! Todo task board tools for Worker Agents and Plan Agents.
//!
//! Split from monolithic `todo.rs` (P5 iteration 90) using a Facade module tree.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_tools::todo::*` public API.
//! - **Command**: each tool struct implements [`crate::tool::Tool`].
//! - **Composite**: [`CreateTodosTool`] batches via [`CreateTodoTool::create_one`].
//! - **Strategy**: dependency resolution and foundation inference in `create_todo`.
//! - **Observer / Callback**: optional hooks in `callbacks` for PlanLoop and run_trace.
//!
//! Worker tools: claim_task, start_task, update_task_progress, submit_task_for_review, list_my_tasks
//! Plan tools:   create_todo, review_todo, check_todo_progress

mod callbacks;
mod create_todo;
mod create_todos;
mod plan_space;
mod worker_board;

#[cfg(test)]
mod tests;

pub use callbacks::{OnGoalCreated, OnGoalRecorded, OnTodoReviewed};
pub use create_todo::CreateTodoTool;
pub use create_todos::CreateTodosTool;
pub use plan_space::{
    CheckTodoProgressTool, CreateGoalTool, ReassignTaskTool, ReviewTodoTool,
};
pub use worker_board::{
    ClaimTaskTool, ListMyTasksTool, StartTaskTool, SubmitTaskForReviewTool,
    UpdateTaskProgressTool,
};
