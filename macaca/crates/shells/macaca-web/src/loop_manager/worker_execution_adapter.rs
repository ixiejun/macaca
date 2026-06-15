//! WorkerLoop task execution adapter via `service.agent_execution`.
//!
//! WorkerLoop retains scheduling/retry semantics; this adapter owns the agent
//! service call and reports task lifecycle transitions through `service.task`.

use std::sync::Arc;

use macaca_host_composition::executor::ApplicationExecutor;
use macaca_proto::{
    AgentExecutionIntent, ApplicationId, FailTaskCommand, SubmitReviewCommand, TaskId, TraceContext,
};
use macaca_sdk::ServiceBackedTaskBoardDataSource;

use super::agent_execution_adapter::run_loop_agent_execution_service_call;
use super::execution_control_adapter::session_loop_coordinator;
use super::planner_helpers::{
    executor_task_completed, executor_task_failed, executor_task_started,
    update_agent_activity_by_name,
};
use crate::session_loop_shell_adapter::{
    wake_plan_loop_and_notify_local, REASON_SESSION_LOOP_WORKER_SUBMIT_REVIEW,
};
use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkerExecutionMode {
    TaskClaimed,
    Retry,
}

impl WorkerExecutionMode {
    fn empty_success_summary(self, title: &str) -> String {
        match self {
            Self::TaskClaimed => format!("Task '{}' completed", title),
            Self::Retry => format!("Task '{}' completed on retry", title),
        }
    }

    pub(crate) fn success_submit_review_detail(self, summary: &str) -> String {
        match self {
            Self::TaskClaimed => summary.chars().take(120).collect::<String>(),
            Self::Retry => "retry_success".to_string(),
        }
    }

    pub(crate) fn panic_error(self) -> &'static str {
        match self {
            Self::TaskClaimed => "Task execution panicked",
            Self::Retry => "Retry task execution panicked",
        }
    }

    pub(crate) fn timeout_error(self) -> &'static str {
        match self {
            Self::TaskClaimed => "Execution timeout (30 min)",
            Self::Retry => "Retry execution timeout (30 min)",
        }
    }
}

pub(crate) fn worker_success_summary(
    mode: WorkerExecutionMode,
    title: &str,
    output: String,
) -> String {
    if output.is_empty() {
        mode.empty_success_summary(title)
    } else {
        output
    }
}

async fn handle_worker_execution_success(
    state: &Arc<AppState>,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: TaskId,
    agent_name: &str,
    title: &str,
    output: String,
    mode: WorkerExecutionMode,
) {
    let summary = worker_success_summary(mode, title, output);
    let task_client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let mut trace = TraceContext::new(format!(
        "worker-submit-review-{}-{}",
        task_id,
        uuid::Uuid::new_v4()
    ));
    trace.session_id = task_session.map(str::to_string);
    trace.task_id = Some(task_id.to_string());
    trace.agent = Some(agent_name.to_string());
    if let Err(error) = task_client
        .submit_review(SubmitReviewCommand {
            app_id: app_id.clone(),
            session_id: task_session.unwrap_or_default().to_string(),
            agent_name: agent_name.to_string(),
            task_id,
            summary: summary.clone(),
            trace: Some(trace),
        })
        .await
    {
        tracing::error!(
            app_id = %app_id,
            task_id = %task_id,
            agent = %agent_name,
            error = %error,
            "worker failed to submit task review through task service"
        );
    }
    executor.broadcast_event(executor_task_completed(
        task_id,
        agent_name,
        summary.clone(),
    ));

    if mode == WorkerExecutionMode::TaskClaimed {
        crate::run_trace::emit_for_scope(
            &state.persist.run_tracer,
            task_session,
            app_id,
            crate::run_trace::phase::WORKER_TASK_SUCCESS,
            "worker_loop",
            crate::run_trace::status::OK,
            None,
            Some(task_id.to_string()),
            None,
            None,
        )
        .await;
    }

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        task_session,
        app_id,
        crate::run_trace::phase::WORKER_SUBMIT_REVIEW,
        "worker_loop",
        crate::run_trace::status::INFO,
        Some(mode.success_submit_review_detail(&summary)),
        Some(task_id.to_string()),
        None,
        None,
    )
    .await;

    let coordinator = session_loop_coordinator(state);
    wake_plan_loop_and_notify_local(
        state,
        &coordinator,
        app_id,
        task_session,
        REASON_SESSION_LOOP_WORKER_SUBMIT_REVIEW,
        Some(task_id.to_string()),
    )
    .await;

    if mode == WorkerExecutionMode::TaskClaimed {
        tracing::info!(agent = %agent_name, "Task completed, submitted for review");
    }
}

async fn handle_worker_execution_failure(
    state: &Arc<AppState>,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: TaskId,
    agent_name: &str,
    error: String,
) {
    fail_task_through_service(
        state,
        app_id,
        task_session,
        task_id,
        agent_name,
        error.clone(),
    )
    .await;
    executor.broadcast_event(executor_task_failed(task_id, agent_name, error.clone()));
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        task_session,
        app_id,
        crate::run_trace::phase::WORKER_TASK_FAILED,
        "worker_loop",
        crate::run_trace::status::ERROR,
        Some(error.chars().take(200).collect()),
        Some(task_id.to_string()),
        None,
        None,
    )
    .await;
}

async fn handle_worker_execution_timeout(
    state: &Arc<AppState>,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: TaskId,
    agent_name: &str,
    mode: WorkerExecutionMode,
) {
    let error = mode.timeout_error();
    fail_task_through_service(
        state,
        app_id,
        task_session,
        task_id,
        agent_name,
        error.to_string(),
    )
    .await;
    executor.broadcast_event(executor_task_failed(task_id, agent_name, error));
}

async fn fail_task_through_service(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: TaskId,
    agent_name: &str,
    error: String,
) {
    let task_client = ServiceBackedTaskBoardDataSource::new(state.system_facade.service_client());
    let mut trace = TraceContext::new(format!(
        "worker-fail-task-{}-{}",
        task_id,
        uuid::Uuid::new_v4()
    ));
    trace.session_id = task_session.map(str::to_string);
    trace.task_id = Some(task_id.to_string());
    trace.agent = Some(agent_name.to_string());
    if let Err(service_error) = task_client
        .fail_task(FailTaskCommand {
            app_id: app_id.clone(),
            session_id: task_session.unwrap_or_default().to_string(),
            agent_name: agent_name.to_string(),
            task_id,
            error,
            trace: Some(trace),
        })
        .await
    {
        tracing::error!(
            app_id = %app_id,
            task_id = %task_id,
            agent = %agent_name,
            error = %service_error,
            "worker failed to record task failure through task service"
        );
    }
}

/// Execute one WorkerLoop-owned task through `service.agent_execution`.
///
/// The worker loop remains responsible for task ordering, retry status, and
/// review submission.  The service owns the agent runtime boundary, keeping
/// worker execution aligned with WASM, YAML, and planner service traces.
pub(crate) async fn execute_worker_task_via_agent_service(
    state: &Arc<AppState>,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    task_id: macaca_proto::TaskId,
    agent_name: &str,
    title: &str,
    prompt: String,
    mode: WorkerExecutionMode,
) {
    executor.broadcast_event(executor_task_started(task_id, agent_name));
    let result = run_loop_agent_execution_service_call(
        state,
        app_id,
        agent_name,
        session_id.map(str::to_string),
        task_id,
        prompt,
        AgentExecutionIntent::TaskWorker,
        "macaca.web.worker_loop",
        std::time::Duration::from_secs(30 * 60),
        serde_json::json!({
            "worker_execution_mode": format!("{mode:?}"),
            "title": title,
        }),
    )
    .await;

    match result {
        Ok(output) => {
            handle_worker_execution_success(
                state, executor, app_id, session_id, task_id, agent_name, title, output, mode,
            )
            .await;
        }
        Err(error) if error.contains("timed out") => {
            handle_worker_execution_timeout(
                state, executor, app_id, session_id, task_id, agent_name, mode,
            )
            .await;
        }
        Err(error) => {
            handle_worker_execution_failure(
                state, executor, app_id, session_id, task_id, agent_name, error,
            )
            .await;
        }
    }

    update_agent_activity_by_name(state, agent_name, macaca_proto::AgentActivity::Idle).await;
}
