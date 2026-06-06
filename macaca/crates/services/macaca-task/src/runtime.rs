//! Local Task Service runtime skeleton.
//!
//! This module intentionally starts as a small host-side orchestration layer
//! for task commands, task service events, and snapshots. The first version is
//! additive and compatible with the existing task board and Plan/Worker loops;
//! later phases may replace the internal execution strategy without changing
//! the command surface.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tracing::{info, warn};

use macaca_proto::{ApplicationId, TodoGoal, TodoItem, TraceContext};

use crate::commands::{
    ClaimTaskCommand, CreateGoalCommand, CreateTaskAssignmentCommand, QueryTaskBoardCommand,
    ResumeCoordinatorCommand, ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand,
    TaskServiceSnapshotCommand,
};
use crate::events::{
    TaskServiceEvent, TaskServiceEventType, TaskServiceGoalSnapshot, TaskServiceSnapshot,
    TaskServiceTaskSnapshot,
};
use crate::todo_board::{TaskBoard, TaskSpace};
use crate::todo_store::TodoStore;

/// Strategy boundary for execution hooks used by the Task Service runtime.
///
/// The runtime does not own planner/reviewer/worker semantics forever. It
/// coordinates commands, state transitions, and audit events, while execution
/// hooks can later be replaced by ServiceRuntime-backed strategies.
#[async_trait]
pub trait TaskServiceExecutionStrategy: Send + Sync {
    /// Handle decomposition after a goal is created.
    async fn decompose_goal(
        &self,
        _goal: &TodoGoal,
        _space: &TaskSpace,
        _trace: Option<TraceContext>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Handle goal evaluation when all tasks are complete.
    async fn evaluate_goal(
        &self,
        _goal: &TodoGoal,
        _tasks: &[TodoItem],
        _trace: Option<TraceContext>,
    ) -> Result<Option<TaskServiceEvent>, String> {
        Ok(None)
    }

    /// Handle resume signaling when the coordinator should continue.
    async fn resume_coordinator(&self, _command: &ResumeCoordinatorCommand) -> Result<(), String> {
        Ok(())
    }
}

/// Default no-op execution strategy used by the initial runtime skeleton.
///
/// The default strategy emits audit events and updates task state, but it does
/// not attempt to replace the existing Web planner/worker pipeline yet. That
/// makes the first implementation additive and easy to validate.
#[derive(Debug, Default, Clone)]
pub struct NoopTaskServiceExecutionStrategy;

#[async_trait]
impl TaskServiceExecutionStrategy for NoopTaskServiceExecutionStrategy {}

/// Runtime-owned Task Service facade.
pub struct TaskServiceRuntime<S = NoopTaskServiceExecutionStrategy> {
    store: Arc<TodoStore>,
    execution: Arc<S>,
    event_sink: Arc<dyn TaskServiceEventSink>,
    snapshots: RwLock<BTreeMap<(String, Option<String>), TaskServiceSnapshot>>,
}

impl<S> TaskServiceRuntime<S>
where
    S: TaskServiceExecutionStrategy + 'static,
{
    /// Create a task service runtime over an existing todo store.
    pub fn new(
        store: Arc<TodoStore>,
        execution: Arc<S>,
        event_sink: Arc<dyn TaskServiceEventSink>,
    ) -> Self {
        Self {
            store,
            execution,
            event_sink,
            snapshots: RwLock::new(BTreeMap::new()),
        }
    }

    /// Emit a structured event through the configured sink.
    async fn emit(&self, event: TaskServiceEvent) {
        info!(
            app_id = %event.app_id.0,
            session_id = ?event.session_id,
            event_type = ?event.event_type,
            "task service event emitted"
        );
        self.event_sink.publish(event).await;
    }

    /// Submit a goal into the runtime and record a goal-ready event.
    pub async fn create_goal(&self, command: CreateGoalCommand) -> Result<TodoGoal, String> {
        let description = command.description.trim().to_string();
        if description.is_empty() {
            return Err("task service goal description cannot be blank".into());
        }

        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let goal = space.push_goal(description.clone()).await;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            None,
            Some(goal.id),
            TaskServiceEventType::GoalReady,
            command.trace.clone(),
            serde_json::json!({
                "goal_id": goal.id.to_string(),
                "description": description,
                "status": format!("{:?}", goal.status),
            }),
        ))
        .await;

        if let Err(error) = self
            .execution
            .decompose_goal(&goal, &space, command.trace.clone())
            .await
        {
            warn!(goal_id = %goal.id, error = %error, "task service goal decomposition hook failed");
        }

        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(goal)
    }

    /// Query a session-scoped task board through the task service boundary.
    pub async fn query_task_board(
        &self,
        command: QueryTaskBoardCommand,
    ) -> Result<Vec<TodoItem>, String> {
        let mut todos = self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await;
        todos.sort_by_key(|todo| todo.sequence_number);
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            Some(command.session_id.clone()),
            None,
            None,
            TaskServiceEventType::TaskSnapshotRequested,
            command.trace.clone(),
            serde_json::json!({
                "count": todos.len(),
                "session_id": command.session_id,
            }),
        ))
        .await;
        Ok(todos)
    }

    /// Create one explicit task assignment in a session-scoped task board.
    ///
    /// This path is for orchestrators that already have a concrete handoff to a
    /// specific agent, such as an Application ABI `agent_delegate` import.  The
    /// Task Service still owns persistence, sequence allocation, dependency
    /// handling, snapshot refresh, and audit events; the caller only supplies the
    /// provider-neutral task metadata.
    pub async fn create_task_assignment(
        &self,
        command: CreateTaskAssignmentCommand,
    ) -> Result<TodoItem, String> {
        let agent_name = command.agent_name.trim().to_string();
        if agent_name.is_empty() {
            return Err("task service assignment agent cannot be blank".into());
        }
        let title = command.title.trim().to_string();
        if title.is_empty() {
            return Err("task service assignment title cannot be blank".into());
        }
        let space = TaskSpace::for_session(
            command.app_id.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let task = space
            .create_task_assignment(
                &agent_name,
                command.created_by.trim(),
                title,
                command.description,
                command.acceptance_criteria,
                command.priority,
                command.depends_on,
                command.parent_task,
            )
            .await;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            Some(command.session_id.clone()),
            Some(task.id),
            task.parent_task,
            TaskServiceEventType::TaskCreated,
            command.trace.clone(),
            serde_json::json!({
                "task_id": task.id.to_string(),
                "agent": task.assigned_agent,
                "status": format!("{:?}", task.status),
                "sequence_number": task.sequence_number,
            }),
        ))
        .await;
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(task)
    }

    /// Claim one task in the session scope.
    pub async fn claim_task(&self, command: ClaimTaskCommand) -> Result<Option<TodoItem>, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let task = board.claim_task(&command.task_id).await;
        if let Some(task) = &task {
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(task.id),
                task.parent_task,
                TaskServiceEventType::TaskClaimed,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": task.id.to_string(),
                    "agent": task.assigned_agent,
                    "status": format!("{:?}", task.status),
                }),
            ))
            .await;
        } else {
            warn!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                agent = %command.agent_name,
                task_id = %command.task_id,
                "task service could not claim requested task"
            );
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(task)
    }

    /// Mark a task as started.
    pub async fn start_task(&self, command: StartTaskCommand) -> Result<bool, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let started = board.mark_task_in_progress(&command.task_id).await;
        if started {
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(command.task_id),
                None,
                TaskServiceEventType::TaskStarted,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "agent": command.agent_name,
                }),
            ))
            .await;
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(started)
    }

    /// Submit a task for review.
    pub async fn submit_review(&self, command: SubmitReviewCommand) -> Result<bool, String> {
        let board = TaskBoard::for_agent(
            command.app_id.clone(),
            command.agent_name.clone(),
            Some(command.session_id.clone()),
            Arc::clone(&self.store),
        );
        let submitted = board
            .submit_task_for_review(&command.task_id, command.summary.clone())
            .await;
        if submitted {
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                Some(command.session_id.clone()),
                Some(command.task_id),
                None,
                TaskServiceEventType::ReviewNeeded,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "summary": command.summary,
                    "agent": command.agent_name,
                }),
            ))
            .await;
        }
        self.refresh_snapshot(&command.app_id, Some(&command.session_id))
            .await;
        Ok(submitted)
    }

    /// Apply a review result to a task and emit the outcome event.
    pub async fn review_task(&self, command: ReviewTaskCommand) -> Result<bool, String> {
        let space = TaskSpace::for_session(
            command.app_id.clone(),
            command.session_id.clone(),
            Arc::clone(&self.store),
        );
        let reviewed = space
            .apply_review_result(
                &command.task_id,
                &command.agent_name,
                command.result.clone(),
            )
            .await;

        if reviewed {
            self.emit(TaskServiceEvent::new(
                command.app_id.clone(),
                command.session_id.clone(),
                Some(command.task_id),
                None,
                TaskServiceEventType::ReviewCompleted,
                command.trace.clone(),
                serde_json::json!({
                    "task_id": command.task_id.to_string(),
                    "passed": command.result.passed,
                    "feedback": command.result.feedback,
                }),
            ))
            .await;
        }

        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(reviewed)
    }

    /// Request coordinator resume when goal or review flow reaches a resume point.
    pub async fn resume_coordinator(
        &self,
        command: ResumeCoordinatorCommand,
    ) -> Result<(), String> {
        self.execution.resume_coordinator(&command).await?;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            command.goal_id,
            command.goal_id,
            TaskServiceEventType::CoordinatorResumeRequested,
            command.trace.clone(),
            serde_json::json!({
                "reason": command.reason,
            }),
        ))
        .await;
        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(())
    }

    /// Return a deterministic snapshot for inspection and diagnostics.
    pub async fn snapshot(
        &self,
        command: TaskServiceSnapshotCommand,
    ) -> Result<TaskServiceSnapshot, String> {
        let snapshot = self
            .build_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            None,
            None,
            TaskServiceEventType::TaskSnapshotEmitted,
            command.trace.clone(),
            serde_json::json!({
                "goals": snapshot.goals.len(),
                "tasks": snapshot.tasks.len(),
            }),
        ))
        .await;
        Ok(snapshot)
    }

    async fn refresh_snapshot(&self, app_id: &ApplicationId, session_id: Option<&str>) {
        let snapshot = self.build_snapshot(app_id, session_id).await;
        let mut snapshots = self.snapshots.write().unwrap();
        snapshots.insert(
            (app_id.to_string(), session_id.map(str::to_string)),
            snapshot,
        );
    }

    async fn build_snapshot(
        &self,
        app_id: &ApplicationId,
        session_id: Option<&str>,
    ) -> TaskServiceSnapshot {
        let mut goals = match session_id {
            Some(session_id) => self.store.list_goals_for_session(app_id, session_id).await,
            None => self.store.list_goals(app_id).await,
        }
        .into_iter()
        .map(|goal| TaskServiceGoalSnapshot {
            goal_id: goal.id,
            description: goal.description,
            status: goal.status,
            session_id: goal.session_id,
        })
        .collect::<Vec<_>>();
        goals.sort_by(|left, right| left.goal_id.0.cmp(&right.goal_id.0));

        let mut tasks = match session_id {
            Some(session_id) => {
                self.store
                    .list_all_todos_for_session(app_id, session_id)
                    .await
            }
            None => self.store.list_all_todos(app_id).await,
        }
        .into_iter()
        .map(|task| TaskServiceTaskSnapshot {
            task_id: task.id,
            title: task.title,
            agent_name: task.assigned_agent,
            status: task.status,
            session_id: task.session_id,
        })
        .collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.task_id.0.cmp(&right.task_id.0));

        TaskServiceSnapshot {
            app_id: app_id.clone(),
            session_id: session_id.map(str::to_string),
            goals,
            tasks,
        }
    }
}

/// Sink boundary for task service events.
#[async_trait]
pub trait TaskServiceEventSink: Send + Sync {
    /// Publish one structured event.
    async fn publish(&self, event: TaskServiceEvent);
}

/// In-memory event sink used by tests and local runtime wiring.
#[derive(Default)]
pub struct InMemoryTaskServiceEventSink {
    events: RwLock<Vec<TaskServiceEvent>>,
}

impl InMemoryTaskServiceEventSink {
    /// Create an empty in-memory sink for tests and local wiring.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a cloned list of published events.
    pub fn snapshot(&self) -> Vec<TaskServiceEvent> {
        self.events.read().unwrap().clone()
    }
}

#[async_trait]
impl TaskServiceEventSink for InMemoryTaskServiceEventSink {
    async fn publish(&self, event: TaskServiceEvent) {
        self.events.write().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

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
                session_id: session_id.into(),
                agent_name: "builder".into(),
                created_by: "planner".into(),
                title: "First independent task".into(),
                description: "This task intentionally stays pending.".into(),
                acceptance_criteria: vec!["The task remains pending.".into()],
                priority: 5,
                depends_on: vec![],
                parent_task: None,
                trace: Some(TraceContext::new("trace-explicit-first")),
            })
            .await
            .unwrap();
        let second = runtime
            .create_task_assignment(CreateTaskAssignmentCommand {
                app_id: app_id.clone(),
                session_id: session_id.into(),
                agent_name: "builder".into(),
                created_by: "planner".into(),
                title: "Second explicit task".into(),
                description: "This task should be claimed by id.".into(),
                acceptance_criteria: vec!["The requested task id is claimed.".into()],
                priority: 5,
                depends_on: vec![],
                parent_task: None,
                trace: Some(TraceContext::new("trace-explicit-second")),
            })
            .await
            .unwrap();

        let claimed = runtime
            .claim_task(ClaimTaskCommand {
                app_id: app_id.clone(),
                session_id: session_id.into(),
                agent_name: "builder".into(),
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
}
