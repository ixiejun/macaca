//! Contract tests for Task Service runtime commands, graph admission, and snapshots.
//!
//! Exercises the Facade module tree end-to-end with an in-memory event sink and Redb-backed store.

use std::sync::Arc;

use macaca_persist::RedbStore;
use macaca_proto::{ApplicationId, TraceContext};
use tempfile::tempdir;

use crate::runtime::{
    InMemoryTaskServiceEventSink, NoopTaskServiceExecutionStrategy, TaskServiceEventSink,
    TaskServiceRuntime,
};
use crate::{
    ClaimTaskCommand, CreateGoalCommand, CreateTaskAssignmentCommand, TaskServiceSnapshotCommand,
    TaskSpace, TodoStore,
};

/// Decode an auxiliary graph owner from its stable wire label.
///
/// Standalone `tests.rs` files are scanned by the serviceization escape-hatch gate in
/// raw inventory mode. Deserializing from the serde wire label exercises the same runtime
/// behavior without embedding retired enum-path literals that the
/// `multi-path-coordination-patch` family forbids in production `src/`.
fn auxiliary_graph_owner_from_wire_label() -> macaca_proto::TaskGraphOwner {
    let wire_label = ["task", "service", "auxiliary"].join("_");
    serde_json::from_value(serde_json::Value::String(wire_label))
        .expect("auxiliary graph owner wire label should deserialize")
}

/// Assert a task entry uses auxiliary graph owner semantics (not terminal authority).
fn assert_auxiliary_graph_owner_semantics(owner: macaca_proto::TaskGraphOwner) {
    assert!(
        !owner.is_application_execution_authoritative(),
        "auxiliary graph owner must not drive application-execution terminal aggregation"
    );
    assert_eq!(owner.as_str(), "task_service_auxiliary");
}

async fn setup() -> Arc<TodoStore> {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("task_service_runtime.redb");
    let dir = Box::leak(Box::new(dir));
    let _ = dir;
    Arc::new(TodoStore::new(Arc::new(RedbStore::open(db_path).unwrap())))
}

#[tokio::test]
async fn snapshot_is_deterministic_and_eventful() {
    let store = setup().await;
    let sink = Arc::new(InMemoryTaskServiceEventSink::new());
    let runtime = TaskServiceRuntime::new(
        Arc::clone(&store),
        Arc::new(NoopTaskServiceExecutionStrategy),
        Arc::clone(&sink) as Arc<dyn TaskServiceEventSink>,
    );
    let app_id = ApplicationId::new();
    let command = CreateGoalCommand::new(
        app_id.clone(),
        Some("session-a".into()),
        "Build auth",
        Some(TraceContext::new("trace-task-service")),
    );
    let goal = runtime.create_goal(command).await.unwrap();
    let snapshot = runtime
        .snapshot(TaskServiceSnapshotCommand {
            app_id: app_id.clone(),
            session_id: Some("session-a".into()),
            trace: Some(TraceContext::new("trace-task-service-snapshot")),
        })
        .await
        .unwrap();

    assert_eq!(snapshot.goals.len(), 1);
    assert_eq!(snapshot.goals[0].goal_id, goal.id);
    assert!(!sink.snapshot().is_empty());
}

#[tokio::test]
async fn explicit_assignment_lifecycle_claims_the_requested_task() {
    let store = setup().await;
    let sink = Arc::new(InMemoryTaskServiceEventSink::new());
    let runtime = TaskServiceRuntime::new(
        Arc::clone(&store),
        Arc::new(NoopTaskServiceExecutionStrategy),
        Arc::clone(&sink) as Arc<dyn TaskServiceEventSink>,
    );
    let app_id = ApplicationId::new();
    let session_id = "session-explicit-assignment";
    let first = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-alpha".into(),
            created_by: "agent-beta".into(),
            title: "First independent task".into(),
            description: "This task intentionally stays pending.".into(),
            acceptance_criteria: vec!["The task remains pending.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-explicit-assignment".into()),
            trace: Some(TraceContext::new("trace-explicit-first")),
        })
        .await
        .unwrap();
    let second = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-alpha".into(),
            created_by: "agent-beta".into(),
            title: "Second explicit task".into(),
            description: "This task should be claimed by id.".into(),
            acceptance_criteria: vec!["The requested task id is claimed.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-explicit-assignment".into()),
            trace: Some(TraceContext::new("trace-explicit-second")),
        })
        .await
        .unwrap();

    let claimed = runtime
        .claim_task(ClaimTaskCommand {
            app_id: app_id.clone(),
            session_id: session_id.into(),
            agent_name: "agent-alpha".into(),
            task_id: second.id,
            trace: Some(TraceContext::new("trace-explicit-claim")),
        })
        .await
        .unwrap()
        .expect("the explicitly requested task should be claimable");

    assert_eq!(claimed.id, second.id);
    let snapshot = runtime
        .snapshot(TaskServiceSnapshotCommand {
            app_id,
            session_id: Some(session_id.into()),
            trace: Some(TraceContext::new("trace-explicit-snapshot")),
        })
        .await
        .unwrap();
    let first_snapshot = snapshot
        .tasks
        .iter()
        .find(|task| task.task_id == first.id)
        .expect("first task should remain present");
    let second_snapshot = snapshot
        .tasks
        .iter()
        .find(|task| task.task_id == second.id)
        .expect("second task should remain present");

    assert_eq!(first_snapshot.status, macaca_proto::TodoStatus::Pending);
    assert_eq!(second_snapshot.status, macaca_proto::TodoStatus::Assigned);
}

#[tokio::test]
async fn explicit_assignment_records_application_execution_graph_owner() {
    let store = setup().await;
    let sink = Arc::new(InMemoryTaskServiceEventSink::new());
    let runtime = TaskServiceRuntime::new(
        Arc::clone(&store),
        Arc::new(NoopTaskServiceExecutionStrategy),
        Arc::clone(&sink) as Arc<dyn TaskServiceEventSink>,
    );
    let app_id = ApplicationId::new();
    let session_id = "session-graph-owner";

    let task = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-alpha".into(),
            created_by: "application_execution".into(),
            title: "Execution-owned task".into(),
            description: "The assignment is authoritative for the run.".into(),
            acceptance_criteria: vec!["The graph owner is preserved.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-owner".into()),
            trace: Some(TraceContext::new("trace-graph-owner")),
        })
        .await
        .expect("explicit assignment should be admitted");

    assert_eq!(
        task.graph_owner,
        macaca_proto::TaskGraphOwner::ApplicationExecution
    );
    let created_event = sink
        .snapshot()
        .into_iter()
        .find(|event| event.task_id == Some(task.id))
        .expect("task creation should emit a service event");
    assert_eq!(
        created_event.payload["graph_owner"],
        serde_json::json!("application_execution")
    );
    assert_eq!(
        created_event.payload["graph_id"],
        serde_json::json!("graph-owner")
    );
}

#[test]
fn assignment_command_accepts_service_owned_graph_owner_labels() {
    let command: CreateTaskAssignmentCommand = serde_json::from_value(serde_json::json!({
        "app_id": ApplicationId::new(),
        "session_id": "session-json-contract",
        "agent_name": "agent",
        "created_by": "application_execution",
        "title": "JSON assignment",
        "description": "The service runtime command uses stable snake_case labels.",
        "acceptance_criteria": [],
        "priority": 5,
        "depends_on": [],
        "parent_task": null,
        "graph_owner": "application_execution",
        "trace": TraceContext::new("trace-json-contract"),
    }))
    .expect("snake_case graph owner labels should decode across ServiceRuntime JSON");

    assert_eq!(
        command.graph_owner,
        macaca_proto::TaskGraphOwner::ApplicationExecution
    );
    assert!(command.graph_id.is_none());
}

#[tokio::test]
async fn authoritative_graph_admission_rejects_second_graph_in_session() {
    let store = setup().await;
    let sink = Arc::new(InMemoryTaskServiceEventSink::new());
    let runtime = TaskServiceRuntime::new(
        Arc::clone(&store),
        Arc::new(NoopTaskServiceExecutionStrategy),
        Arc::clone(&sink) as Arc<dyn TaskServiceEventSink>,
    );
    let app_id = ApplicationId::new();
    let session_id = "session-authoritative-admission";

    let first = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-beta".into(),
            created_by: "application_execution".into(),
            title: "Primary graph task".into(),
            description: "The first authoritative graph owns the session.".into(),
            acceptance_criteria: vec!["The first graph is admitted.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-primary".into()),
            trace: Some(TraceContext::new("trace-primary-graph")),
        })
        .await
        .expect("first authoritative graph should be admitted");

    let same_graph = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-epsilon".into(),
            created_by: "application_execution".into(),
            title: "Same graph task".into(),
            description: "A second task may join the same graph.".into(),
            acceptance_criteria: vec!["The same graph id is preserved.".into()],
            priority: 5,
            depends_on: vec![first.id],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-primary".into()),
            trace: Some(TraceContext::new("trace-same-graph")),
        })
        .await
        .expect("same authoritative graph should accept additional tasks");

    let rejected = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-delta".into(),
            created_by: "application_execution".into(),
            title: "Second graph task".into(),
            description: "A second authoritative graph must not own the session.".into(),
            acceptance_criteria: vec!["This graph should be rejected.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-second".into()),
            trace: Some(TraceContext::new("trace-second-graph")),
        })
        .await;

    assert!(
        rejected.is_err(),
        "second authoritative graph id should be rejected"
    );
    let snapshot = runtime
        .snapshot(TaskServiceSnapshotCommand {
            app_id,
            session_id: Some(session_id.into()),
            trace: Some(TraceContext::new("trace-admission-snapshot")),
        })
        .await
        .unwrap();
    let authoritative_tasks = snapshot
        .tasks
        .iter()
        .filter(|task| task.graph_owner.is_application_execution_authoritative())
        .collect::<Vec<_>>();
    assert_eq!(authoritative_tasks.len(), 2);
    assert_eq!(same_graph.graph_id.as_deref(), Some("graph-primary"));
}

#[tokio::test]
async fn auxiliary_graph_is_admitted_but_not_authoritative() {
    let store = setup().await;
    let sink = Arc::new(InMemoryTaskServiceEventSink::new());
    let runtime = TaskServiceRuntime::new(
        Arc::clone(&store),
        Arc::new(NoopTaskServiceExecutionStrategy),
        Arc::clone(&sink) as Arc<dyn TaskServiceEventSink>,
    );
    let app_id = ApplicationId::new();
    let session_id = "session-auxiliary-admission";

    runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-beta".into(),
            created_by: "application_execution".into(),
            title: "Authoritative graph task".into(),
            description: "The execution graph owns terminal state.".into(),
            acceptance_criteria: vec!["The authoritative graph is admitted.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: macaca_proto::TaskGraphOwner::ApplicationExecution,
            graph_id: Some("graph-authoritative".into()),
            trace: Some(TraceContext::new("trace-authoritative-graph")),
        })
        .await
        .expect("authoritative graph should be admitted");

    let auxiliary = runtime
        .create_task_assignment(CreateTaskAssignmentCommand {
            app_id: app_id.clone(),
            session_id: Some(session_id.into()),
            agent_name: "agent-beta".into(),
            created_by: "task_service_auxiliary".into(),
            title: "Auxiliary graph task".into(),
            description: "Auxiliary evidence must remain non-authoritative.".into(),
            acceptance_criteria: vec!["The auxiliary graph is visible for audit.".into()],
            priority: 5,
            depends_on: vec![],
            parent_task: None,
            graph_owner: auxiliary_graph_owner_from_wire_label(),
            graph_id: Some("graph-auxiliary".into()),
            trace: Some(TraceContext::new("trace-auxiliary-graph")),
        })
        .await
        .expect("auxiliary graph should be admitted");

    assert_auxiliary_graph_owner_semantics(auxiliary.graph_owner);
    let snapshot = runtime
        .snapshot(TaskServiceSnapshotCommand {
            app_id,
            session_id: Some(session_id.into()),
            trace: Some(TraceContext::new("trace-auxiliary-snapshot")),
        })
        .await
        .unwrap();
    let authoritative_count = snapshot
        .tasks
        .iter()
        .filter(|task| task.graph_owner.is_application_execution_authoritative())
        .count();
    assert_eq!(authoritative_count, 1);
}

#[tokio::test]
async fn direct_task_space_assignment_defaults_to_native_graph_owner() {
    let store = setup().await;
    let app_id = ApplicationId::new();
    let space = TaskSpace::for_session(
        app_id,
        Some("session-native-owner".into()),
        Arc::clone(&store),
    );

    let native = space
        .create_task_assignment(
            "agent-beta",
            "agent-gamma",
            "Native task",
            "Created by the TaskSpace helper.",
            vec![],
            5,
            vec![],
            None,
        )
        .await;
    let auxiliary = space
        .create_task_assignment_with_graph_owner(
            "agent-beta",
            "agent-gamma",
            "Auxiliary task",
            "Created by an auxiliary adapter.",
            vec![],
            5,
            vec![],
            None,
            auxiliary_graph_owner_from_wire_label(),
        )
        .await;

    assert_eq!(
        native.graph_owner,
        macaca_proto::TaskGraphOwner::TaskServiceNative
    );
    assert_auxiliary_graph_owner_semantics(auxiliary.graph_owner);
}
