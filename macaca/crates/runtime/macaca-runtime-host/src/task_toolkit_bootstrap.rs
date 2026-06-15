//! Runtime-host factory for task and todo toolkit tools.
//!
//! Framework shells need per-agent task tools, but they should not construct
//! `TaskSpace`, `TaskBoard`, or concrete todo tools directly. This module owns
//! the Abstract Factory for those service-family tools and exposes only generic
//! [`macaca_tools::Tool`] objects back to shells.

use std::collections::HashMap;
use std::sync::Arc;

use macaca_proto::{ApplicationId, TaskId, TodoGoal};
use macaca_task::{TaskBoard, TaskSpace, TodoStore};
use macaca_tools::{
    CheckTodoProgressTool, ClaimTaskTool, CreateGoalTool, CreateTodoTool, CreateTodosTool,
    ListMyTasksTool, ReassignTaskTool, ReviewTodoTool, StartTaskTool, SubmitTaskForReviewTool,
    Tool, UpdateTaskProgressTool,
};

/// Task-tool policy resolved by the caller from application capability policy.
///
/// Runtime-host intentionally owns only the generic tool-family shape. It does
/// not inspect application manifests or role names when deciding which policy
/// applies to a concrete agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskToolkitPolicy {
    /// Goal manager agents can create high-level goals and inspect progress.
    GoalManager,
    /// Planner agents can create, review, reassign, and track todos.
    Planner,
    /// Worker agents can claim, start, update, submit, and list assigned tasks.
    Worker,
}

/// Optional callback invoked after a goal row is persisted.
///
/// The callback is a narrow Observer port. Shells may attach trace, memento, or
/// execution-control side effects without owning concrete tool construction.
pub type GoalRecordedObserver = Arc<dyn Fn(TodoGoal) + Send + Sync>;

/// Provider-neutral request for task toolkit tool construction.
pub struct TaskToolkitBootstrapRequest {
    /// Application scope for the task board.
    pub application_id: ApplicationId,
    /// Agent that will receive the generated tools.
    pub agent_name: String,
    /// Optional session scope for session-local todo queries.
    pub session_id: Option<String>,
    /// Optional goal id attached to planner-created todos.
    pub active_goal_id: Option<TaskId>,
    /// Concrete policy selected by the shell/application policy layer.
    pub policy: TaskToolkitPolicy,
    /// Shared todo store owned by the Task Service/runtime host composition.
    pub todo_store: Arc<TodoStore>,
    /// Agents that should not receive executable task assignments.
    pub disallowed_task_assignees: Vec<String>,
    /// Capability summaries used by assignment validation Strategy code.
    pub assignee_capabilities: HashMap<String, Vec<String>>,
    /// Optional observer for goal-created trace and wait-registration effects.
    pub on_goal_recorded: Option<GoalRecordedObserver>,
}

/// Build the task/todo tool family for one agent.
///
/// The returned tools are generic trait objects so shells can adapt them to the
/// framework toolkit without seeing concrete `TaskSpace` or `TaskBoard`
/// construction. All key construction decisions are logged for auditability.
pub fn bootstrap_task_toolkit_tools(
    request: TaskToolkitBootstrapRequest,
    trace_label: impl Into<String>,
) -> Vec<Box<dyn Tool>> {
    let trace_label = trace_label.into();
    tracing::info!(
        trace_label = %trace_label,
        app_id = %request.application_id,
        agent = %request.agent_name,
        policy = ?request.policy,
        "runtime-host task toolkit bootstrap requested"
    );

    let tools = match request.policy {
        TaskToolkitPolicy::GoalManager => goal_manager_tools(request),
        TaskToolkitPolicy::Planner => planner_tools(request),
        TaskToolkitPolicy::Worker => worker_tools(request),
    };

    tracing::info!(
        trace_label = %trace_label,
        count = tools.len(),
        "runtime-host task toolkit bootstrap completed"
    );
    tools
}

fn task_space(request: &TaskToolkitBootstrapRequest) -> Arc<TaskSpace> {
    Arc::new(TaskSpace::for_session(
        request.application_id,
        request.session_id.clone(),
        Arc::clone(&request.todo_store),
    ))
}

fn goal_manager_tools(request: TaskToolkitBootstrapRequest) -> Vec<Box<dyn Tool>> {
    let space = task_space(&request);
    vec![
        Box::new(CreateGoalTool {
            space: Arc::clone(&space),
            on_created: None,
            on_goal_recorded: request.on_goal_recorded,
        }),
        Box::new(CheckTodoProgressTool { space }),
    ]
}

fn planner_tools(request: TaskToolkitBootstrapRequest) -> Vec<Box<dyn Tool>> {
    let space = task_space(&request);
    let create_todo = CreateTodoTool {
        space: Arc::clone(&space),
        coordinator_name: request.agent_name.clone(),
        disallowed_assignees: request.disallowed_task_assignees.clone(),
        assignee_capabilities: request.assignee_capabilities.clone(),
        active_goal_id: request.active_goal_id,
    };

    vec![
        Box::new(create_todo.clone_for_batch()),
        Box::new(CreateTodosTool { create_todo }),
        Box::new(ReviewTodoTool {
            space: Arc::clone(&space),
            on_reviewed: None,
        }),
        Box::new(CheckTodoProgressTool {
            space: Arc::clone(&space),
        }),
        Box::new(ReassignTaskTool {
            space: Arc::clone(&space),
        }),
        Box::new(CreateGoalTool {
            space,
            on_created: None,
            on_goal_recorded: request.on_goal_recorded,
        }),
    ]
}

fn worker_tools(request: TaskToolkitBootstrapRequest) -> Vec<Box<dyn Tool>> {
    let board = Arc::new(TaskBoard::for_agent(
        request.application_id,
        request.agent_name,
        request.session_id,
        request.todo_store,
    ));

    vec![
        Box::new(ClaimTaskTool {
            board: Arc::clone(&board),
        }),
        Box::new(StartTaskTool {
            board: Arc::clone(&board),
        }),
        Box::new(UpdateTaskProgressTool {
            board: Arc::clone(&board),
        }),
        Box::new(SubmitTaskForReviewTool {
            board: Arc::clone(&board),
        }),
        Box::new(ListMyTasksTool { board }),
    ]
}

trait CloneForBatch {
    fn clone_for_batch(&self) -> CreateTodoTool;
}

impl CloneForBatch for CreateTodoTool {
    fn clone_for_batch(&self) -> CreateTodoTool {
        CreateTodoTool {
            space: Arc::clone(&self.space),
            coordinator_name: self.coordinator_name.clone(),
            disallowed_assignees: self.disallowed_assignees.clone(),
            assignee_capabilities: self.assignee_capabilities.clone(),
            active_goal_id: self.active_goal_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_bootstrap_returns_planner_tool_family() {
        let dir = tempfile::tempdir().expect("tempdir should be available for task toolkit test");
        let path = dir.path().join("task-toolkit.redb");
        let _leaked_dir = Box::leak(Box::new(dir));
        let store = Arc::new(TodoStore::new(Arc::new(
            macaca_persist::RedbStore::open(path)
                .expect("redb task toolkit test store should open"),
        )));
        let tools = bootstrap_task_toolkit_tools(
            TaskToolkitBootstrapRequest {
                application_id: ApplicationId::new(),
                agent_name: "agent-a".into(),
                session_id: Some("session-a".into()),
                active_goal_id: None,
                policy: TaskToolkitPolicy::Planner,
                todo_store: store,
                disallowed_task_assignees: Vec::new(),
                assignee_capabilities: HashMap::new(),
                on_goal_recorded: None,
            },
            "planner-task-toolkit-test",
        );

        let names = tools.iter().map(|tool| tool.name()).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "create_todo",
                "create_todos",
                "review_todo",
                "check_todo_progress",
                "reassign_task",
                "create_goal"
            ]
        );
    }
}
