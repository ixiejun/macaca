//! Web Shell command adapter boundary for Route C Phase 12.
//!
//! The Web layer owns HTTP parsing, response mapping, and presentation logging.
//! It must not own task/session/trace/service/package semantics. This adapter
//! converts validated HTTP scope into SDK system-facade commands, then maps the
//! facade result back to the existing JSON shape expected by the frontend.

use std::sync::Arc;

use serde_json::json;
use tracing::info;

use macaca_proto::{ApplicationId, MacacaError, MacacaResult};
use macaca_sdk::{
    StaticSystemStatusDataSource, SystemFacade, SystemStatusSnapshot, TaskBoardQueryCommand,
    TodoStoreTaskBoardDataSource,
};
use macaca_task::TodoStore;

/// Thin Web Shell facade specialized for current Web runtime dependencies.
pub struct WebShellFacade {
    system: SystemFacade<TodoStoreTaskBoardDataSource, StaticSystemStatusDataSource>,
}

impl WebShellFacade {
    /// Build a shell facade from current Web state dependencies.
    ///
    /// The status adapter is intentionally inert for the task-board route. It
    /// keeps the facade type complete without making Web own status semantics.
    pub fn for_task_board(todo_store: Arc<TodoStore>) -> Self {
        Self {
            system: SystemFacade::new(
                TodoStoreTaskBoardDataSource::new(todo_store),
                StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                    version: env!("CARGO_PKG_VERSION").into(),
                    agent_count: 0,
                    loaded_apps: 0,
                    max_agents: 0,
                    llm_provider: "shell-unavailable".into(),
                    app_runtime: "macaca-app/AppRuntime".into(),
                    gateway_enabled: false,
                }),
            ),
        }
    }

    /// Query the task board through the SDK facade and preserve legacy JSON shape.
    pub async fn list_todos_json(
        &self,
        app_id: ApplicationId,
        session_id: &str,
    ) -> MacacaResult<serde_json::Value> {
        info!(
            app_id = %app_id.0,
            session_id,
            "web shell task board command received"
        );
        let command = TaskBoardQueryCommand::new(app_id, session_id)
            .map_err(|error| MacacaError::Config(format!("invalid task board command: {error}")))?;
        let result = self.system.query_task_board(command).await?;
        info!(
            count = result.count,
            "web shell task board command completed"
        );
        Ok(json!({ "todos": result.todos, "count": result.count }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;

    #[tokio::test]
    async fn web_shell_task_board_preserves_legacy_json_shape() {
        let tempdir = tempfile::tempdir().unwrap();
        let store = Arc::new(RedbStore::open(tempdir.path().join("shell.redb")).unwrap());
        let todo_store = Arc::new(TodoStore::new(store));
        let shell = WebShellFacade::for_task_board(todo_store);
        let response = shell
            .list_todos_json(ApplicationId(uuid::Uuid::new_v4()), "session-a")
            .await
            .unwrap();
        assert!(response.get("todos").unwrap().is_array());
        assert_eq!(response.get("count").unwrap().as_u64(), Some(0));
    }
}
