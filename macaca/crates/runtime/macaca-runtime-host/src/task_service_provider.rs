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
    BuildDecompositionPromptCommand, BuildGoalEvaluationPromptCommand, CleanupPolicy,
    CompleteGoalCommand, FailTaskCommand, KernelServiceId, MacacaError, MacacaResult,
    ParseGoalEvaluationCommand, QueryAgentTodosCommand, QueryTaskClaimDiagnosticsCommand,
    QueryTaskGoalsCommand, QueryTaskProgressCommand, ServiceCallResult, ServiceCommand,
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

pub use macaca_proto::{
    TASK_AGENT_TODOS_COMMAND, TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND,
    TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND, TASK_CLAIM_COMMAND, TASK_CLAIM_DIAGNOSTICS_COMMAND,
    TASK_COMPLETE_GOAL_COMMAND, TASK_CREATE_ASSIGNMENT_COMMAND, TASK_CREATE_GOAL_COMMAND,
    TASK_FAIL_COMMAND, TASK_GOALS_COMMAND, TASK_PARSE_GOAL_EVALUATION_COMMAND,
    TASK_PROGRESS_COMMAND, TASK_QUERY_COMMAND, TASK_RESUME_COORDINATOR_COMMAND,
    TASK_REVIEW_COMMAND, TASK_SNAPSHOT_COMMAND, TASK_START_COMMAND, TASK_SUBMIT_REVIEW_COMMAND,
};

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
            TASK_COMPLETE_GOAL_COMMAND => {
                let typed: CompleteGoalCommand = Self::decode(command)?;
                let completed = self
                    .runtime
                    .complete_goal(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(serde_json::json!({ "completed": completed }), trace)
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
                let board = self
                    .runtime
                    .query_task_board(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(board, trace)
            }
            TASK_PROGRESS_COMMAND => {
                let typed: QueryTaskProgressCommand = Self::decode(command)?;
                let progress = self
                    .runtime
                    .query_progress(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(progress, trace)
            }
            TASK_AGENT_TODOS_COMMAND => {
                let typed: QueryAgentTodosCommand = Self::decode(command)?;
                let board = self
                    .runtime
                    .query_agent_todos(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(board, trace)
            }
            TASK_GOALS_COMMAND => {
                let typed: QueryTaskGoalsCommand = Self::decode(command)?;
                let goals = self.runtime.query_goals(typed).await.map_err(task_error)?;
                Self::result(goals, trace)
            }
            TASK_CLAIM_DIAGNOSTICS_COMMAND => {
                let typed: QueryTaskClaimDiagnosticsCommand = Self::decode(command)?;
                let diagnostics = self
                    .runtime
                    .query_claim_diagnostics(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(diagnostics, trace)
            }
            TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND => {
                let typed: BuildDecompositionPromptCommand = Self::decode(command)?;
                let prompt = self
                    .runtime
                    .build_decomposition_prompt(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(prompt, trace)
            }
            TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND => {
                let typed: BuildGoalEvaluationPromptCommand = Self::decode(command)?;
                let prompt = self
                    .runtime
                    .build_goal_evaluation_prompt(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(prompt, trace)
            }
            TASK_PARSE_GOAL_EVALUATION_COMMAND => {
                let typed: ParseGoalEvaluationCommand = Self::decode(command)?;
                let evaluation = self
                    .runtime
                    .parse_goal_evaluation(typed)
                    .await
                    .map_err(task_error)?;
                Self::result(evaluation, trace)
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
            TASK_FAIL_COMMAND => {
                let typed: FailTaskCommand = Self::decode(command)?;
                let failed = self.runtime.fail_task(typed).await.map_err(task_error)?;
                Self::result(serde_json::json!({ "failed": failed }), trace)
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
        "command.task.complete_goal".into(),
        TASK_COMPLETE_GOAL_COMMAND.into(),
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
        .insert("command.task.progress".into(), TASK_PROGRESS_COMMAND.into());
    descriptor.metadata.insert(
        "command.task.agent_todos".into(),
        TASK_AGENT_TODOS_COMMAND.into(),
    );
    descriptor
        .metadata
        .insert("command.task.goals".into(), TASK_GOALS_COMMAND.into());
    descriptor.metadata.insert(
        "command.task.claim_diagnostics".into(),
        TASK_CLAIM_DIAGNOSTICS_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.task.build_decomposition_prompt".into(),
        TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.task.build_goal_evaluation_prompt".into(),
        TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.task.parse_goal_evaluation".into(),
        TASK_PARSE_GOAL_EVALUATION_COMMAND.into(),
    );
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
        .insert("command.task.fail".into(), TASK_FAIL_COMMAND.into());
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
mod tests;
