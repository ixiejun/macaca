//! SDK task client boundary for shell-facing task board operations.
//!
//! Route C keeps task semantics out of Web and CLI. This module models task
//! board reads and task-service calls as typed commands. The current
//! compatibility backend still reads the `TodoStore` directly for session
//! board queries, while the richer task service client stays replaceable for
//! later S4 runtime wiring.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::{ApplicationId, MacacaError, MacacaResult, TodoGoal, TodoItem};
use macaca_task::{
    ClaimTaskCommand, CreateGoalCommand, QueryTaskBoardCommand, ResumeCoordinatorCommand,
    ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand, TaskServiceSnapshot,
    TaskServiceSnapshotCommand, TodoStore,
};

/// Command used by shells to request one session-scoped task board view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardQueryCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
}

impl TaskBoardQueryCommand {
    /// Build a session-scoped task board query and reject blank session ids.
    ///
    /// The constructor acts as the Specification boundary for this command: a
    /// task board view is meaningless without a session scope, so invalid input
    /// is rejected before any store or service client is invoked.
    pub fn new(app_id: ApplicationId, session_id: impl Into<String>) -> MacacaResult<Self> {
        let session_id = session_id.into().trim().to_string();
        if session_id.is_empty() {
            return Err(MacacaError::Config(
                "task board query requires non-empty session_id".into(),
            ));
        }
        Ok(Self { app_id, session_id })
    }
}

/// Replay-safe task board result preserving the current Web response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoardQueryResult {
    pub todos: Vec<TodoItem>,
    pub count: usize,
}

/// Replaceable client for task-board reads.
///
/// S3 uses this trait as a Strategy boundary. The current implementation reads
/// from `TodoStore`, while S4 can replace it with a Task Service client without
/// changing Web/CLI command shapes.
#[async_trait]
pub trait SystemTaskClient: Send + Sync {
    /// Return all task items for the requested application/session scope.
    async fn query_task_board(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult>;
}

/// Adapter that lets current `TodoStore` back the SDK facade without Web owning semantics.
pub struct TodoStoreTaskBoardDataSource {
    store: Arc<TodoStore>,
}

impl TodoStoreTaskBoardDataSource {
    /// Create a data source over the existing task store.
    pub fn new(store: Arc<TodoStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl SystemTaskClient for TodoStoreTaskBoardDataSource {
    async fn query_task_board(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            "sdk task client querying local todo store"
        );
        let mut todos = self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await;
        todos.sort_by_key(|todo| todo.sequence_number);
        let count = todos.len();
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count,
            "sdk task client completed local todo store query"
        );
        Ok(TaskBoardQueryResult { todos, count })
    }
}

/// Backward-compatible alias for the pre-S3 task-board data-source name.
pub trait TaskBoardDataSource: SystemTaskClient {}

impl<T> TaskBoardDataSource for T where T: SystemTaskClient {}

/// Replaceable client for the typed Task Service boundary.
///
/// The client remains intentionally small and capability-scoped.  Web and CLI
/// can depend on this trait when they need typed goal, claim, review, resume,
/// or snapshot interactions without taking a dependency on the eventual task
/// service host implementation.
#[async_trait]
pub trait TaskServiceClient: Send + Sync {
    /// Create a new goal in the task service boundary.
    async fn create_goal(&self, command: &CreateGoalCommand) -> MacacaResult<TodoGoal>;

    /// Query the existing session-scoped task board.
    async fn query_task_board(
        &self,
        command: &QueryTaskBoardCommand,
    ) -> MacacaResult<TaskBoardQueryResult>;

    /// Claim one task from the task service boundary.
    async fn claim_task(&self, command: &ClaimTaskCommand) -> MacacaResult<Option<TodoItem>>;

    /// Mark a task as started by an agent.
    async fn start_task(&self, command: &StartTaskCommand) -> MacacaResult<bool>;

    /// Submit a task for review.
    async fn submit_review(&self, command: &SubmitReviewCommand) -> MacacaResult<bool>;

    /// Apply a review result to a task.
    async fn review_task(&self, command: &ReviewTaskCommand) -> MacacaResult<bool>;

    /// Request coordinator resume after a task lifecycle milestone.
    async fn resume_coordinator(&self, command: &ResumeCoordinatorCommand) -> MacacaResult<()>;

    /// Inspect the deterministic task service snapshot.
    async fn snapshot(
        &self,
        command: &TaskServiceSnapshotCommand,
    ) -> MacacaResult<TaskServiceSnapshot>;
}

/// Compatibility placeholder for shells that are not yet wired to a runtime-backed Task Service.
#[derive(Debug, Default, Clone)]
pub struct UnavailableTaskServiceClient;

#[async_trait]
impl TaskServiceClient for UnavailableTaskServiceClient {
    async fn create_goal(&self, _command: &CreateGoalCommand) -> MacacaResult<TodoGoal> {
        Err(MacacaError::Config(
            "task service create_goal is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn query_task_board(
        &self,
        command: &QueryTaskBoardCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        Err(MacacaError::Config(format!(
            "task service board query for session '{}' is unavailable through the local SDK compatibility client",
            command.session_id
        )))
    }

    async fn claim_task(&self, _command: &ClaimTaskCommand) -> MacacaResult<Option<TodoItem>> {
        Err(MacacaError::Config(
            "task service claim_task is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn start_task(&self, _command: &StartTaskCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service start_task is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn submit_review(&self, _command: &SubmitReviewCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service submit_review is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn review_task(&self, _command: &ReviewTaskCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service review_task is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn resume_coordinator(&self, _command: &ResumeCoordinatorCommand) -> MacacaResult<()> {
        Err(MacacaError::Config(
            "task service resume_coordinator is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }

    async fn snapshot(
        &self,
        _command: &TaskServiceSnapshotCommand,
    ) -> MacacaResult<TaskServiceSnapshot> {
        Err(MacacaError::Config(
            "task service snapshot is unavailable through the local SDK compatibility client"
                .into(),
        ))
    }
}
