//! Shell-facing system facade for Web and CLI thin-shell migration.
//!
//! This module defines typed commands and replaceable data-source traits for
//! presentation shells. Web routes and CLI handlers should translate transport
//! input into these commands, then delegate to a facade implementation. The SDK
//! owns command shape and orchestration boundaries, while concrete crates own
//! storage adapters, terminal formatting, HTTP status mapping, and SSE transport.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use macaca_kernel::Kernel;
use macaca_proto::{ApplicationId, MacacaResult, TodoItem};
use macaca_task::TodoStore;

/// Command used by shells to request one session-scoped task board view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardQueryCommand {
    pub app_id: ApplicationId,
    pub session_id: String,
}

impl TaskBoardQueryCommand {
    /// Build a session-scoped task board query and reject blank session ids.
    pub fn new(app_id: ApplicationId, session_id: impl Into<String>) -> MacacaResult<Self> {
        let session_id = session_id.into().trim().to_string();
        if session_id.is_empty() {
            return Err(macaca_proto::MacacaError::Config(
                "task board query requires non-empty session_id".into(),
            ));
        }
        Ok(Self { app_id, session_id })
    }
}

/// Replay command for shell-facing session event reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventQueryCommand {
    pub session_id: String,
    pub since: Option<u64>,
    pub limit: Option<usize>,
}

/// Cursor command for shell-facing trace tail operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceTailCommand {
    pub session_id: String,
    pub since: Option<u64>,
}

/// Read-only service inspection command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInspectionCommand {
    pub scope: String,
}

/// Read-only package inspection command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInspectionCommand {
    pub package_ref: Option<String>,
}

/// Policy-ready approval command produced by Web/CLI confirmation surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionCommand {
    pub decision_id: String,
    pub approved: bool,
    pub decided_at: DateTime<Utc>,
}

/// Replay-safe task board result preserving the current Web response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoardQueryResult {
    pub todos: Vec<TodoItem>,
    pub count: usize,
}

/// Small system status snapshot used by CLI status-like commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStatusSnapshot {
    pub version: String,
    pub agent_count: usize,
    pub loaded_apps: usize,
    pub max_agents: usize,
    pub llm_provider: String,
    pub app_runtime: String,
    pub gateway_enabled: bool,
}

/// Replaceable data source for task-board reads.
#[async_trait]
pub trait TaskBoardDataSource: Send + Sync {
    /// Return all task items for the requested application/session scope.
    async fn list_session_todos(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<Vec<TodoItem>>;
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
impl TaskBoardDataSource for TodoStoreTaskBoardDataSource {
    async fn list_session_todos(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<Vec<TodoItem>> {
        Ok(self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await)
    }
}

/// Replaceable data source for CLI/system status inspection.
#[async_trait]
pub trait SystemStatusDataSource: Send + Sync {
    /// Return one shell-facing status snapshot.
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot>;
}

/// Adapter that reads status from kernel/app runtime counts without Web state.
pub struct StaticSystemStatusDataSource {
    snapshot: SystemStatusSnapshot,
}

impl StaticSystemStatusDataSource {
    /// Create a status source from an already prepared snapshot.
    pub fn new(snapshot: SystemStatusSnapshot) -> Self {
        Self { snapshot }
    }
}

#[async_trait]
impl SystemStatusDataSource for StaticSystemStatusDataSource {
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        Ok(self.snapshot.clone())
    }
}

/// Build a CLI-oriented status snapshot from lower-layer runtime objects.
pub async fn kernel_status_snapshot(
    kernel: &Kernel,
    loaded_apps: usize,
    max_agents: usize,
    llm_provider: impl Into<String>,
    gateway_enabled: bool,
) -> SystemStatusSnapshot {
    SystemStatusSnapshot {
        version: env!("CARGO_PKG_VERSION").into(),
        agent_count: kernel.agent_count().await,
        loaded_apps,
        max_agents,
        llm_provider: llm_provider.into(),
        app_runtime: "macaca-app/AppRuntime".into(),
        gateway_enabled,
    }
}

/// SDK system facade consumed by Web/CLI thin shells.
pub struct SystemFacade<T, S> {
    task_board: T,
    status: S,
}

impl<T, S> SystemFacade<T, S>
where
    T: TaskBoardDataSource,
    S: SystemStatusDataSource,
{
    /// Create a facade from explicit replaceable data-source adapters.
    pub fn new(task_board: T, status: S) -> Self {
        Self { task_board, status }
    }

    /// Query a session-scoped task board through the facade boundary.
    pub async fn query_task_board(
        &self,
        command: TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            "system facade task board query started"
        );
        let mut todos = self.task_board.list_session_todos(&command).await?;
        todos.sort_by_key(|todo| todo.sequence_number);
        let count = todos.len();
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count,
            "system facade task board query completed"
        );
        Ok(TaskBoardQueryResult { todos, count })
    }

    /// Return a shell-facing status snapshot without exposing presentation internals.
    pub async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("system facade status snapshot requested");
        let snapshot = self.status.status_snapshot().await?;
        if snapshot.max_agents == 0 {
            warn!("system facade status snapshot reported zero max_agents");
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::TodoStatus;

    struct MockTaskBoardDataSource {
        todos: Vec<TodoItem>,
    }

    #[async_trait]
    impl TaskBoardDataSource for MockTaskBoardDataSource {
        async fn list_session_todos(
            &self,
            _command: &TaskBoardQueryCommand,
        ) -> MacacaResult<Vec<TodoItem>> {
            Ok(self.todos.clone())
        }
    }

    fn todo(sequence_number: u32) -> TodoItem {
        let mut item = TodoItem::new(
            ApplicationId(uuid::Uuid::new_v4()),
            Some("session-a".into()),
            "agent",
            "planner",
            "title",
            "description",
            1,
        );
        item.session_id = Some("session-a".into());
        item.status = TodoStatus::Pending;
        item.sequence_number = sequence_number;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();
        item
    }

    #[tokio::test]
    async fn system_facade_returns_sorted_task_board_without_web_dependency() {
        let facade = SystemFacade::new(
            MockTaskBoardDataSource {
                todos: vec![todo(2), todo(1)],
            },
            StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                version: "test".into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 8,
                llm_provider: "stub".into(),
                app_runtime: "macaca-app/AppRuntime".into(),
                gateway_enabled: false,
            }),
        );
        let result = facade
            .query_task_board(
                TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "session-a")
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.count, 2);
        assert_eq!(result.todos[0].sequence_number, 1);
    }

    #[test]
    fn task_board_command_rejects_blank_session_scope() {
        let error = TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "  ");
        assert!(error.is_err());
    }
}
