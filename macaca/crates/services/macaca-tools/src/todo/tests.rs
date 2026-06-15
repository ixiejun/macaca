//! Contract tests for todo task-board tools.
//!
//! Uses provider-neutral fixture agent ids (Object Mother pattern).

use std::collections::HashMap;
use std::sync::Arc;

use macaca_persist::RedbStore;
use macaca_proto::{MacacaResult, TodoStatus};
use macaca_task::{TaskSpace, TodoStore};
use serde_json::{json, Value};
use tempfile::tempdir;

use crate::tool::Tool;

use super::create_todo::CreateTodoTool;
use super::create_todos::CreateTodosTool;

/// Provider-neutral fixture agent ids (Object Mother pattern).
const FIXTURE_ENTRY_SUPERVISOR: &str = "entry-agent";
const FIXTURE_PLAN_AGENT: &str = "plan-agent";
const FIXTURE_DESIGN_AGENT: &str = "fixture-design";
const FIXTURE_API_AGENT: &str = "fixture-api";
const FIXTURE_UI_AGENT: &str = "fixture-ui";

async fn exec_tool(tool: &dyn Tool, input: Value) -> MacacaResult<Value> {
    crate::tool::ToolCommandExecutor::execute_command(tool, crate::tool::ToolCommand::new(input))
        .await
}

#[tokio::test]
async fn create_todo_rejects_supervisor_agents() {
    let dir = tempdir().expect("tempdir");
    let db = RedbStore::open(dir.path().join("todo-tests.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let space = Arc::new(TaskSpace::for_session(
        macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
        Some("session".into()),
        store,
    ));
    let tool = CreateTodoTool {
        space,
        coordinator_name: FIXTURE_PLAN_AGENT.into(),
        disallowed_assignees: vec![FIXTURE_ENTRY_SUPERVISOR.into(), FIXTURE_PLAN_AGENT.into()],
        assignee_capabilities: HashMap::new(),
        active_goal_id: None,
    };

    let err = exec_tool(
        &tool,
        json!({
            "agent": FIXTURE_ENTRY_SUPERVISOR,
            "title": "Should fail",
            "description": "Supervisor must not get TaskBoard work"
        }),
    )
    .await
    .expect_err("supervisor assignment should be rejected");

    assert!(
        err.to_string()
            .contains("cannot assign tasks to supervisor agent"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn create_todo_requires_agent_field() {
    let dir = tempdir().expect("tempdir");
    let db = RedbStore::open(dir.path().join("todo-tests-missing-agent.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let space = Arc::new(TaskSpace::for_session(
        macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
        Some("session".into()),
        store,
    ));
    let tool = CreateTodoTool {
        space,
        coordinator_name: "entry_custom".into(),
        disallowed_assignees: vec![],
        assignee_capabilities: HashMap::new(),
        active_goal_id: None,
    };

    let err = exec_tool(
        &tool,
        json!({
            "title": "Missing agent",
            "description": "should fail when agent is not provided"
        }),
    )
    .await
    .expect_err("missing agent must be rejected");

    assert!(
        err.to_string().contains("missing required field: agent"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn create_todo_preserves_requested_agent_even_when_profile_differs() {
    let dir = tempdir().expect("tempdir");
    let db = RedbStore::open(dir.path().join("todo-tests-preserve-agent.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let space = Arc::new(TaskSpace::for_session(
        macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
        Some("session".into()),
        store,
    ));
    let mut profiles = HashMap::new();
    profiles.insert(
        FIXTURE_DESIGN_AGENT.to_string(),
        vec![
            "design_analysis".into(),
            "architecture specification interface planning".into(),
        ],
    );
    profiles.insert(
        FIXTURE_API_AGENT.to_string(),
        vec![
            "api_development".into(),
            "rest api server database integration".into(),
        ],
    );
    let tool = CreateTodoTool {
        space: Arc::clone(&space),
        coordinator_name: FIXTURE_PLAN_AGENT.into(),
        disallowed_assignees: vec![],
        assignee_capabilities: profiles,
        active_goal_id: None,
    };

    let out = exec_tool(
        &tool,
        json!({
            "agent": FIXTURE_DESIGN_AGENT,
            "title": "Implement API service layer",
            "description": "Build REST endpoints and database integration"
        }),
    )
    .await
    .expect("create_todo should succeed");

    assert_eq!(
        out["agent"].as_str().unwrap_or_default(),
        FIXTURE_DESIGN_AGENT,
        "create_todo must preserve plan agent's explicit assignee"
    );
    assert_eq!(
        out["requested_agent"].as_str().unwrap_or_default(),
        FIXTURE_DESIGN_AGENT
    );
    assert!(out["routing_reason"].is_null());
    let all = space.list_all().await;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].assigned_agent, FIXTURE_DESIGN_AGENT);
}

#[tokio::test]
async fn create_todo_preserves_requested_agent_for_foundation_tasks() {
    let dir = tempdir().expect("tempdir");
    let db =
        RedbStore::open(dir.path().join("todo-tests-foundation-preserve.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let space = Arc::new(TaskSpace::for_session(
        macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
        Some("session".into()),
        store,
    ));
    let mut profiles = HashMap::new();
    profiles.insert(
        FIXTURE_DESIGN_AGENT.to_string(),
        vec![
            "architecture_design Define system boundaries and technical constraints".into(),
            "interface_contract_design Define API and data contracts".into(),
        ],
    );
    profiles.insert(
        FIXTURE_API_AGENT.to_string(),
        vec!["api_development Implement REST API with PostgreSQL".into()],
    );
    let tool = CreateTodoTool {
        space: Arc::clone(&space),
        coordinator_name: FIXTURE_PLAN_AGENT.into(),
        disallowed_assignees: vec![],
        assignee_capabilities: profiles,
        active_goal_id: None,
    };

    let out = exec_tool(
        &tool,
        json!({
            "agent": FIXTURE_API_AGENT,
            "title": "设计项目架构和API规范",
            "description": "设计整体项目架构、接口规范、数据模型和契约。输出架构设计文档。"
        }),
    )
    .await
    .expect("create_todo should succeed");

    assert_eq!(
        out["agent"].as_str().unwrap_or_default(),
        FIXTURE_API_AGENT,
        "create_todo must not override plan agent's explicit assignee"
    );
    assert_eq!(
        out["requested_agent"].as_str().unwrap_or_default(),
        FIXTURE_API_AGENT
    );
    assert!(out["routing_reason"].is_null());
    let all = space.list_all().await;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].assigned_agent, FIXTURE_API_AGENT);
}

#[tokio::test]
async fn create_todo_auto_adds_foundation_dependencies_for_goal() {
    let dir = tempdir().expect("tempdir");
    let db =
        RedbStore::open(dir.path().join("todo-tests-foundation-deps.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let app_id = macaca_proto::ApplicationId(uuid::Uuid::new_v4());
    let goal_id = macaca_proto::TaskId::new();
    let space = Arc::new(TaskSpace::for_session(
        app_id,
        Some("session".into()),
        Arc::clone(&store),
    ));
    let mut profiles = HashMap::new();
    profiles.insert(
        FIXTURE_DESIGN_AGENT.to_string(),
        vec!["design_analysis architecture specification".into()],
    );
    profiles.insert(
        FIXTURE_UI_AGENT.to_string(),
        vec!["ui_development react nextjs presentation".into()],
    );
    let tool = CreateTodoTool {
        space: Arc::clone(&space),
        coordinator_name: FIXTURE_PLAN_AGENT.into(),
        disallowed_assignees: vec![],
        assignee_capabilities: profiles,
        active_goal_id: Some(goal_id),
    };

    let design_task = exec_tool(
        &tool,
        json!({
            "agent": FIXTURE_DESIGN_AGENT,
            "title": "Define architecture and interfaces",
            "description": "Produce design spec and API contracts"
        }),
    )
    .await
    .expect("design task create");
    let design_id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(design_task["task_id"].as_str().unwrap_or_default())
            .expect("design task id"),
    );

    let ui_task = exec_tool(
        &tool,
        json!({
            "agent": FIXTURE_UI_AGENT,
            "title": "Implement UI from spec",
            "description": "Build pages and connect to API layer"
        }),
    )
    .await
    .expect("ui task create");

    assert!(
        ui_task["auto_inferred_dependencies"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "ui task should get inferred dependency on design task"
    );

    let all = space.list_all().await;
    let ui_item = all
        .into_iter()
        .find(|t| t.id.to_string() == ui_task["task_id"].as_str().unwrap_or_default())
        .expect("ui task exists");
    assert_eq!(ui_item.status, TodoStatus::Blocked);
    assert!(
        ui_item.depends_on.contains(&design_id),
        "ui task should depend on design task"
    );
}

#[tokio::test]
async fn create_todo_deduplicates_same_goal_agent_and_title() {
    let dir = tempdir().expect("tempdir");
    let db = RedbStore::open(dir.path().join("todo-tests-dedupe.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let goal_id = macaca_proto::TaskId::new();
    let space = Arc::new(TaskSpace::for_session(
        macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
        Some("session".into()),
        Arc::clone(&store),
    ));
    let tool = CreateTodoTool {
        space: Arc::clone(&space),
        coordinator_name: FIXTURE_PLAN_AGENT.into(),
        disallowed_assignees: vec![],
        assignee_capabilities: HashMap::new(),
        active_goal_id: Some(goal_id),
    };

    let first = exec_tool(
        &tool,
        json!({
            "agent": "news_fact_checker",
            "title": "DeepSeek V4 fact check",
            "description": "Verify claims"
        }),
    )
    .await
    .expect("first create_todo should succeed");
    let second = exec_tool(
        &tool,
        json!({
            "agent": "news_fact_checker",
            "title": "  deepseek v4 fact check  ",
            "description": "Verify claims again"
        }),
    )
    .await
    .expect("duplicate create_todo should return existing task");

    assert_eq!(first["task_id"], second["task_id"]);
    assert_eq!(second["deduplicated"].as_bool(), Some(true));
    let all = space.list_all().await;
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn create_todos_creates_multiple_tasks_in_one_call() {
    let dir = tempdir().expect("tempdir");
    let db = RedbStore::open(dir.path().join("todo-tests-batch.redb")).expect("open redb");
    let store = Arc::new(TodoStore::new(Arc::new(db)));
    let app_id = macaca_proto::ApplicationId(uuid::Uuid::new_v4());
    let goal_id = macaca_proto::TaskId::new();
    let space = Arc::new(TaskSpace::for_session(
        app_id,
        Some("session".into()),
        Arc::clone(&store),
    ));
    let tool = CreateTodosTool {
        create_todo: CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: HashMap::new(),
            active_goal_id: Some(goal_id),
        },
    };

    let out = exec_tool(
        &tool,
        json!({
            "tasks": [
                {
                    "agent": "news_researcher",
                    "title": "Collect sources",
                    "description": "Gather source material",
                    "priority": 9
                },
                {
                    "agent": "news_writer",
                    "title": "Draft article",
                    "description": "Write the first draft",
                    "priority": 8,
                    "depends_on_titles": ["Collect sources"]
                }
            ]
        }),
    )
    .await
    .expect("create_todos should succeed");

    assert_eq!(out["count"].as_u64(), Some(2));
    let all = space.list_all().await;
    assert_eq!(all.len(), 2);
    let writer = all
        .iter()
        .find(|task| task.assigned_agent == "news_writer")
        .expect("writer task");
    assert_eq!(writer.depends_on.len(), 1);
}
