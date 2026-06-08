//! Contract tests for the shell-facing SDK system facade.
//!
//! Tests are flattened at the module root (no nested `mod tests`) so escape-hatch
//! gates can scan production modules without false positives from test fixtures.

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{ApplicationId, MacacaResult, TodoItem, TodoStatus};

use super::types::SystemFacade;
use crate::service_client::ServiceCallCommand;
use crate::status_client::{StaticSystemStatusDataSource, SystemStatusSnapshot};
use crate::task_client::{SystemTaskClient, TaskBoardQueryCommand, TaskBoardQueryResult};

/// Provider-neutral fixture ids (Object Mother pattern) — avoids forbidden
/// application role literals in standalone `tests.rs` raw escape-hatch scans.
const FIXTURE_AGENT_ID: &str = "fixture-alpha";
const FIXTURE_ROLE_ID: &str = "fixture-role";

struct MockTaskBoardClient {
    todos: Vec<TodoItem>,
}

#[async_trait]
impl SystemTaskClient for MockTaskBoardClient {
    async fn query_task_board(
        &self,
        _command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        let mut todos = self.todos.clone();
        todos.sort_by_key(|todo| todo.sequence_number);
        let count = todos.len();
        Ok(TaskBoardQueryResult { todos, count })
    }
}

fn todo(sequence_number: u32) -> TodoItem {
    let mut item = TodoItem::new(
        ApplicationId(uuid::Uuid::new_v4()),
        Some("session-a".into()),
        FIXTURE_AGENT_ID,
        FIXTURE_ROLE_ID,
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
        MockTaskBoardClient {
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

#[tokio::test]
async fn default_service_client_returns_structured_unavailable() {
    let facade = SystemFacade::new(
        MockTaskBoardClient { todos: Vec::new() },
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
    let command =
        ServiceCallCommand::new("service-a", "command-a", serde_json::json!({})).unwrap();
    let error = facade.call_service(command).await.unwrap_err();
    assert!(error.to_string().contains("unavailable"));
}

#[test]
fn task_board_command_rejects_blank_session_scope() {
    let error = TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "  ");
    assert!(error.is_err());
}
