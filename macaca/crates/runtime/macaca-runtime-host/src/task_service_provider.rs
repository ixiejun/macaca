//! Runtime-host provider for `service.task`.
//!
//! The Task Service owns generic goal and task-board state for every
//! application.  This provider is a small Adapter between Macaca's typed
//! [`ServiceCommand`] boundary and the existing [`macaca_task::TaskServiceRuntime`].
//! It deliberately avoids application names, workflow names, or domain-specific
//! task semantics; callers must provide provider-neutral command payloads with a
//! trace so the service runtime can keep task collaboration auditable.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, MacacaError, MacacaResult, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, TraceContext,
};
use macaca_task::{
    ClaimTaskCommand, CreateGoalCommand, CreateTaskAssignmentCommand, InMemoryTaskServiceEventSink,
    NoopTaskServiceExecutionStrategy, QueryTaskBoardCommand, ResumeCoordinatorCommand,
    ReviewTaskCommand, StartTaskCommand, SubmitReviewCommand, TaskServiceRuntime,
    TaskServiceSnapshotCommand, TodoStore,
};
use serde::Serialize;
use tracing::info;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Stable command names exposed by `service.task`.
///
/// The names match the Application ABI host import operations for create/query
/// and use explicit task lifecycle verbs for the remaining runtime methods.
/// Keeping these constants in the provider avoids stringly-typed branches being
/// duplicated across WASM, Web bridge, and future CLI adapters.
pub const TASK_CREATE_GOAL_COMMAND: &str = "task.create_goal";
pub const TASK_CREATE_ASSIGNMENT_COMMAND: &str = "task.create_assignment";
pub const TASK_QUERY_COMMAND: &str = "task.query";
pub const TASK_CLAIM_COMMAND: &str = "task.claim";
pub const TASK_START_COMMAND: &str = "task.start";
pub const TASK_SUBMIT_REVIEW_COMMAND: &str = "task.submit_review";
pub const TASK_REVIEW_COMMAND: &str = "task.review";
pub const TASK_RESUME_COORDINATOR_COMMAND: &str = "task.resume_coordinator";
pub const TASK_SNAPSHOT_COMMAND: &str = "service.snapshot";

type LocalTaskRuntime = TaskServiceRuntime<NoopTaskServiceExecutionStrategy>;

/// ServiceRuntime-facing provider for the generic Task Service.
pub struct TaskSystemServiceProvider {
    descriptor: ServiceDescriptor,
    runtime: Arc<LocalTaskRuntime>,
}

impl TaskSystemServiceProvider {
    /// Build a local provider over the shared persistent todo store.
    ///
    /// The execution strategy is intentionally the no-op strategy for this
    /// provider.  It admits goals, records board state, and emits service events
    /// without taking over planner/worker business semantics that remain behind
    /// their own task/agent execution services.
    pub fn local(store: Arc<TodoStore>) -> Self {
        let event_sink = Arc::new(InMemoryTaskServiceEventSink::new());
        let execution = Arc::new(NoopTaskServiceExecutionStrategy);
        Self {
            descriptor: task_service_descriptor(),
            runtime: Arc::new(TaskServiceRuntime::new(store, execution, event_sink)),
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn decode<T: serde::de::DeserializeOwned>(command: ServiceCommand) -> ServiceResult<T> {
        serde_json::from_value(command.payload)
            .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
    }

    fn result<T: Serialize>(output: T, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            status: "ok".into(),
            output: serde_json::to_value(output)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for TaskSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name.as_str(),
            trace_id = %trace.trace_id,
            "task service provider command received"
        );
        match command.name.as_str() {
            TASK_CREATE_GOAL_COMMAND => {
                let typed: CreateGoalCommand = Self::decode(command)?;
                let goal = self.runtime.create_goal(typed).await.map_err(task_error)?;
                Self::result(goal, trace)
            }
            TASK_CREATE_ASSIGNMENT_COMMAND => {
                let typed: CreateTaskAssignmentCommand = Self::decode(command)?;
                let task = self
                    .runtime
                    .create_task_assignment(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(serde_json::json!({ "task": task }), trace)
            }
            TASK_QUERY_COMMAND => {
                let typed: QueryTaskBoardCommand = Self::decode(command)?;
                let todos = self
                    .runtime
                    .query_task_board(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(
                    serde_json::json!({ "todos": todos, "count": todos.len() }),
                    trace,
                )
            }
            TASK_CLAIM_COMMAND => {
                let typed: ClaimTaskCommand = Self::decode(command)?;
                let task = self.runtime.claim_task(typed).await.map_err(task_error)?;
                Self::result(serde_json::json!({ "task": task }), trace)
            }
            TASK_START_COMMAND => {
                let typed: StartTaskCommand = Self::decode(command)?;
                let started = self.runtime.start_task(typed).await.map_err(task_error)?;
                Self::result(serde_json::json!({ "started": started }), trace)
            }
            TASK_SUBMIT_REVIEW_COMMAND => {
                let typed: SubmitReviewCommand = Self::decode(command)?;
                let submitted = self
                    .runtime
                    .submit_review(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(serde_json::json!({ "submitted": submitted }), trace)
            }
            TASK_REVIEW_COMMAND => {
                let typed: ReviewTaskCommand = Self::decode(command)?;
                let reviewed = self.runtime.review_task(typed).await.map_err(task_error)?;
                Self::result(serde_json::json!({ "reviewed": reviewed }), trace)
            }
            TASK_RESUME_COORDINATOR_COMMAND => {
                let typed: ResumeCoordinatorCommand = Self::decode(command)?;
                self.runtime
                    .resume_coordinator(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(serde_json::json!({ "resumed": true }), trace)
            }
            TASK_SNAPSHOT_COMMAND => {
                let typed: TaskServiceSnapshotCommand = Self::decode(command)?;
                let snapshot = self.runtime.snapshot(typed).await.map_err(task_error)?;
                Self::result(snapshot, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported task service command {other}"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.descriptor.health.clone())
    }
}

/// Register and start the local Task Service provider.
///
/// This function is the approved composition root for the built-in local task
/// provider.  Hosts pass the already-created persistent task store, while
/// runtime-host owns only the generic provider registration mechanics.
pub async fn bootstrap_local_task_service(
    runtime: Arc<ServiceRuntime>,
    store: Arc<TodoStore>,
    trace_id: impl Into<String>,
) -> MacacaResult<KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(TaskSystemServiceProvider::local(store));
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "task service registering local provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "task service local provider started"
    );
    Ok(service_id)
}

fn task_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = macaca_task::task_service_descriptor();
    descriptor.metadata.insert(
        "command.task.create_goal".into(),
        TASK_CREATE_GOAL_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.task.create_assignment".into(),
        TASK_CREATE_ASSIGNMENT_COMMAND.into(),
    );
    descriptor
        .metadata
        .insert("command.task.query".into(), TASK_QUERY_COMMAND.into());
    descriptor
        .metadata
        .insert("command.task.claim".into(), TASK_CLAIM_COMMAND.into());
    descriptor
        .metadata
        .insert("command.task.start".into(), TASK_START_COMMAND.into());
    descriptor.metadata.insert(
        "command.task.submit_review".into(),
        TASK_SUBMIT_REVIEW_COMMAND.into(),
    );
    descriptor
        .metadata
        .insert("command.task.review".into(), TASK_REVIEW_COMMAND.into());
    descriptor.metadata.insert(
        "command.task.resume_coordinator".into(),
        TASK_RESUME_COORDINATOR_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.service.snapshot".into(),
        TASK_SNAPSHOT_COMMAND.into(),
    );
    descriptor
}

fn task_error(error: String) -> ServiceError {
    ServiceError::AdapterFailure(error)
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

#[cfg(test)]
mod tests {
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
                    "agent_name": "worker",
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
        // Persist the tempdir for the life of the test process.  RedbStore keeps
        // the database file open, so dropping the directory immediately can
        // remove the file on some platforms before async assertions complete.
        let _leaked_dir = Box::leak(Box::new(dir));
        Arc::new(RedbStore::open(path).expect("redb task provider test store should open"))
    }
}
