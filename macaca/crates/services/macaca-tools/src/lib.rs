//! Tool abstractions and built-in tool implementations for Agent OS.

pub mod builtin;
pub mod orchestration;
pub mod todo;
pub mod tool;

pub use builtin::{DefaultToolSet, FileReadTool, FileWriteTool, ShellTool};
pub use orchestration::{
    DelegateTaskTool, GetTaskResultCallback, GetTaskResultTool, ListAgentsTool, TaskResultData,
};
pub use todo::{
    CheckTodoProgressTool, ClaimTaskTool, CreateGoalTool, CreateTodoTool, CreateTodosTool,
    ListMyTasksTool, OnGoalCreated, OnGoalRecorded, OnTodoReviewed, ReassignTaskTool,
    ReviewTodoTool, StartTaskTool, SubmitTaskForReviewTool, UpdateTaskProgressTool,
};
pub use tool::{
    CompositeToolSet, Tool, ToolCatalog, ToolCommand, ToolCommandContext, ToolCommandExecutor,
    ToolCommandMiddleware, ToolCommandPipeline, ToolSchemaProvider, TraceEvent,
    TraceToolCommandMiddleware,
};
