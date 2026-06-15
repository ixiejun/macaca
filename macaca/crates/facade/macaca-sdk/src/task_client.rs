//! SDK task client boundary for shell-facing task board operations.
//!
//! The SDK keeps task semantics out of Web and CLI. This module models task
//! board reads and task-service calls as typed commands. The service-backed
//! adapter keeps shell code on the canonical service path without constructing
//! task runtimes or reading task stores directly.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::info;

use macaca_proto::{
    AgentTaskBoardResult, ApplicationId, BuildDecompositionPromptCommand,
    BuildGoalEvaluationPromptCommand, ClaimTaskCommand, CompleteGoalCommand, CreateGoalCommand,
    CreateTaskAssignmentCommand, FailTaskCommand, GoalEvaluationResult, MacacaError, MacacaResult,
    ParseGoalEvaluationCommand, QueryAgentTodosCommand, QueryTaskBoardCommand,
    QueryTaskClaimDiagnosticsCommand, QueryTaskGoalsCommand, QueryTaskProgressCommand,
    ResumeCoordinatorCommand, ReviewTaskCommand, SessionClaimDiagnostics, StartTaskCommand,
    SubmitReviewCommand, TaskGoalsResult, TaskProgressSummary, TaskPromptResult,
    TaskServiceSnapshot, TaskServiceSnapshotCommand, TodoGoal, TodoItem, TraceContext,
    TASK_AGENT_TODOS_COMMAND, TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND,
    TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND, TASK_CLAIM_DIAGNOSTICS_COMMAND,
    TASK_COMPLETE_GOAL_COMMAND, TASK_CREATE_ASSIGNMENT_COMMAND, TASK_CREATE_GOAL_COMMAND,
    TASK_FAIL_COMMAND, TASK_GOALS_COMMAND, TASK_PARSE_GOAL_EVALUATION_COMMAND,
    TASK_PROGRESS_COMMAND, TASK_QUERY_COMMAND, TASK_SERVICE_ID,
};

pub use macaca_proto::TaskBoardQueryResult;

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

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

/// Replaceable client for task-board reads.
///
/// The trait is a Strategy boundary: implementations can call local, remote, or
/// plugin-backed Task Service providers without changing Web/CLI command shapes.
#[async_trait]
pub trait SystemTaskClient: Send + Sync {
    /// Return all task items for the requested application/session scope.
    async fn query_task_board(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult>;
}

/// Service-backed task board data source.
///
/// This Adapter keeps the SDK facade on the protocol path: callers provide the
/// stable shell-facing query command, and the data source forwards a typed Task
/// Service command through `SystemServiceClient`. It never owns a local task
/// store or constructs a task runtime.
pub struct ServiceBackedTaskBoardDataSource {
    service: Arc<dyn SystemServiceClient>,
}

/// Runtime-host response wrapper for one Task Service assignment.
///
/// The provider keeps this response object so existing WASM host-import helpers
/// can continue extracting `task.id` without learning any SDK-only shape.
#[derive(Debug, Deserialize)]
struct TaskAssignmentResponse {
    task: TodoItem,
}

/// Runtime-host response wrapper for a goal completion transition.
#[derive(Debug, Deserialize)]
struct GoalCompletionResponse {
    completed: bool,
}

/// Runtime-host response wrapper for a boolean task lifecycle transition.
#[derive(Debug, Deserialize)]
struct TaskLifecycleResponse {
    #[serde(default)]
    submitted: bool,
    #[serde(default)]
    failed: bool,
}

impl ServiceBackedTaskBoardDataSource {
    /// Create a data source over a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }

    /// Create a high-level task goal through the Task Service boundary.
    pub async fn create_goal(&self, command: CreateGoalCommand) -> MacacaResult<TodoGoal> {
        self.call_task_service(TASK_CREATE_GOAL_COMMAND, command, "create-goal")
            .await
    }

    /// Mark a high-level task goal as completed through the Task Service boundary.
    pub async fn complete_goal(&self, command: CompleteGoalCommand) -> MacacaResult<bool> {
        let response: GoalCompletionResponse = self
            .call_task_service(TASK_COMPLETE_GOAL_COMMAND, command, "complete-goal")
            .await?;
        Ok(response.completed)
    }

    /// Create one explicit task assignment through the Task Service boundary.
    pub async fn create_task_assignment(
        &self,
        command: CreateTaskAssignmentCommand,
    ) -> MacacaResult<TodoItem> {
        let response: TaskAssignmentResponse = self
            .call_task_service(
                TASK_CREATE_ASSIGNMENT_COMMAND,
                command,
                "create-task-assignment",
            )
            .await?;
        Ok(response.task)
    }

    /// Query aggregate task progress through the Task Service boundary.
    pub async fn query_progress(
        &self,
        command: QueryTaskProgressCommand,
    ) -> MacacaResult<TaskProgressSummary> {
        self.call_task_service(TASK_PROGRESS_COMMAND, command, "task-progress")
            .await
    }

    /// Query one agent's task board through the Task Service boundary.
    pub async fn query_agent_todos(
        &self,
        command: QueryAgentTodosCommand,
    ) -> MacacaResult<AgentTaskBoardResult> {
        self.call_task_service(TASK_AGENT_TODOS_COMMAND, command, "agent-task-board")
            .await
    }

    /// Query task goals through the Task Service boundary.
    pub async fn query_goals(
        &self,
        command: QueryTaskGoalsCommand,
    ) -> MacacaResult<TaskGoalsResult> {
        self.call_task_service(TASK_GOALS_COMMAND, command, "task-goals")
            .await
    }

    /// Query session claim diagnostics through the Task Service boundary.
    pub async fn query_claim_diagnostics(
        &self,
        command: QueryTaskClaimDiagnosticsCommand,
    ) -> MacacaResult<SessionClaimDiagnostics> {
        self.call_task_service(
            TASK_CLAIM_DIAGNOSTICS_COMMAND,
            command,
            "task-claim-diagnostics",
        )
        .await
    }

    /// Submit a task for review through the Task Service boundary.
    pub async fn submit_review(&self, command: SubmitReviewCommand) -> MacacaResult<bool> {
        let response: TaskLifecycleResponse = self
            .call_task_service(
                macaca_proto::TASK_SUBMIT_REVIEW_COMMAND,
                command,
                "submit-review",
            )
            .await?;
        Ok(response.submitted)
    }

    /// Mark a task as failed through the Task Service boundary.
    pub async fn fail_task(&self, command: FailTaskCommand) -> MacacaResult<bool> {
        let response: TaskLifecycleResponse = self
            .call_task_service(TASK_FAIL_COMMAND, command, "fail-task")
            .await?;
        Ok(response.failed)
    }

    /// Build a goal decomposition prompt through the Task Service boundary.
    pub async fn build_decomposition_prompt(
        &self,
        command: BuildDecompositionPromptCommand,
    ) -> MacacaResult<String> {
        let response: TaskPromptResult = self
            .call_task_service(
                TASK_BUILD_DECOMPOSITION_PROMPT_COMMAND,
                command,
                "build-decomposition-prompt",
            )
            .await?;
        Ok(response.prompt)
    }

    /// Build a goal evaluation prompt through the Task Service boundary.
    pub async fn build_goal_evaluation_prompt(
        &self,
        command: BuildGoalEvaluationPromptCommand,
    ) -> MacacaResult<String> {
        let response: TaskPromptResult = self
            .call_task_service(
                TASK_BUILD_GOAL_EVALUATION_PROMPT_COMMAND,
                command,
                "build-goal-evaluation-prompt",
            )
            .await?;
        Ok(response.prompt)
    }

    /// Parse a goal evaluation response through the Task Service boundary.
    pub async fn parse_goal_evaluation(
        &self,
        command: ParseGoalEvaluationCommand,
    ) -> MacacaResult<GoalEvaluationResult> {
        self.call_task_service(
            TASK_PARSE_GOAL_EVALUATION_COMMAND,
            command,
            "parse-goal-evaluation",
        )
        .await
    }

    async fn call_task_service<C, R>(
        &self,
        command_name: &'static str,
        command: C,
        operation: &'static str,
    ) -> MacacaResult<R>
    where
        C: Serialize,
        R: DeserializeOwned,
    {
        let trace = TraceContext::new(format!("sdk-task-{}-{}", operation, uuid::Uuid::new_v4()));
        info!(
            service_id = TASK_SERVICE_ID,
            command = command_name,
            trace_id = %trace.trace_id,
            "sdk task client dispatching task service command"
        );
        let payload = serde_json::to_value(command)
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        let call =
            ServiceCallCommand::new(TASK_SERVICE_ID, command_name, payload)?.with_trace(trace);
        serde_json::from_value(self.service.call_service(&call).await?.output)
            .map_err(|error| MacacaError::Config(error.to_string()))
    }
}

#[async_trait]
impl SystemTaskClient for ServiceBackedTaskBoardDataSource {
    async fn query_task_board(
        &self,
        command: &TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        let mut trace = TraceContext::new(format!("sdk-task-board-{}", uuid::Uuid::new_v4()));
        trace.session_id = Some(command.session_id.clone());
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            trace_id = %trace.trace_id,
            "sdk task client querying task service"
        );
        let service_command = QueryTaskBoardCommand::new(
            command.app_id,
            command.session_id.clone(),
            Some(trace.clone()),
        );
        let board: TaskBoardQueryResult = self
            .call_task_service(TASK_QUERY_COMMAND, service_command, "task-board")
            .await?;
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count = board.count,
            trace_id = %trace.trace_id,
            "sdk task client completed task service query"
        );
        Ok(board)
    }
}

/// Stable alias for the task-board data-source name.
pub trait TaskBoardDataSource: SystemTaskClient {}

impl<T> TaskBoardDataSource for T where T: SystemTaskClient {}

/// Replaceable client for the typed Task Service boundary.
///
/// The client remains intentionally small and capability-scoped.  Web and CLI
/// can depend on this trait when they need typed goal, claim, review, resume,
/// or snapshot interactions without taking a dependency on the eventual task
/// service host implementation.
#[async_trait]
pub trait TaskServiceClient: Send + Sync {
    /// Create a new goal in the task service boundary.
    async fn create_goal(&self, command: &CreateGoalCommand) -> MacacaResult<TodoGoal>;

    /// Query the existing session-scoped task board.
    async fn query_task_board(
        &self,
        command: &QueryTaskBoardCommand,
    ) -> MacacaResult<TaskBoardQueryResult>;

    /// Claim one task from the task service boundary.
    async fn claim_task(&self, command: &ClaimTaskCommand) -> MacacaResult<Option<TodoItem>>;

    /// Mark a task as started by an agent.
    async fn start_task(&self, command: &StartTaskCommand) -> MacacaResult<bool>;

    /// Submit a task for review.
    async fn submit_review(&self, command: &SubmitReviewCommand) -> MacacaResult<bool>;

    /// Apply a review result to a task.
    async fn review_task(&self, command: &ReviewTaskCommand) -> MacacaResult<bool>;

    /// Request coordinator resume after a task lifecycle milestone.
    async fn resume_coordinator(&self, command: &ResumeCoordinatorCommand) -> MacacaResult<()>;

    /// Inspect the deterministic task service snapshot.
    async fn snapshot(
        &self,
        command: &TaskServiceSnapshotCommand,
    ) -> MacacaResult<TaskServiceSnapshot>;
}

/// Null Object for shells that are not wired to a runtime-backed Task Service.
#[derive(Debug, Default, Clone)]
pub struct UnavailableTaskServiceClient;

#[async_trait]
impl TaskServiceClient for UnavailableTaskServiceClient {
    async fn create_goal(&self, _command: &CreateGoalCommand) -> MacacaResult<TodoGoal> {
        Err(MacacaError::Config(
            "task service create_goal is unavailable through the local SDK task client".into(),
        ))
    }

    async fn query_task_board(
        &self,
        command: &QueryTaskBoardCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        Err(MacacaError::Config(format!(
            "task service board query for session '{}' is unavailable through the local SDK task client",
            command.session_id
        )))
    }

    async fn claim_task(&self, _command: &ClaimTaskCommand) -> MacacaResult<Option<TodoItem>> {
        Err(MacacaError::Config(
            "task service claim_task is unavailable through the local SDK task client".into(),
        ))
    }

    async fn start_task(&self, _command: &StartTaskCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service start_task is unavailable through the local SDK task client".into(),
        ))
    }

    async fn submit_review(&self, _command: &SubmitReviewCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service submit_review is unavailable through the local SDK task client".into(),
        ))
    }

    async fn review_task(&self, _command: &ReviewTaskCommand) -> MacacaResult<bool> {
        Err(MacacaError::Config(
            "task service review_task is unavailable through the local SDK task client".into(),
        ))
    }

    async fn resume_coordinator(&self, _command: &ResumeCoordinatorCommand) -> MacacaResult<()> {
        Err(MacacaError::Config(
            "task service resume_coordinator is unavailable through the local SDK task client"
                .into(),
        ))
    }

    async fn snapshot(
        &self,
        _command: &TaskServiceSnapshotCommand,
    ) -> MacacaResult<TaskServiceSnapshot> {
        Err(MacacaError::Config(
            "task service snapshot is unavailable through the local SDK task client".into(),
        ))
    }
}
