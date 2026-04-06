//! Tool abstractions and built-in tool implementations for Agent OS.

pub mod builtin;
pub mod tool;
pub mod orchestration;
pub mod todo;

pub use tool::{Tool, ToolSet, TraceEvent};
pub use builtin::{DefaultToolSet, FileReadTool, FileWriteTool, ShellTool};
pub use orchestration::{
    OrchestrationState, DelegateTaskTool, GetTaskResultTool, GetTaskResultCallback,
    TaskResultData, ReportResultTool, ListAgentsTool,
};
pub use todo::{
    ClaimTaskTool, StartTaskTool, UpdateTaskProgressTool,
    SubmitTaskForReviewTool, ListMyTasksTool,
    CreateTodoTool, ReviewTodoTool, CheckTodoProgressTool, CreateGoalTool, ReassignTaskTool,
    OnGoalCreated,
};
