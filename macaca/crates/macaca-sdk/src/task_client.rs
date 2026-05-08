//! SDK task client boundary for shell-facing task board operations.
//!
//! Route C keeps task semantics out of Web and CLI. This module models task
//! board reads as typed commands and adapts the current `TodoStore` as a local
//! compatibility backend until S4 promotes Task/Planner/Review into services.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::{ApplicationId, MacacaError, MacacaResult, TodoItem};
use macaca_task::TodoStore;

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
