//! Task Service provider contract tests.

use super::*;
use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, ServiceCommandName, TaskId};
use uuid::Uuid;

#[tokio::test]
async fn provider_creates_and_queries_session_scoped_goal() {
    let store = Arc::new(TodoStore::new(test_store()));
    let provider = TaskSystemServiceProvider::local(Arc::clone(&store));
    let app_id = ApplicationId(Uuid::new_v4());
    let session_id = "session-task-provider";
    let trace = TraceContext::new("trace-task-provider-create");
    let create = CreateGoalCommand::new(
        app_id.clone(),
        Some(session_id.into()),
        "Coordinate agents through task service",
        Some(trace.clone()),
    );

    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_CREATE_GOAL_COMMAND),
            payload: serde_json::to_value(create).unwrap(),
            trace: Some(trace),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    let goal: macaca_proto::TodoGoal = serde_json::from_value(result.output).unwrap();
    assert_eq!(goal.description, "Coordinate agents through task service");

    let query = QueryTaskBoardCommand::new(
        app_id,
        session_id,
        Some(TraceContext::new("trace-task-provider-query")),
    );
    let queried = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_QUERY_COMMAND),
            payload: serde_json::to_value(query).unwrap(),
            trace: Some(TraceContext::new("trace-task-provider-query")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(queried.output["count"], serde_json::json!(0));

    let assignment = CreateTaskAssignmentCommand {
        app_id: app_id.clone(),
        session_id: Some(session_id.into()),
        agent_name: "agent-alpha".into(),
        created_by: "agent-reviewer".into(),
        title: "Query provider task board".into(),
        description: "Exercise task query service commands.".into(),
        acceptance_criteria: vec!["command returns typed task payload".into()],
        priority: 5,
        depends_on: Vec::new(),
        parent_task: None,
        graph_owner: Default::default(),
        graph_id: None,
        trace: Some(TraceContext::new("trace-task-provider-assignment")),
    };
    provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_CREATE_ASSIGNMENT_COMMAND),
            payload: serde_json::to_value(assignment).unwrap(),
            trace: Some(TraceContext::new("trace-task-provider-assignment")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();

    let progress = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_PROGRESS_COMMAND),
            payload: serde_json::json!({
                "app_id": app_id,
                "session_id": session_id,
                "trace": TraceContext::new("trace-task-provider-progress"),
            }),
            trace: Some(TraceContext::new("trace-task-provider-progress")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(progress.output["total"], serde_json::json!(1));
    assert_eq!(progress.output["pending"], serde_json::json!(1));

    let agent_board = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_AGENT_TODOS_COMMAND),
            payload: serde_json::json!({
                "app_id": app_id,
                "session_id": null,
                "agent_name": "agent-alpha",
                "trace": TraceContext::new("trace-task-provider-agent-board"),
            }),
            trace: Some(TraceContext::new("trace-task-provider-agent-board")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(
        agent_board.output["agent"],
        serde_json::json!("agent-alpha")
    );
    assert_eq!(agent_board.output["count"], serde_json::json!(1));

    let goals = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_GOALS_COMMAND),
            payload: serde_json::json!({
                "app_id": app_id,
                "session_id": session_id,
                "trace": TraceContext::new("trace-task-provider-goals"),
            }),
            trace: Some(TraceContext::new("trace-task-provider-goals")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(goals.output["count"], serde_json::json!(1));

    let completed = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_COMPLETE_GOAL_COMMAND),
            payload: serde_json::to_value(CompleteGoalCommand::new(
                app_id.clone(),
                Some(session_id.into()),
                goal.id,
                Some(TraceContext::new("trace-task-provider-complete-goal")),
            ))
            .unwrap(),
            trace: Some(TraceContext::new("trace-task-provider-complete-goal")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(completed.output["completed"], serde_json::json!(true));

    let completed_goals = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_GOALS_COMMAND),
            payload: serde_json::json!({
                "app_id": app_id,
                "session_id": session_id,
                "trace": TraceContext::new("trace-task-provider-completed-goals"),
            }),
            trace: Some(TraceContext::new("trace-task-provider-completed-goals")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(
        completed_goals.output["goals"][0]["status"],
        serde_json::json!("Completed")
    );

    let diagnostics = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_CLAIM_DIAGNOSTICS_COMMAND),
            payload: serde_json::json!({
                "app_id": app_id,
                "session_id": session_id,
                "trace": TraceContext::new("trace-task-provider-claim-diagnostics"),
            }),
            trace: Some(TraceContext::new("trace-task-provider-claim-diagnostics")),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(diagnostics.output["agents"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn provider_rejects_missing_trace_before_task_runtime() {
    let store = Arc::new(TodoStore::new(test_store()));
    let provider = TaskSystemServiceProvider::local(store);

    let error = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(TASK_START_COMMAND),
            payload: serde_json::json!({
                "app_id": ApplicationId(Uuid::new_v4()),
                "session_id": "session-a",
                "agent_name": "agent-executor",
                "task_id": TaskId::new(),
                "trace": null,
            }),
            trace: None,
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap_err();

    assert!(matches!(error, ServiceError::MissingTraceContext));
}

fn test_store() -> Arc<dyn macaca_persist::PersistBackend> {
    let dir = tempfile::tempdir().expect("tempdir should be available for task provider test");
    let path = dir.path().join("task-provider.redb");
    // RedbStore keeps the database file open, so the tempdir must outlive the
    // async provider assertions for the current test process.
    let _leaked_dir = Box::leak(Box::new(dir));
    Arc::new(RedbStore::open(path).expect("redb task provider test store should open"))
}
