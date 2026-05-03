//! PlanLoop and WorkerLoop lifecycle management.
//!
//! `ensure_plan_and_worker_loops` is the main entry point — it idempotently
//! starts a PlanLoop and per-worker WorkerLoops for an application, along
//! with their event consumers that handle task decomposition, review,
//! delegation, and anomaly detection.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::Json;
use futures::FutureExt;

use macaca_app::{app_entry_agent_name_or, app_task_planning_contract, AppPlanningAgentProfile};
use macaca_framework::execution::ExecutionContext;
use macaca_framework::plan::PlanNotebook;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_kernel::executor::{ApplicationExecutor, ExecutorEvent, TaskResult};
use macaca_kernel::AgentInfo;
use macaca_persist::AppendEventCommand;
use macaca_proto::ApplicationId;

use crate::routes::{err, ErrorResponse};
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};
use crate::state::AppState;

fn planner_scope_session_id(app_id: &ApplicationId, session_id: Option<&str>) -> String {
    session_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("_macaca_app_{}", app_id.0))
}

async fn persist_planner_notebook_update(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    update: impl FnOnce(&mut PlanNotebook),
) {
    let sid = planner_scope_session_id(app_id, session_id);
    let mut notebook = PlanNotebook::new();
    let _ = load_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &mut notebook,
    )
    .await;

    update(&mut notebook);

    let _ = save_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &notebook,
    )
    .await;
}

fn select_entry_and_plan_agents(
    agents: &[AgentInfo],
    manifest_entry: Option<&str>,
) -> (String, String) {
    let entry = manifest_entry
        .map(str::to_string)
        .or_else(|| agents.first().map(|a| a.name.clone()))
        .unwrap_or_else(|| "entry_agent".to_string());
    let planner = agents
        .iter()
        .find(|a| a.capabilities.iter().any(|c| c == "task_planning"))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| entry.clone());
    (entry, planner)
}

fn mark_decomposition_in_notebook(
    notebook: &mut PlanNotebook,
    goal_id: macaca_proto::TaskId,
    description: &str,
) {
    notebook.create_plan(
        format!("goal:{}", goal_id),
        description.to_string(),
        "Decompose goal into executable todos",
    );
    if let Some(plan_mut) = notebook.current_plan_mut() {
        plan_mut.add_subtask(
            "decompose_goal",
            format!("Decompose goal {}", goal_id),
            "Todos created and persisted to TodoBoard",
        );
        let _ = plan_mut.start_subtask(0);
        let _ = plan_mut.finish_subtask(0, "decomposition delegated to planner");
    }
    let _ = notebook.finish_plan(format!("goal {} decomposition recorded", goal_id));
}

async fn planner_notebook_mark_decomposition(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    description: &str,
) {
    persist_planner_notebook_update(state, app_id, session_id, |notebook| {
        mark_decomposition_in_notebook(notebook, goal_id, description);
    })
    .await;
}

fn mark_review_in_notebook(
    notebook: &mut PlanNotebook,
    task_id: macaca_proto::TaskId,
    task_title: &str,
) {
    notebook.create_plan(
        format!("review:{}", task_id),
        format!("Review task '{}'", task_title),
        "Task review decision persisted via review_todo",
    );
    if let Some(plan_mut) = notebook.current_plan_mut() {
        plan_mut.add_subtask(
            "review_todo",
            format!("Review todo {}", task_id),
            "Todo status updated to completed/needs_optimization/failed",
        );
        let _ = plan_mut.start_subtask(0);
        let _ = plan_mut.finish_subtask(0, "review delegated to planner");
    }
    let _ = notebook.finish_plan(format!("task {} review recorded", task_id));
}

async fn planner_notebook_mark_review(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    task_id: macaca_proto::TaskId,
    task_title: &str,
) {
    persist_planner_notebook_update(state, app_id, session_id, |notebook| {
        mark_review_in_notebook(notebook, task_id, task_title);
    })
    .await;
}

async fn update_agent_activity_by_name(
    state: &Arc<AppState>,
    agent_name: &str,
    activity: macaca_proto::AgentActivity,
) {
    if let Some(manifest) = state.kernel.get_agent_by_name(agent_name).await {
        state
            .kernel
            .update_agent_activity(&manifest.id, activity)
            .await;
    }
}

fn executor_task_started(task_id: macaca_proto::TaskId, agent: &str) -> ExecutorEvent {
    ExecutorEvent::TaskStarted {
        task_id,
        agent: agent.to_string(),
    }
}

fn executor_task_completed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    output: impl Into<String>,
) -> ExecutorEvent {
    ExecutorEvent::TaskCompleted {
        task_id,
        agent: agent.to_string(),
        result: TaskResult {
            task_id,
            success: true,
            output: output.into(),
            error: None,
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: None,
        },
    }
}

fn executor_task_failed(
    task_id: macaca_proto::TaskId,
    agent: &str,
    error: impl Into<String>,
) -> ExecutorEvent {
    ExecutorEvent::TaskFailed {
        task_id,
        agent: agent.to_string(),
        error: error.into(),
    }
}

fn goal_has_decomposed_tasks(
    tasks: &[macaca_proto::TodoItem],
    goal_id: macaca_proto::TaskId,
) -> bool {
    tasks.iter().any(|task| task.parent_task == Some(goal_id))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlannerFrameworkCallKind {
    DecomposeGoal,
    Review,
    FollowUp,
    GoalEvaluation,
}

impl PlannerFrameworkCallKind {
    fn timeout(self) -> std::time::Duration {
        match self {
            Self::DecomposeGoal => std::time::Duration::from_secs(90),
            Self::Review | Self::FollowUp => std::time::Duration::from_secs(5 * 60),
            Self::GoalEvaluation => std::time::Duration::from_secs(10 * 60),
        }
    }

    fn goal_context(self, task_id: macaca_proto::TaskId) -> Option<macaca_proto::TaskId> {
        match self {
            Self::DecomposeGoal | Self::FollowUp | Self::GoalEvaluation => Some(task_id),
            Self::Review => None,
        }
    }

    fn success_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Planner decomposition completed",
            Self::Review => "Review completed",
            Self::FollowUp => "Follow-up tasks created",
            Self::GoalEvaluation => "Goal evaluation completed",
        }
    }

    fn reply_error_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Planner decomposition failed",
            Self::Review => "Review failed",
            Self::FollowUp => "Follow-up task creation failed",
            Self::GoalEvaluation => "Goal evaluation failed",
        }
    }

    fn build_error_log_prefix(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "Failed to build planner agent",
            Self::Review => "Failed to build planner agent for review",
            Self::FollowUp => "Failed to build planner agent for follow-up",
            Self::GoalEvaluation => "Failed to build planner agent for goal evaluation",
        }
    }

    fn missing_executor_log(self) -> &'static str {
        match self {
            Self::DecomposeGoal => "No executor found for planner decomposition",
            Self::Review => "No executor found for planner review",
            Self::FollowUp => "No executor found for planner follow-up",
            Self::GoalEvaluation => "No executor found for planner goal evaluation",
        }
    }
}

async fn run_planner_framework_call(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    plan_agent_name: &str,
    session_id: Option<String>,
    task_id: macaca_proto::TaskId,
    prompt: String,
    activity_context: String,
    kind: PlannerFrameworkCallKind,
) -> Result<String, String> {
    let Some(executor) = state.executor_registry.get(app_id).await else {
        let error = kind.missing_executor_log().to_string();
        tracing::error!("{}", error);
        return Err(error);
    };

    update_agent_activity_by_name(
        state,
        plan_agent_name,
        macaca_proto::AgentActivity::Working {
            context: activity_context,
        },
    )
    .await;
    executor.broadcast_event(executor_task_started(task_id, plan_agent_name));

    let intent = match kind {
        PlannerFrameworkCallKind::DecomposeGoal => {
            macaca_framework::construction::AgentBuildIntent::PlannerDecomposition {
                goal_id: kind.goal_context(task_id),
            }
        }
        PlannerFrameworkCallKind::Review => {
            macaca_framework::construction::AgentBuildIntent::PlannerReview { task_id }
        }
        PlannerFrameworkCallKind::FollowUp => {
            macaca_framework::construction::AgentBuildIntent::PlannerFollowUp {
                goal_id: kind.goal_context(task_id),
            }
        }
        PlannerFrameworkCallKind::GoalEvaluation => {
            macaca_framework::construction::AgentBuildIntent::GoalEvaluation {
                goal_id: kind.goal_context(task_id),
            }
        }
    };

    let build_result = crate::framework_runner::FrameworkRunner::build_for_intent(
        state,
        app_id,
        plan_agent_name,
        session_id,
        task_id,
        Arc::clone(&executor),
        intent,
    )
    .await;

    let result = match build_result {
        Ok(agent) => {
            use macaca_framework::agent::Agent;

            let msg = macaca_framework::message::Msg::user("plan_loop", prompt.as_str());
            let timeout = kind.timeout();
            match tokio::time::timeout(timeout, agent.reply(msg)).await {
                Ok(Ok(reply)) => {
                    let output = reply.get_text();
                    executor.broadcast_event(executor_task_completed(
                        task_id,
                        plan_agent_name,
                        output.clone(),
                    ));
                    tracing::info!(
                        "{}: {}",
                        kind.success_log_prefix(),
                        output.chars().take(100).collect::<String>()
                    );
                    Ok(output)
                }
                Ok(Err(e)) => {
                    let error = e.to_string();
                    executor.broadcast_event(executor_task_failed(
                        task_id,
                        plan_agent_name,
                        error.clone(),
                    ));
                    tracing::error!("{}: {}", kind.reply_error_log_prefix(), error);
                    Err(error)
                }
                Err(_) => {
                    let error = format!(
                        "{} timed out after {}s",
                        kind.reply_error_log_prefix(),
                        timeout.as_secs()
                    );
                    executor.broadcast_event(executor_task_failed(
                        task_id,
                        plan_agent_name,
                        error.clone(),
                    ));
                    tracing::error!("{}", error);
                    Err(error)
                }
            }
        }
        Err(e) => {
            executor.broadcast_event(executor_task_failed(task_id, plan_agent_name, e.clone()));
            tracing::error!("{}: {}", kind.build_error_log_prefix(), e);
            Err(e)
        }
    };

    update_agent_activity_by_name(state, plan_agent_name, macaca_proto::AgentActivity::Idle).await;
    result
}

async fn list_goal_todos_for_scope(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
) -> Vec<macaca_proto::TodoItem> {
    let todos = match session_id {
        Some(sid) => {
            state
                .persist
                .todo_store
                .list_all_todos_for_session(app_id, sid)
                .await
        }
        None => state.persist.todo_store.list_all_todos(app_id).await,
    };
    todos
        .into_iter()
        .filter(|task| task.parent_task == Some(goal_id))
        .collect()
}

async fn wake_worker_loops(state: &Arc<AppState>, app_id: &ApplicationId) {
    if let Some(wakers) = state.loops.worker_loop_wakers.read().await.get(app_id) {
        for waker in wakers {
            waker.wake();
        }
    }
}

async fn mark_goal_decomposition_ready(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    task_count: usize,
) {
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::InProgress)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_ready",
        "plan_loop",
        crate::run_trace::status::OK,
        Some(format!("tasks={task_count}")),
        None,
        Some(goal_id.to_string()),
        None,
    )
    .await;
    wake_worker_loops(state, app_id).await;
}

async fn mark_goal_decomposition_failed(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    error: &str,
) {
    let mut cancelled = 0usize;
    let mut tasks = list_goal_todos_for_scope(state, app_id, session_id, goal_id).await;
    for task in &mut tasks {
        if matches!(
            task.status,
            macaca_proto::TodoStatus::Pending
                | macaca_proto::TodoStatus::Blocked
                | macaca_proto::TodoStatus::Assigned
        ) {
            task.status = macaca_proto::TodoStatus::Cancelled;
            task.updated_at = chrono::Utc::now();
            state.persist.todo_store.save_todo(task).await;
            cancelled += 1;
        }
    }
    state
        .persist
        .todo_store
        .update_goal_status(app_id, &goal_id, macaca_proto::TodoGoalStatus::Failed)
        .await;
    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_failed",
        "plan_loop",
        crate::run_trace::status::ERROR,
        Some(error.chars().take(200).collect::<String>()),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({ "cancelled_partial_todos": cancelled })),
    )
    .await;
}

#[derive(Clone, Debug)]
struct PlannerWorkerDossier {
    name: String,
    capabilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FallbackTaskPhase {
    Research = 0,
    Validate = 1,
    Analyze = 2,
    Produce = 3,
    Finalize = 4,
    Execute = 5,
}

fn fallback_phase_for(capabilities: &[String]) -> FallbackTaskPhase {
    let text = capabilities.join(" ").to_lowercase();
    if text.contains("research")
        || text.contains("source")
        || text.contains("web")
        || text.contains("discovery")
    {
        FallbackTaskPhase::Research
    } else if text.contains("fact")
        || text.contains("verify")
        || text.contains("validation")
        || text.contains("quality")
    {
        FallbackTaskPhase::Validate
    } else if text.contains("analysis")
        || text.contains("analyze")
        || text.contains("architecture")
        || text.contains("design")
        || text.contains("spec")
        || text.contains("planning")
        || text.contains("interface")
    {
        FallbackTaskPhase::Analyze
    } else if text.contains("write")
        || text.contains("draft")
        || text.contains("implement")
        || text.contains("build")
        || text.contains("code")
        || text.contains("frontend")
        || text.contains("backend")
    {
        FallbackTaskPhase::Produce
    } else if text.contains("edit")
        || text.contains("review")
        || text.contains("package")
        || text.contains("polish")
        || text.contains("test")
        || text.contains("qa")
    {
        FallbackTaskPhase::Finalize
    } else {
        FallbackTaskPhase::Execute
    }
}

fn fallback_task_template(
    phase: FallbackTaskPhase,
    agent_name: &str,
    goal_description: &str,
) -> (String, String, Vec<String>) {
    let clipped_goal = goal_description.chars().take(500).collect::<String>();
    let title = match phase {
        FallbackTaskPhase::Research => "Collect source material and context",
        FallbackTaskPhase::Validate => "Validate facts, assumptions, and constraints",
        FallbackTaskPhase::Analyze => "Produce specialist analysis and handoff brief",
        FallbackTaskPhase::Produce => "Create the primary specialist deliverable",
        FallbackTaskPhase::Finalize => "Review, polish, and package final deliverables",
        FallbackTaskPhase::Execute => "Complete specialist contribution",
    };
    let description = format!(
        "Planner LLM timed out before creating todos. Fallback task for `{agent_name}`.\n\n\
         Goal:\n{clipped_goal}\n\n\
         Use your declared capabilities and allowed tools to produce the durable deliverable \
         expected for this stage. Save outputs under the shared workspace when files are needed, \
         then submit a concise review summary with exact paths."
    );
    let criteria = match phase {
        FallbackTaskPhase::Research => vec![
            "Collect reliable source material or project context relevant to the goal".to_string(),
            "Separate confirmed facts from assumptions or unresolved questions".to_string(),
            "Save a durable handoff artifact in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Validate => vec![
            "Validate important claims, assumptions, dependencies, or constraints".to_string(),
            "Flag unsupported, risky, contradictory, or incomplete information".to_string(),
            "Save findings and safe wording or implementation guidance when applicable".to_string(),
        ],
        FallbackTaskPhase::Analyze => vec![
            "Create a clear specialist brief, plan, architecture, or analysis for downstream work"
                .to_string(),
            "Identify dependencies, risks, and handoff requirements".to_string(),
            "Save the brief in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Produce => vec![
            "Produce the primary deliverable requested by the goal for this specialist area"
                .to_string(),
            "Use upstream handoff artifacts if they exist".to_string(),
            "Save durable output files or a detailed completion summary".to_string(),
        ],
        FallbackTaskPhase::Finalize => vec![
            "Review upstream deliverables for completeness, correctness, and user fit".to_string(),
            "Polish or package final outputs without hiding uncertainty".to_string(),
            "Submit exact output paths and remaining caveats".to_string(),
        ],
        FallbackTaskPhase::Execute => vec![
            "Complete a useful specialist contribution toward the goal".to_string(),
            "Create durable output or a clear handoff summary".to_string(),
            "Submit exact paths, decisions, and unresolved blockers".to_string(),
        ],
    };
    (title.to_string(), description, criteria)
}

async fn create_fallback_decomposition_tasks(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    plan_agent_name: &str,
    goal_description: &str,
    workers: &[PlannerWorkerDossier],
    initial_dependency: Option<macaca_proto::TaskId>,
    planner_error: &str,
) -> Vec<macaca_proto::TodoItem> {
    let mut ordered = workers.to_vec();
    ordered.sort_by_key(|worker| {
        (
            fallback_phase_for(&worker.capabilities),
            worker.name.clone(),
        )
    });
    ordered.truncate(5);

    if ordered.is_empty() {
        return Vec::new();
    }

    tracing::warn!(
        goal_id = %goal_id,
        planner_error = %planner_error,
        fallback_tasks = ordered.len(),
        "Planner produced no todos; creating capability-based fallback task chain"
    );

    let space = macaca_task::TaskSpace::for_session(
        app_id.clone(),
        session_id.map(str::to_string),
        Arc::clone(&state.persist.todo_store),
    );
    let mut created = Vec::new();
    let mut previous: Option<macaca_proto::TaskId> = initial_dependency;

    for (index, worker) in ordered.iter().enumerate() {
        let phase = fallback_phase_for(&worker.capabilities);
        let (title, description, acceptance_criteria) =
            fallback_task_template(phase, &worker.name, goal_description);
        let depends_on = previous.into_iter().collect::<Vec<_>>();
        let item = space
            .create_task_assignment(
                &worker.name,
                plan_agent_name,
                title,
                description,
                acceptance_criteria,
                8u8.saturating_sub(index.min(3) as u8),
                depends_on,
                Some(goal_id),
            )
            .await;
        previous = Some(item.id);
        created.push(item);
    }

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        session_id,
        app_id,
        "plan.goal_decomposition_fallback_ready",
        "plan_loop",
        crate::run_trace::status::INFO,
        Some(format!(
            "planner_error={}; fallback_tasks={}",
            planner_error.chars().take(160).collect::<String>(),
            created.len()
        )),
        None,
        Some(goal_id.to_string()),
        Some(serde_json::json!({
            "task_count": created.len(),
            "planner_error": planner_error,
            "agents": created.iter().map(|task| task.assigned_agent.clone()).collect::<Vec<_>>(),
        })),
    )
    .await;

    created
}

fn terminal_goal_task(tasks: &[macaca_proto::TodoItem]) -> Option<macaca_proto::TaskId> {
    let dependency_ids = tasks
        .iter()
        .flat_map(|task| task.depends_on.iter().copied())
        .collect::<std::collections::HashSet<_>>();
    tasks
        .iter()
        .rev()
        .find(|task| !dependency_ids.contains(&task.id))
        .map(|task| task.id)
        .or_else(|| tasks.last().map(|task| task.id))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkerExecutionMode {
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

    fn success_submit_review_detail(self, summary: &str) -> String {
        match self {
            Self::TaskClaimed => summary.chars().take(120).collect::<String>(),
            Self::Retry => "retry_success".to_string(),
        }
    }

    fn panic_error(self) -> &'static str {
        match self {
            Self::TaskClaimed => "Task execution panicked",
            Self::Retry => "Retry task execution panicked",
        }
    }

    fn timeout_error(self) -> &'static str {
        match self {
            Self::TaskClaimed => "Execution timeout (30 min)",
            Self::Retry => "Retry execution timeout (30 min)",
        }
    }
}

fn worker_success_summary(mode: WorkerExecutionMode, title: &str, output: String) -> String {
    if output.is_empty() {
        mode.empty_success_summary(title)
    } else {
        output
    }
}

async fn handle_worker_execution_success(
    state: &Arc<AppState>,
    board: &macaca_task::TaskBoard,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: macaca_proto::TaskId,
    agent_name: &str,
    title: &str,
    output: String,
    mode: WorkerExecutionMode,
) {
    let summary = worker_success_summary(mode, title, output);
    board
        .submit_task_for_review(&task_id, summary.clone())
        .await;
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

    if let Some(waker) = state.loops.plan_loop_wakers.read().await.get(app_id) {
        waker.wake();
    }

    if mode == WorkerExecutionMode::TaskClaimed {
        tracing::info!(agent = %agent_name, "Task completed, submitted for review");
    }
}

async fn handle_worker_execution_failure(
    state: &Arc<AppState>,
    board: &macaca_task::TaskBoard,
    executor: &ApplicationExecutor,
    app_id: &ApplicationId,
    task_session: Option<&str>,
    task_id: macaca_proto::TaskId,
    agent_name: &str,
    error: String,
) {
    board.fail_task(&task_id, error.clone()).await;
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
    board: &macaca_task::TaskBoard,
    executor: &ApplicationExecutor,
    task_id: macaca_proto::TaskId,
    agent_name: &str,
    mode: WorkerExecutionMode,
) {
    let error = mode.timeout_error();
    board.fail_task(&task_id, error.into()).await;
    executor.broadcast_event(executor_task_failed(task_id, agent_name, error));
}

/// Ensure PlanLoop and WorkerLoops are running for an application.
/// Idempotent — safe to call multiple times.
///
/// `session_id` scopes the PlanLoop and WorkerLoops to a specific session so
/// that progress tracking (e.g. failed-task anomaly alerts) only considers
/// tasks belonging to the current session instead of the full history.
pub(crate) async fn ensure_plan_and_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<String>,
) {
    // Determine entry/planner agents from declarative config + capabilities.
    let manifest_entry = {
        let registry = state.registry.read().await;
        registry
            .get_app(app_id)
            .map(|app| app_entry_agent_name_or(&app.manifest, "entry_agent"))
    };
    let (entry_agent_name, plan_agent_name): (String, String) =
        if let Some(executor) = state.executor_registry.get(app_id).await {
            let agents = executor.list_agents().await;
            select_entry_and_plan_agents(&agents, manifest_entry.as_deref())
        } else {
            let entry = manifest_entry.unwrap_or_else(|| "entry_agent".to_string());
            (entry.clone(), entry)
        };

    // ── Migrate legacy tasks (one-time, idempotent) ──
    state
        .persist
        .todo_store
        .migrate_sequence_numbers(app_id)
        .await;

    // ── PlanLoop ──
    {
        let already = state
            .loops
            .plan_loop_handles
            .read()
            .await
            .contains_key(app_id);
        if !already {
            let mut handles = state.loops.plan_loop_handles.write().await;
            if !handles.contains_key(app_id) {
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                handles.insert(app_id.clone(), Arc::clone(&shutdown));

                let store = Arc::clone(&state.persist.todo_store);
                let plan_space = Arc::new(macaca_task::TaskSpace::for_session(
                    app_id.clone(),
                    session_id.clone(),
                    Arc::clone(&store),
                ));
                let plan_loop = macaca_task::PlanLoop::with_components(
                    plan_space,
                    macaca_task::PlanLoopConfig::default(),
                );
                let plan_waker = plan_loop.waker();
                state
                    .loops
                    .plan_loop_wakers
                    .write()
                    .await
                    .insert(app_id.clone(), plan_waker);

                let (event_tx, mut event_rx) =
                    tokio::sync::mpsc::channel::<macaca_task::PlanEvent>(64);
                let event_tx_for_consumer = event_tx.clone();

                tokio::spawn(async move {
                    plan_loop
                        .run_with_default_template(shutdown, event_tx)
                        .await;
                });

                let rt_plan_start = Arc::clone(&state.persist.run_tracer);
                let app_plan_start = app_id.clone();
                tokio::spawn(async move {
                    crate::run_trace::emit_for_scope(
                        &rt_plan_start,
                        None,
                        &app_plan_start,
                        crate::run_trace::phase::PLAN_LOOP_STARTED,
                        "plan_loop",
                        crate::run_trace::status::INFO,
                        Some("spawned".into()),
                        None,
                        None,
                        None,
                    )
                    .await;
                });

                let state_for_consumer = Arc::clone(state);
                let app_id_for_consumer = app_id.clone();
                let session_store_for_consumer = Arc::clone(&state.persist.session_store);
                let plan_agent_name_for_loop = plan_agent_name.clone();
                // Capture entry agent so decomposition only lists worker candidates.
                let entry_agent_for_loop = entry_agent_name.clone();
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        match event {
                            macaca_task::PlanEvent::GoalReady {
                                goal_id,
                                description,
                                session_id,
                            } => {
                                let existing_goal_tasks = match session_id.as_deref() {
                                    Some(sid) => {
                                        state_for_consumer
                                            .persist
                                            .todo_store
                                            .list_all_todos_for_session(&app_id_for_consumer, sid)
                                            .await
                                    }
                                    None => {
                                        state_for_consumer
                                            .persist
                                            .todo_store
                                            .list_all_todos(&app_id_for_consumer)
                                            .await
                                    }
                                };
                                if goal_has_decomposed_tasks(&existing_goal_tasks, goal_id) {
                                    tracing::info!(
                                        goal_id = %goal_id,
                                        session_id = ?session_id,
                                        "Skipping stale GoalReady event because goal already has tasks"
                                    );
                                    continue;
                                }

                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    session_id.as_deref(),
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_GOAL_READY,
                                    "plan_loop",
                                    crate::run_trace::status::INFO,
                                    Some("decompose_goal".into()),
                                    None,
                                    Some(goal_id.to_string()),
                                    Some(serde_json::json!({
                                        "description": description.chars().take(240).collect::<String>(),
                                    })),
                                )
                                .await;
                                planner_notebook_mark_decomposition(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    session_id.as_deref(),
                                    goal_id,
                                    &description,
                                )
                                .await;
                                {
                                    // Dynamically get available worker agents + capabilities (no hardcoding)
                                    let (worker_profiles, worker_dossiers): (
                                        Vec<AppPlanningAgentProfile>,
                                        Vec<PlannerWorkerDossier>,
                                    ) = if let Some(executor) = state_for_consumer
                                        .executor_registry
                                        .get(&app_id_for_consumer)
                                        .await
                                    {
                                        let agents = executor.list_agents().await;
                                        let manifest_by_name: std::collections::HashMap<_, _> =
                                            state_for_consumer
                                                .kernel
                                                .list_agents()
                                                .await
                                                .into_iter()
                                                .map(|m| (m.name.clone(), m))
                                                .collect();
                                        let workers: Vec<_> = agents
                                            .iter()
                                            .filter(|a| {
                                                a.name != entry_agent_for_loop
                                                    && a.name != plan_agent_name_for_loop
                                            })
                                            .collect();
                                        let dossiers = workers
                                            .iter()
                                            .map(|a| {
                                                let capabilities = manifest_by_name
                                                    .get(&a.name)
                                                    .map(|m| {
                                                        m.capabilities
                                                            .iter()
                                                            .map(|c| {
                                                                format!(
                                                                    "{}: {}",
                                                                    c.name, c.description
                                                                )
                                                            })
                                                            .collect::<Vec<_>>()
                                                    })
                                                    .filter(|caps| !caps.is_empty())
                                                    .unwrap_or_else(|| a.capabilities.clone());
                                                PlannerWorkerDossier {
                                                    name: a.name.clone(),
                                                    capabilities,
                                                }
                                            })
                                            .collect();
                                        let profiles = workers
                                            .iter()
                                            .map(|a| {
                                                let manifest = manifest_by_name.get(&a.name);
                                                let capabilities = manifest
                                                    .map(|m| {
                                                        m.capabilities
                                                            .iter()
                                                            .map(|c| {
                                                                format!(
                                                                    "{}: {}",
                                                                    c.name, c.description
                                                                )
                                                            })
                                                            .collect::<Vec<_>>()
                                                    })
                                                    .filter(|caps| !caps.is_empty())
                                                    .unwrap_or_else(|| a.capabilities.clone());
                                                let allowed_tools = manifest
                                                    .map(|m| m.permission.allowed_tools.clone())
                                                    .unwrap_or_default();
                                                let permission_level = manifest
                                                    .map(|m| match m.permission.level {
                                                        macaca_proto::PermissionLevel::System => {
                                                            "system".to_string()
                                                        }
                                                        macaca_proto::PermissionLevel::User => {
                                                            "user".to_string()
                                                        }
                                                    })
                                                    .unwrap_or_else(|| "unknown".to_string());
                                                let model = manifest
                                                    .map(|m| m.model.clone())
                                                    .filter(|model| !model.is_empty())
                                                    .unwrap_or_else(|| "app default".to_string());
                                                AppPlanningAgentProfile {
                                                    name: a.name.clone(),
                                                    capabilities,
                                                    available: a.available,
                                                    current_load: a.current_load as usize,
                                                    max_load: a.max_load as usize,
                                                    permission_level,
                                                    model,
                                                    allowed_tools,
                                                }
                                            })
                                            .collect();
                                        (profiles, dossiers)
                                    } else {
                                        (vec![], vec![])
                                    };
                                    let prompt = {
                                        let registry = state_for_consumer.registry.read().await;
                                        if let Some(app) = registry.get_app(&app_id_for_consumer) {
                                            let contract = app_task_planning_contract(
                                                &app.manifest,
                                                worker_profiles.clone(),
                                            );
                                            macaca_task::build_decomposition_prompt(
                                                &description,
                                                &contract,
                                            )
                                        } else {
                                            macaca_task::build_decomposition_prompt(
                                                &description,
                                                &macaca_app::AppTaskPlanningContract {
                                                    workflow_name: "default".into(),
                                                    entry_agent: entry_agent_for_loop.clone(),
                                                    worker_agents: worker_profiles.clone(),
                                                },
                                            )
                                        }
                                    };
                                    let decomposition_result = run_planner_framework_call(
                                        &state_for_consumer,
                                        &app_id_for_consumer,
                                        &plan_agent_name_for_loop,
                                        session_id.clone(),
                                        goal_id,
                                        prompt,
                                        format!(
                                            "Decomposing goal: {}",
                                            description.chars().take(80).collect::<String>()
                                        ),
                                        PlannerFrameworkCallKind::DecomposeGoal,
                                    )
                                    .await;
                                    let goal_tasks = list_goal_todos_for_scope(
                                        &state_for_consumer,
                                        &app_id_for_consumer,
                                        session_id.as_deref(),
                                        goal_id,
                                    )
                                    .await;
                                    match decomposition_result {
                                        Ok(_) if !goal_tasks.is_empty() => {
                                            mark_goal_decomposition_ready(
                                                &state_for_consumer,
                                                &app_id_for_consumer,
                                                session_id.as_deref(),
                                                goal_id,
                                                goal_tasks.len(),
                                            )
                                            .await;
                                        }
                                        Ok(_) => {
                                            let fallback_tasks = create_fallback_decomposition_tasks(
                                                &state_for_consumer,
                                                &app_id_for_consumer,
                                                session_id.as_deref(),
                                                goal_id,
                                                &plan_agent_name_for_loop,
                                                &description,
                                                &worker_dossiers,
                                                None,
                                                "Planner decomposition returned without creating todos",
                                            )
                                            .await;
                                            if fallback_tasks.is_empty() {
                                                mark_goal_decomposition_failed(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    "Planner decomposition returned without creating todos",
                                                )
                                                .await;
                                            } else {
                                                mark_goal_decomposition_ready(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    fallback_tasks.len(),
                                                )
                                                .await;
                                            }
                                        }
                                        Err(error) if !goal_tasks.is_empty() => {
                                            let existing_agents = goal_tasks
                                                .iter()
                                                .map(|task| task.assigned_agent.clone())
                                                .collect::<std::collections::HashSet<_>>();
                                            let remaining_workers = worker_dossiers
                                                .iter()
                                                .filter(|worker| {
                                                    !existing_agents.contains(&worker.name)
                                                })
                                                .cloned()
                                                .collect::<Vec<_>>();
                                            let fallback_tasks =
                                                create_fallback_decomposition_tasks(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    &plan_agent_name_for_loop,
                                                    &description,
                                                    &remaining_workers,
                                                    terminal_goal_task(&goal_tasks),
                                                    &error,
                                                )
                                                .await;
                                            let total_tasks =
                                                goal_tasks.len() + fallback_tasks.len();
                                            tracing::warn!(
                                                goal_id = %goal_id,
                                                task_count = total_tasks,
                                                error = %error,
                                                "Planner decomposition failed after creating todos; allowing partial decomposition to proceed"
                                            );
                                            crate::run_trace::emit_for_scope(
                                                &state_for_consumer.persist.run_tracer,
                                                session_id.as_deref(),
                                                &app_id_for_consumer,
                                                "plan.goal_decomposition_partial_ready",
                                                "plan_loop",
                                                crate::run_trace::status::INFO,
                                                Some(format!(
                                                    "planner_error={}; tasks={}",
                                                    error.chars().take(160).collect::<String>(),
                                                    total_tasks
                                                )),
                                                None,
                                                Some(goal_id.to_string()),
                                                Some(serde_json::json!({
                                                    "task_count": total_tasks,
                                                    "planner_created_tasks": goal_tasks.len(),
                                                    "fallback_added_tasks": fallback_tasks.len(),
                                                    "planner_error": error,
                                                })),
                                            )
                                            .await;
                                            mark_goal_decomposition_ready(
                                                &state_for_consumer,
                                                &app_id_for_consumer,
                                                session_id.as_deref(),
                                                goal_id,
                                                total_tasks,
                                            )
                                            .await;
                                        }
                                        Err(error) => {
                                            let fallback_tasks =
                                                create_fallback_decomposition_tasks(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    &plan_agent_name_for_loop,
                                                    &description,
                                                    &worker_dossiers,
                                                    None,
                                                    &error,
                                                )
                                                .await;
                                            if fallback_tasks.is_empty() {
                                                mark_goal_decomposition_failed(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    &error,
                                                )
                                                .await;
                                            } else {
                                                mark_goal_decomposition_ready(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    session_id.as_deref(),
                                                    goal_id,
                                                    fallback_tasks.len(),
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                    crate::run_trace::emit_for_scope(
                                        &state_for_consumer.persist.run_tracer,
                                        session_id.as_deref(),
                                        &app_id_for_consumer,
                                        crate::run_trace::phase::PLAN_GOAL_DELEGATE,
                                        "plan_loop",
                                        crate::run_trace::status::INFO,
                                        Some(format!("delegated_to={plan_agent_name_for_loop}")),
                                        None,
                                        Some(goal_id.to_string()),
                                        None,
                                    )
                                    .await;
                                }
                                // Emit SSE decision event
                                let msg = format!(
                                    "New goal submitted, decomposing into tasks: {}",
                                    description
                                );
                                let plan_payload = serde_json::json!({
                                    "decision_type": "goal_ready",
                                    "goal_id": goal_id.to_string(),
                                    "description": description,
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    sse_event,
                                    plan_payload,
                                )
                                .await;
                                // Persist decision
                                save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                        decision_type: "goal_ready".into(),
                                        message: msg,
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": description }),
                                    }).await;
                            }
                            macaca_task::PlanEvent::ReviewNeeded {
                                task_id,
                                agent,
                                title,
                                summary,
                                criteria,
                                session_id,
                            } => {
                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    session_id.as_deref(),
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_REVIEW_NEEDED,
                                    "plan_loop",
                                    crate::run_trace::status::WAITING,
                                    Some(format!("title={title} agent={agent}")),
                                    Some(task_id.to_string()),
                                    None,
                                    None,
                                )
                                .await;
                                planner_notebook_mark_review(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    session_id.as_deref(),
                                    task_id,
                                    &title,
                                )
                                .await;
                                {
                                    let prompt = format!(
                                        "Review this completed task using review_todo:\n\
                                         Task ID: {}\n Agent: {}\n Title: {}\n\
                                         Summary: {}\n Criteria: {:?}\n\n\
                                         Verify the work meets the criteria. Use review_todo with passed=true/false and feedback.",
                                        task_id, agent, title, summary, criteria
                                    );
                                    let _ = run_planner_framework_call(
                                        &state_for_consumer,
                                        &app_id_for_consumer,
                                        &plan_agent_name_for_loop,
                                        session_id.clone(),
                                        task_id,
                                        prompt,
                                        format!("Reviewing {} task: {}", agent, title),
                                        PlannerFrameworkCallKind::Review,
                                    )
                                    .await;
                                    crate::run_trace::emit_for_scope(
                                        &state_for_consumer.persist.run_tracer,
                                        session_id.as_deref(),
                                        &app_id_for_consumer,
                                        crate::run_trace::phase::PLAN_REVIEW_DELEGATE,
                                        "plan_loop",
                                        crate::run_trace::status::INFO,
                                        Some("delegated_review_to_planner".into()),
                                        Some(task_id.to_string()),
                                        None,
                                        None,
                                    )
                                    .await;
                                    // Review delegate completed — wake all WorkerLoops so unblocked tasks are claimed immediately
                                    if let Some(wakers) = state_for_consumer
                                        .loops
                                        .worker_loop_wakers
                                        .read()
                                        .await
                                        .get(&app_id_for_consumer)
                                    {
                                        for w in wakers {
                                            w.wake();
                                        }
                                    }
                                    // Broadcast review result as SSE + EventLog
                                    let review_payload = serde_json::json!({
                                        "decision_type": "task_reviewed",
                                        "task_id": task_id.to_string(),
                                        "agent": agent,
                                        "title": title,
                                        "message": format!("Task '{}' reviewed for agent {}", title, agent),
                                    });
                                    let review_event = Event::default()
                                        .event("plan_decision")
                                        .data(review_payload.to_string());
                                    broadcast_to_app_sessions(
                                        &state_for_consumer,
                                        &app_id_for_consumer,
                                        review_event,
                                        review_payload,
                                    )
                                    .await;
                                }
                                // Emit SSE decision event
                                let msg =
                                    format!("Task '{}' submitted for review by {}", title, agent);
                                let plan_payload = serde_json::json!({
                                    "decision_type": "review_needed",
                                    "task_id": task_id.to_string(),
                                    "agent": agent,
                                    "title": title,
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    sse_event,
                                    plan_payload,
                                )
                                .await;
                                // Persist decision
                                save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                        decision_type: "review_needed".into(),
                                        message: msg,
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "task_id": task_id.to_string(), "agent": agent, "title": title }),
                                    }).await;
                            }
                            macaca_task::PlanEvent::AllTasksDone { completed, failed } => {
                                tracing::info!(completed, failed, "All tasks done for app");
                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    None,
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_ALL_TASKS_DONE,
                                    "plan_loop",
                                    crate::run_trace::status::INFO,
                                    Some(format!("completed={completed} failed={failed}")),
                                    None,
                                    None,
                                    Some(serde_json::json!({ "completed": completed, "failed": failed })),
                                )
                                .await;
                            }
                            macaca_task::PlanEvent::AnomalyDetected { ref message } => {
                                tracing::warn!(message, "Plan loop anomaly detected");
                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    None,
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_ANOMALY,
                                    "plan_loop",
                                    crate::run_trace::status::ERROR,
                                    Some(message.clone()),
                                    None,
                                    None,
                                    None,
                                )
                                .await;
                                state_for_consumer
                                    .config
                                    .alert_manager
                                    .fire(macaca_kernel::alert::Alert::warning(
                                        "Task Anomaly",
                                        message.clone(),
                                        "plan_loop",
                                    ))
                                    .await;
                                // Emit SSE decision event
                                let msg = message.clone();
                                let plan_payload = serde_json::json!({
                                    "decision_type": "anomaly",
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    sse_event,
                                    plan_payload,
                                )
                                .await;
                                // Persist decision
                                save_plan_decision(
                                    &session_store_for_consumer,
                                    &app_id_for_consumer,
                                    PlanDecisionEvent {
                                        decision_type: "anomaly".into(),
                                        message: msg.clone(),
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "message": msg }),
                                    },
                                )
                                .await;
                            }
                            macaca_task::PlanEvent::EvaluateGoalCompletion {
                                goal_id,
                                goal_description,
                                completed_count,
                                failed_count,
                                task_summaries,
                                session_id,
                            } => {
                                tracing::info!(goal_id = %goal_id, "Evaluating goal completion");
                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    session_id.as_deref(),
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_EVALUATE_GOAL,
                                    "plan_loop",
                                    crate::run_trace::status::INFO,
                                    Some(format!("completed={completed_count} failed={failed_count}")),
                                    None,
                                    Some(goal_id.to_string()),
                                    Some(serde_json::json!({
                                        "description": goal_description.chars().take(200).collect::<String>(),
                                    })),
                                )
                                .await;
                                let evaluation_prompt = macaca_task::GoalEvaluator::build_prompt(
                                    &goal_description,
                                    &task_summaries,
                                    completed_count,
                                    failed_count,
                                );
                                let evaluation_result = run_planner_framework_call(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    &plan_agent_name_for_loop,
                                    session_id.clone(),
                                    goal_id,
                                    evaluation_prompt,
                                    format!(
                                        "Evaluating goal completion: {}",
                                        goal_description.chars().take(80).collect::<String>()
                                    ),
                                    PlannerFrameworkCallKind::GoalEvaluation,
                                )
                                .await
                                .map(|reply| {
                                    macaca_task::GoalEvaluator::parse_eval_response(&reply)
                                });
                                match evaluation_result {
                                    Ok(macaca_task::GoalEvaluation::Satisfied { summary }) => {
                                        tracing::info!(goal_id = %goal_id, summary = %summary, "Goal satisfied");
                                        let space = macaca_task::TaskSpace::for_session(
                                            app_id_for_consumer.clone(),
                                            None,
                                            Arc::clone(&state_for_consumer.persist.todo_store),
                                        );
                                        space.complete_goal(&goal_id).await;
                                        let _ = event_tx_for_consumer
                                            .send(macaca_task::PlanEvent::GoalCompleted {
                                                goal_id: goal_id.clone(),
                                                description: goal_description.clone(),
                                            })
                                            .await;
                                        // Emit SSE decision event
                                        let msg = format!("Goal completed: {}", summary);
                                        let plan_payload = serde_json::json!({
                                            "decision_type": "goal_satisfied",
                                            "goal_id": goal_id.to_string(),
                                            "description": goal_description,
                                            "summary": summary,
                                            "message": msg,
                                        });
                                        let sse_event = Event::default()
                                            .event("plan_decision")
                                            .data(plan_payload.to_string());
                                        broadcast_to_app_sessions(
                                            &state_for_consumer,
                                            &app_id_for_consumer,
                                            sse_event,
                                            plan_payload,
                                        )
                                        .await;
                                        // Persist decision
                                        save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                                decision_type: "goal_satisfied".into(),
                                                message: msg,
                                                timestamp: chrono::Utc::now(),
                                                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "summary": summary }),
                                            }).await;
                                        crate::run_trace::emit_for_scope(
                                            &state_for_consumer.persist.run_tracer,
                                            session_id.as_deref(),
                                            &app_id_for_consumer,
                                            crate::run_trace::phase::PLAN_GOAL_SATISFIED,
                                            "plan_loop",
                                            crate::run_trace::status::OK,
                                            Some(summary.chars().take(200).collect::<String>()),
                                            None,
                                            Some(goal_id.to_string()),
                                            None,
                                        )
                                        .await;
                                    }
                                    Ok(macaca_task::GoalEvaluation::NeedsMoreWork {
                                        reason,
                                        suggestions,
                                    }) => {
                                        tracing::info!(goal_id = %goal_id, reason = %reason, "Goal needs more work");
                                        {
                                            let prompt = format!(
                                                "The goal '{}' needs additional work. Reason: {}\nSuggestions:\n{}\n\nCreate follow-up tasks using create_todo.",
                                                goal_description, reason,
                                                suggestions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
                                            );
                                            let _ = run_planner_framework_call(
                                                &state_for_consumer,
                                                &app_id_for_consumer,
                                                &plan_agent_name_for_loop,
                                                session_id.clone(),
                                                goal_id,
                                                prompt,
                                                format!(
                                                    "Planning follow-up work: {}",
                                                    reason.chars().take(80).collect::<String>()
                                                ),
                                                PlannerFrameworkCallKind::FollowUp,
                                            )
                                            .await;
                                        }
                                        // Emit SSE decision event
                                        let msg = format!("Goal needs more work: {}", reason);
                                        let plan_payload = serde_json::json!({
                                            "decision_type": "goal_needs_work",
                                            "goal_id": goal_id.to_string(),
                                            "description": goal_description,
                                            "reason": reason,
                                            "suggestions": suggestions,
                                            "message": msg,
                                        });
                                        let sse_event = Event::default()
                                            .event("plan_decision")
                                            .data(plan_payload.to_string());
                                        broadcast_to_app_sessions(
                                            &state_for_consumer,
                                            &app_id_for_consumer,
                                            sse_event,
                                            plan_payload,
                                        )
                                        .await;
                                        // Persist decision
                                        save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                                decision_type: "goal_needs_work".into(),
                                                message: msg,
                                                timestamp: chrono::Utc::now(),
                                                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "reason": reason, "suggestions": suggestions }),
                                            }).await;
                                        crate::run_trace::emit_for_scope(
                                            &state_for_consumer.persist.run_tracer,
                                            session_id.as_deref(),
                                            &app_id_for_consumer,
                                            crate::run_trace::phase::PLAN_GOAL_NEEDS_WORK,
                                            "plan_loop",
                                            crate::run_trace::status::INFO,
                                            Some(reason.clone()),
                                            None,
                                            Some(goal_id.to_string()),
                                            None,
                                        )
                                        .await;
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "GoalEvaluator failed, marking complete by default");
                                        crate::run_trace::emit_for_scope(
                                            &state_for_consumer.persist.run_tracer,
                                            session_id.as_deref(),
                                            &app_id_for_consumer,
                                            crate::run_trace::phase::PLAN_GOAL_EVAL_FALLBACK,
                                            "plan_loop",
                                            crate::run_trace::status::ERROR,
                                            Some(e.clone()),
                                            None,
                                            Some(goal_id.to_string()),
                                            None,
                                        )
                                        .await;
                                        let space = macaca_task::TaskSpace::for_session(
                                            app_id_for_consumer.clone(),
                                            None,
                                            Arc::clone(&state_for_consumer.persist.todo_store),
                                        );
                                        space.complete_goal(&goal_id).await;
                                    }
                                }
                            }
                            macaca_task::PlanEvent::GoalCompleted {
                                goal_id,
                                description,
                            } => {
                                tracing::info!(goal_id = %goal_id, "Goal completed: {}", description);

                                // ===== RESUME COORDINATOR IF PAUSED =====
                                let goal_id_str = goal_id.to_string();
                                let mut gts =
                                    state_for_consumer.sessions.goal_to_session.write().await;
                                let trace_sid = gts.get(&goal_id_str).cloned();
                                let waiting_session = gts.remove(&goal_id_str);
                                drop(gts);

                                crate::run_trace::emit_for_scope(
                                    &state_for_consumer.persist.run_tracer,
                                    trace_sid.as_deref(),
                                    &app_id_for_consumer,
                                    crate::run_trace::phase::PLAN_GOAL_COMPLETED,
                                    "plan_loop",
                                    crate::run_trace::status::OK,
                                    Some(description.chars().take(200).collect::<String>()),
                                    None,
                                    Some(goal_id_str.clone()),
                                    None,
                                )
                                .await;

                                if let Some(sid) = waiting_session {
                                    let sessions =
                                        state_for_consumer.sessions.active_sessions.read().await;
                                    if let Some(session) = sessions.get(&sid) {
                                        let resume_reason = crate::runtime_resume::RuntimeResumeSignal::DelegateCompleted {
                                            task_id: goal_id_str.clone(),
                                            success: true,
                                            output: format!("Goal completed: {}", description),
                                        };
                                        session
                                            .pause_signal
                                            .store(false, std::sync::atomic::Ordering::SeqCst);
                                        let _ = session.resume_tx.send(resume_reason).await;

                                        let resumed_payload = serde_json::json!({
                                            "session_id": sid,
                                            "task_id": goal_id_str,
                                            "success": true,
                                            "goal_completed": true,
                                        });
                                        state_for_consumer
                                            .persist
                                            .event_log
                                            .append_command(AppendEventCommand::new(
                                                &sid,
                                                "loop_resumed",
                                                &entry_agent_for_loop,
                                                resumed_payload.clone(),
                                            ))
                                            .await;
                                        let _ = session
                                            .sse_tx
                                            .read()
                                            .await
                                            .send(Ok(Event::default()
                                                .event("loop_resumed")
                                                .data(resumed_payload.to_string())))
                                            .await;

                                        let mut exec_ctx = ExecutionContext::new(
                                            sid.clone(),
                                            app_id_for_consumer.0.to_string(),
                                            plan_agent_name_for_loop.clone(),
                                        );
                                        let _ = load_module_state(
                                            state_for_consumer
                                                .sessions
                                                .framework_session_store
                                                .as_ref(),
                                            &sid,
                                            &mut exec_ctx,
                                        )
                                        .await;
                                        exec_ctx.mark_resumed(Some(format!(
                                            "goal_completed:{}",
                                            goal_id
                                        )));
                                        let _ = save_module_state(
                                            state_for_consumer
                                                .sessions
                                                .framework_session_store
                                                .as_ref(),
                                            &sid,
                                            &exec_ctx,
                                        )
                                        .await;

                                        tracing::info!(goal_id = %goal_id, session_id = %sid, "Resumed coordinator after goal completion");
                                    }
                                } else {
                                    tracing::warn!(
                                        goal_id = %goal_id,
                                        app_id = %app_id_for_consumer,
                                        "Goal completed but no exact paused coordinator mapping was found"
                                    );
                                }

                                // Emit SSE decision event
                                let msg = format!("Goal completed: {}", description);
                                let plan_payload = serde_json::json!({
                                    "decision_type": "goal_completed",
                                    "goal_id": goal_id.to_string(),
                                    "description": description,
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(
                                    &state_for_consumer,
                                    &app_id_for_consumer,
                                    sse_event,
                                    plan_payload,
                                )
                                .await;
                                // Persist decision
                                save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                        decision_type: "goal_completed".into(),
                                        message: msg,
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": description }),
                                    }).await;
                            }
                        }
                    }
                });
                tracing::info!(app_id = %app_id, "PlanLoop started for app");
            }
        }
    }

    // ── WorkerLoops ──
    {
        let already = state
            .loops
            .worker_loop_handles
            .read()
            .await
            .contains_key(app_id);
        if !already {
            if let Some(executor) = state.executor_registry.get(app_id).await {
                let agents = executor.list_agents().await;
                let mut shutdowns: Vec<Arc<std::sync::atomic::AtomicBool>> = Vec::new();
                let mut worker_wakers: Vec<macaca_task::WorkerLoopWaker> = Vec::new();

                for agent_info in &agents {
                    let agent_name = agent_info.name.clone();
                    // Skip the entry agent and plan agent — they don't pull from the TaskBoard.
                    // The entry agent handles user interaction; plan_agent handles decomposition + review.
                    let is_entry = agent_name == entry_agent_name;
                    if is_entry || agent_name == plan_agent_name {
                        continue;
                    }

                    let board = Arc::new(macaca_task::TaskBoard::for_agent(
                        app_id.clone(),
                        agent_name.clone(),
                        session_id.clone(),
                        Arc::clone(&state.persist.todo_store),
                    ));
                    let worker_loop = macaca_task::WorkerLoop::with_components(
                        Arc::clone(&board),
                        macaca_task::WorkerLoopConfig::default(),
                    );
                    worker_wakers.push(worker_loop.waker());
                    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    shutdowns.push(Arc::clone(&shutdown));

                    let (event_tx, mut event_rx) =
                        tokio::sync::mpsc::channel::<macaca_task::WorkerEvent>(32);
                    let shutdown_clone = Arc::clone(&shutdown);
                    tokio::spawn(async move {
                        worker_loop
                            .run_with_default_template(shutdown_clone, event_tx)
                            .await;
                    });

                    let executor_clone = Arc::clone(&executor);
                    let agent_name_clone = agent_name.clone();
                    let board_clone = Arc::clone(&board);
                    let state_for_worker = Arc::clone(state);
                    let app_id_for_worker = app_id.clone();
                    let session_store_for_worker = Arc::clone(&state.persist.session_store);
                    tokio::spawn(async move {
                        while let Some(event) = event_rx.recv().await {
                            match event {
                                macaca_task::WorkerEvent::TaskClaimed {
                                    task_id,
                                    title,
                                    description,
                                    acceptance_criteria,
                                    context,
                                    optimization_suggestions,
                                    session_id: task_session,
                                    ..
                                } => {
                                    let mut prompt = format!(
                                        "Execute this task:\n\nTitle: {}\nDescription: {}",
                                        title, description
                                    );
                                    if !acceptance_criteria.is_empty() {
                                        prompt.push_str(&format!(
                                            "\n\nAcceptance Criteria:\n{}",
                                            acceptance_criteria
                                                .iter()
                                                .map(|c| format!("- {}", c))
                                                .collect::<Vec<_>>()
                                                .join("\n")
                                        ));
                                    }
                                    if let Some(ctx) = context {
                                        prompt.push_str(&format!("\n\nContext: {}", ctx));
                                    }
                                    if let Some(sug) = optimization_suggestions {
                                        prompt.push_str(&format!("\n\nOptimization: {}", sug));
                                    }
                                    tracing::info!(agent = %agent_name_clone, title = %title, "WorkerLoop claimed task");
                                    crate::run_trace::emit_for_scope(
                                        &state_for_worker.persist.run_tracer,
                                        task_session.as_deref(),
                                        &app_id_for_worker,
                                        crate::run_trace::phase::WORKER_TASK_CLAIMED,
                                        "worker_loop",
                                        crate::run_trace::status::INFO,
                                        Some(format!("agent={agent_name_clone}")),
                                        Some(task_id.to_string()),
                                        None,
                                        Some(serde_json::json!({ "title": title })),
                                    )
                                    .await;
                                    // Emit SSE decision event for task claimed
                                    let msg = format!(
                                        "Agent '{}' claimed task: {}",
                                        agent_name_clone, title
                                    );
                                    let plan_payload = serde_json::json!({
                                        "decision_type": "task_claimed",
                                        "task_id": task_id.to_string(),
                                        "agent": agent_name_clone,
                                        "title": title,
                                        "message": msg,
                                    });
                                    let sse_event = Event::default()
                                        .event("plan_decision")
                                        .data(plan_payload.to_string());
                                    broadcast_to_app_sessions(
                                        &state_for_worker,
                                        &app_id_for_worker,
                                        sse_event,
                                        plan_payload,
                                    )
                                    .await;
                                    // Persist decision
                                    save_plan_decision(&session_store_for_worker, &app_id_for_worker, PlanDecisionEvent {
                                            decision_type: "task_claimed".into(),
                                            message: msg,
                                            timestamp: chrono::Utc::now(),
                                            data: serde_json::json!({ "task_id": task_id.to_string(), "agent": agent_name_clone, "title": title }),
                                        }).await;
                                    // delegate_task returns a task_id. We must wait for execution
                                    // to complete, then update the TaskBoard based on the result.
                                    crate::run_trace::emit_for_scope(
                                        &state_for_worker.persist.run_tracer,
                                        task_session.as_deref(),
                                        &app_id_for_worker,
                                        crate::run_trace::phase::WORKER_DELEGATE_START,
                                        "worker_loop",
                                        crate::run_trace::status::INFO,
                                        Some(format!("agent={agent_name_clone}")),
                                        Some(task_id.to_string()),
                                        None,
                                        None,
                                    )
                                    .await;
                                    // Update agent status to Working BEFORE delegation
                                    if let Some(agent_manifest) = state_for_worker
                                        .kernel
                                        .list_agents()
                                        .await
                                        .iter()
                                        .find(|a| a.name == agent_name_clone)
                                    {
                                        state_for_worker
                                            .kernel
                                            .update_agent_activity(
                                                &agent_manifest.id,
                                                macaca_proto::AgentActivity::Working {
                                                    context: format!("Executing: {}", title),
                                                },
                                            )
                                            .await;
                                    }
                                    // Use ReActAgent with executor hooks for trace events
                                    // Emit TaskStarted event for SSE/EventLog
                                    executor_clone.broadcast_event(executor_task_started(
                                        task_id,
                                        &agent_name_clone,
                                    ));
                                    match crate::framework_runner::FrameworkRunner::build_for_intent(
                                        &state_for_worker,
                                        &app_id_for_worker,
                                        &agent_name_clone,
                                        task_session.clone(),
                                        task_id,
                                        Arc::clone(&executor_clone),
                                        macaca_framework::construction::AgentBuildIntent::WorkerTask { task_id },
                                    ).await {
                                        Ok(agent) => {
                                            use macaca_framework::agent::Agent;
                                            let msg = macaca_framework::message::Msg::user("worker_loop", prompt.as_str());
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(30 * 60),
                                                AssertUnwindSafe(agent.reply(msg)).catch_unwind(),
                                            ).await {
                                                Ok(Ok(Ok(reply))) => {
                                                    let output = reply.get_text();
                                                    handle_worker_execution_success(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        &title,
                                                        output,
                                                        WorkerExecutionMode::TaskClaimed,
                                                    )
                                                    .await;
                                                }
                                                Ok(Ok(Err(e))) => {
                                                    let error = e.to_string();
                                                    handle_worker_execution_failure(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        error,
                                                    )
                                                    .await;
                                                    tracing::error!(agent = %agent_name_clone, "Task execution failed: {}", e);
                                                }
                                                Ok(Err(_panic)) => {
                                                    let error = WorkerExecutionMode::TaskClaimed
                                                        .panic_error()
                                                        .to_string();
                                                    handle_worker_execution_failure(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        error,
                                                    )
                                                    .await;
                                                    tracing::error!(agent = %agent_name_clone, task_id = %task_id, "Task execution panicked");
                                                }
                                                Err(_) => {
                                                    tracing::error!(agent = %agent_name_clone, "Task execution timeout after 30min");
                                                    handle_worker_execution_timeout(
                                                        &board_clone,
                                                        &executor_clone,
                                                        task_id,
                                                        &agent_name_clone,
                                                        WorkerExecutionMode::TaskClaimed,
                                                    )
                                                    .await;
                                                }
                                            }
                                            // Reset agent status to Idle
                                            if let Some(agent_manifest) = state_for_worker.kernel.list_agents().await.iter().find(|a| a.name == agent_name_clone) {
                                                state_for_worker.kernel.update_agent_activity(
                                                    &agent_manifest.id,
                                                    macaca_proto::AgentActivity::Idle,
                                                ).await;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(agent = %agent_name_clone, error = %e, "Failed to build agent");
                                            // Emit TaskFailed since TaskStarted was already sent
                                            executor_clone.broadcast_event(executor_task_failed(
                                                task_id,
                                                &agent_name_clone,
                                                e.clone(),
                                            ));
                                            crate::run_trace::emit_for_scope(
                                                &state_for_worker.persist.run_tracer,
                                                task_session.as_deref(),
                                                &app_id_for_worker,
                                                crate::run_trace::phase::WORKER_DELEGATE_ERROR,
                                                "worker_loop",
                                                crate::run_trace::status::ERROR,
                                                Some(e.clone()),
                                                Some(task_id.to_string()),
                                                None,
                                                None,
                                            ).await;
                                            // Reset to Pending for retry
                                            if let Some(mut task) = board_clone.current_task().await {
                                                if task.id == task_id {
                                                    task.status = macaca_proto::TodoStatus::Pending;
                                                    task.updated_at = chrono::Utc::now();
                                                    state_for_worker.persist.todo_store.save_todo(&task).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                macaca_task::WorkerEvent::RetryTask {
                                    task_id,
                                    title,
                                    description,
                                    optimization_suggestions,
                                    session_id: task_session,
                                    ..
                                } => {
                                    crate::run_trace::emit_for_scope(
                                        &state_for_worker.persist.run_tracer,
                                        task_session.as_deref(),
                                        &app_id_for_worker,
                                        crate::run_trace::phase::WORKER_RETRY_START,
                                        "worker_loop",
                                        crate::run_trace::status::INFO,
                                        Some(format!("agent={agent_name_clone}")),
                                        Some(task_id.to_string()),
                                        None,
                                        Some(serde_json::json!({ "title": title })),
                                    )
                                    .await;
                                    let prompt = format!(
                                        "Retry task:\n\nTitle: {}\nDescription: {}\n\nFeedback: {}",
                                        title, description, optimization_suggestions
                                    );
                                    crate::run_trace::emit_for_scope(
                                        &state_for_worker.persist.run_tracer,
                                        task_session.as_deref(),
                                        &app_id_for_worker,
                                        crate::run_trace::phase::WORKER_DELEGATE_START,
                                        "worker_loop",
                                        crate::run_trace::status::INFO,
                                        Some("retry".into()),
                                        Some(task_id.to_string()),
                                        None,
                                        None,
                                    )
                                    .await;
                                    // Use ReActAgent with executor hooks for trace events (retry)
                                    // Emit TaskStarted event for SSE/EventLog (retry)
                                    executor_clone.broadcast_event(executor_task_started(
                                        task_id,
                                        &agent_name_clone,
                                    ));
                                    match crate::framework_runner::FrameworkRunner::build_for_intent(
                                        &state_for_worker,
                                        &app_id_for_worker,
                                        &agent_name_clone,
                                        task_session.clone(),
                                        task_id,
                                        Arc::clone(&executor_clone),
                                        macaca_framework::construction::AgentBuildIntent::WorkerTask { task_id },
                                    ).await {
                                        Ok(agent) => {
                                            use macaca_framework::agent::Agent;
                                            let msg = macaca_framework::message::Msg::user("worker_loop", prompt.as_str());
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(30 * 60),
                                                AssertUnwindSafe(agent.reply(msg)).catch_unwind(),
                                            ).await {
                                                Ok(Ok(Ok(reply))) => {
                                                    let output = reply.get_text();
                                                    handle_worker_execution_success(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        &title,
                                                        output,
                                                        WorkerExecutionMode::Retry,
                                                    )
                                                    .await;
                                                }
                                                Ok(Ok(Err(e))) => {
                                                    let error = e.to_string();
                                                    handle_worker_execution_failure(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        error,
                                                    )
                                                    .await;
                                                }
                                                Ok(Err(_panic)) => {
                                                    let error = WorkerExecutionMode::Retry
                                                        .panic_error()
                                                        .to_string();
                                                    handle_worker_execution_failure(
                                                        &state_for_worker,
                                                        &board_clone,
                                                        &executor_clone,
                                                        &app_id_for_worker,
                                                        task_session.as_deref(),
                                                        task_id,
                                                        &agent_name_clone,
                                                        error,
                                                    )
                                                    .await;
                                                }
                                                Err(_) => {
                                                    handle_worker_execution_timeout(
                                                        &board_clone,
                                                        &executor_clone,
                                                        task_id,
                                                        &agent_name_clone,
                                                        WorkerExecutionMode::Retry,
                                                    )
                                                    .await;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            crate::run_trace::emit_for_scope(
                                                &state_for_worker.persist.run_tracer,
                                                task_session.as_deref(),
                                                &app_id_for_worker,
                                                crate::run_trace::phase::WORKER_DELEGATE_ERROR,
                                                "worker_loop",
                                                crate::run_trace::status::ERROR,
                                                Some(e.clone()),
                                                Some(task_id.to_string()),
                                                None,
                                                None,
                                            ).await;
                                            tracing::warn!(agent = %agent_name_clone, error = %e, "Retry build_agent failed, resetting task to Pending");
                                            if let Some(mut task) = board_clone.current_task().await {
                                                if task.id == task_id {
                                                    task.status = macaca_proto::TodoStatus::Pending;
                                                    task.updated_at = chrono::Utc::now();
                                                    state_for_worker.persist.todo_store.save_todo(&task).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                macaca_task::WorkerEvent::Idle => {}
                            }
                        }
                    });
                }
                state
                    .loops
                    .worker_loop_handles
                    .write()
                    .await
                    .insert(app_id.clone(), shutdowns);
                state
                    .loops
                    .worker_loop_wakers
                    .write()
                    .await
                    .insert(app_id.clone(), worker_wakers);
                tracing::info!(app_id = %app_id, "WorkerLoops started for app");
            }
        }
    }
}

/// POST /api/apps/{app_id}/goals — create a new goal
pub(crate) async fn create_goal(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let description = body["description"].as_str().ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "Missing 'description' field".into(),
        )
    })?;
    let store = Arc::clone(&state.persist.todo_store);
    let session_id = body["session_id"].as_str().map(|s| s.to_string());
    let space =
        macaca_task::TaskSpace::for_session(app_id.clone(), session_id.clone(), Arc::clone(&store));
    let goal = space.push_goal(description).await;

    crate::run_trace::emit_for_scope(
        &state.persist.run_tracer,
        goal.session_id.as_deref(),
        &app_id,
        crate::run_trace::phase::GOAL_CREATE_HTTP,
        "api.create_goal",
        crate::run_trace::status::OK,
        Some(description.chars().take(160).collect::<String>()),
        None,
        Some(goal.id.to_string()),
        None,
    )
    .await;

    // Start PlanLoop + WorkerLoops if not already running
    ensure_plan_and_worker_loops(&state, &app_id, session_id).await;

    Ok(Json(
        serde_json::json!({ "goal_id": goal.id.to_string(), "status": "pending" }),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        executor_task_completed, executor_task_failed, executor_task_started,
        goal_has_decomposed_tasks, mark_decomposition_in_notebook, mark_review_in_notebook,
        planner_scope_session_id, select_entry_and_plan_agents, worker_success_summary,
        PlannerFrameworkCallKind, WorkerExecutionMode,
    };
    use macaca_framework::construction::AgentBuildIntent;
    use macaca_framework::plan::{PlanNotebook, PlanState, SubTaskState};
    use macaca_kernel::executor::ExecutorEvent;
    use macaca_kernel::AgentInfo;

    fn agent(name: &str, capabilities: &[&str]) -> AgentInfo {
        AgentInfo {
            id: format!("id-{name}"),
            name: name.to_string(),
            capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
            current_load: 0,
            max_load: 4,
            available: true,
        }
    }

    #[test]
    fn planner_selection_is_capability_driven_not_name_driven() {
        let agents = vec![
            agent("orchestrator", &["todo_goal_management"]),
            agent("decomposer", &["task_planning"]),
            agent("executor_a", &["todo_execution"]),
        ];
        let (entry, planner) = select_entry_and_plan_agents(&agents, Some("orchestrator"));
        assert_eq!(entry, "orchestrator");
        assert_eq!(planner, "decomposer");
    }

    #[test]
    fn planner_falls_back_to_entry_when_no_planning_capability() {
        let agents = vec![
            agent("entry_custom", &["todo_goal_management"]),
            agent("worker_custom", &["todo_execution"]),
        ];
        let (entry, planner) = select_entry_and_plan_agents(&agents, Some("entry_custom"));
        assert_eq!(entry, "entry_custom");
        assert_eq!(planner, "entry_custom");
    }

    #[test]
    fn planner_scope_session_id_preserves_session_and_app_fallback() {
        let app_id = macaca_proto::ApplicationId::from_name("demo-app");

        assert_eq!(
            planner_scope_session_id(&app_id, Some("session-123")),
            "session-123"
        );
        assert_eq!(
            planner_scope_session_id(&app_id, None),
            format!("_macaca_app_{}", app_id.0)
        );
    }

    #[test]
    fn goal_has_decomposed_tasks_detects_existing_parent_task() {
        let app_id = macaca_proto::ApplicationId::from_name("demo-app");
        let goal_id = macaca_proto::TaskId::new();
        let other_goal_id = macaca_proto::TaskId::new();
        let mut task = macaca_proto::TodoItem::new(
            app_id,
            Some("session-123".into()),
            "architect",
            "planner",
            "Design architecture",
            "Define architecture and API contracts",
            9,
        );
        task.parent_task = Some(goal_id);

        assert!(goal_has_decomposed_tasks(&[task.clone()], goal_id));
        assert!(!goal_has_decomposed_tasks(&[task], other_goal_id));
        assert!(!goal_has_decomposed_tasks(&[], goal_id));
    }

    #[test]
    fn planner_notebook_decomposition_content_is_preserved() {
        let goal_id = macaca_proto::TaskId::new();
        let description = "Build a small app";
        let mut notebook = PlanNotebook::new();

        mark_decomposition_in_notebook(&mut notebook, goal_id, description);

        assert!(notebook.current_plan().is_none());
        assert_eq!(notebook.historical_plans().len(), 1);
        let plan = &notebook.historical_plans()[0];
        assert_eq!(plan.name, format!("goal:{}", goal_id));
        assert_eq!(plan.description, description);
        assert_eq!(
            plan.expected_outcome,
            "Decompose goal into executable todos"
        );
        assert_eq!(plan.state, PlanState::Done);
        assert_eq!(
            plan.outcome.as_deref(),
            Some(format!("goal {} decomposition recorded", goal_id).as_str())
        );
        assert_eq!(plan.subtasks.len(), 1);
        let subtask = &plan.subtasks[0];
        assert_eq!(subtask.name, "decompose_goal");
        assert_eq!(subtask.description, format!("Decompose goal {}", goal_id));
        assert_eq!(
            subtask.expected_outcome,
            "Todos created and persisted to TodoBoard"
        );
        assert_eq!(subtask.state, SubTaskState::Done);
        assert_eq!(
            subtask.outcome.as_deref(),
            Some("decomposition delegated to planner")
        );
    }

    #[test]
    fn planner_notebook_review_content_is_preserved() {
        let task_id = macaca_proto::TaskId::new();
        let task_title = "Implement API";
        let mut notebook = PlanNotebook::new();

        mark_review_in_notebook(&mut notebook, task_id, task_title);

        assert!(notebook.current_plan().is_none());
        assert_eq!(notebook.historical_plans().len(), 1);
        let plan = &notebook.historical_plans()[0];
        assert_eq!(plan.name, format!("review:{}", task_id));
        assert_eq!(plan.description, format!("Review task '{}'", task_title));
        assert_eq!(
            plan.expected_outcome,
            "Task review decision persisted via review_todo"
        );
        assert_eq!(plan.state, PlanState::Done);
        assert_eq!(
            plan.outcome.as_deref(),
            Some(format!("task {} review recorded", task_id).as_str())
        );
        assert_eq!(plan.subtasks.len(), 1);
        let subtask = &plan.subtasks[0];
        assert_eq!(subtask.name, "review_todo");
        assert_eq!(subtask.description, format!("Review todo {}", task_id));
        assert_eq!(
            subtask.expected_outcome,
            "Todo status updated to completed/needs_optimization/failed"
        );
        assert_eq!(subtask.state, SubTaskState::Done);
        assert_eq!(
            subtask.outcome.as_deref(),
            Some("review delegated to planner")
        );
    }

    #[test]
    fn executor_task_started_helper_preserves_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_started(task_id, "planner");

        match event {
            ExecutorEvent::TaskStarted {
                task_id: got,
                agent,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "planner");
            }
            other => panic!("expected TaskStarted, got {other:?}"),
        }
    }

    #[test]
    fn executor_task_completed_helper_preserves_result_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_completed(task_id, "backend", "done");

        match event {
            ExecutorEvent::TaskCompleted {
                task_id: got,
                agent,
                result,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "backend");
                assert_eq!(result.task_id, task_id);
                assert!(result.success);
                assert_eq!(result.output, "done");
                assert_eq!(result.error, None);
                assert!(result.artifacts.is_empty());
                assert!(result.tokens_used.is_none());
                assert!(result.completed_at <= chrono::Utc::now());
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn executor_task_failed_helper_preserves_fields() {
        let task_id = macaca_proto::TaskId::new();

        let event = executor_task_failed(task_id, "frontend", "boom");

        match event {
            ExecutorEvent::TaskFailed {
                task_id: got,
                agent,
                error,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "frontend");
                assert_eq!(error, "boom");
            }
            other => panic!("expected TaskFailed, got {other:?}"),
        }
    }

    #[test]
    fn planner_framework_call_uses_goal_scoped_intents() {
        let goal_id = macaca_proto::TaskId::new();

        assert_eq!(
            AgentBuildIntent::PlannerDecomposition {
                goal_id: PlannerFrameworkCallKind::DecomposeGoal.goal_context(goal_id),
            },
            AgentBuildIntent::PlannerDecomposition {
                goal_id: Some(goal_id)
            }
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.goal_context(goal_id),
            Some(goal_id)
        );
        assert_eq!(
            AgentBuildIntent::PlannerFollowUp {
                goal_id: PlannerFrameworkCallKind::FollowUp.goal_context(goal_id),
            },
            AgentBuildIntent::PlannerFollowUp {
                goal_id: Some(goal_id)
            }
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.goal_context(goal_id),
            Some(goal_id)
        );
        assert_eq!(
            AgentBuildIntent::GoalEvaluation {
                goal_id: PlannerFrameworkCallKind::GoalEvaluation.goal_context(goal_id),
            },
            AgentBuildIntent::GoalEvaluation {
                goal_id: Some(goal_id)
            }
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.goal_context(goal_id),
            Some(goal_id)
        );
    }

    #[test]
    fn planner_framework_call_uses_review_intent_for_review() {
        let task_id = macaca_proto::TaskId::new();
        let intent = match PlannerFrameworkCallKind::Review {
            PlannerFrameworkCallKind::DecomposeGoal => AgentBuildIntent::PlannerDecomposition {
                goal_id: Some(task_id),
            },
            PlannerFrameworkCallKind::Review => AgentBuildIntent::PlannerReview { task_id },
            PlannerFrameworkCallKind::FollowUp => AgentBuildIntent::PlannerFollowUp {
                goal_id: Some(task_id),
            },
            PlannerFrameworkCallKind::GoalEvaluation => AgentBuildIntent::GoalEvaluation {
                goal_id: Some(task_id),
            },
        };

        assert_eq!(intent, AgentBuildIntent::PlannerReview { task_id });
        assert_eq!(PlannerFrameworkCallKind::Review.goal_context(task_id), None);
    }

    #[test]
    fn planner_framework_call_messages_preserve_existing_log_text() {
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.success_log_prefix(),
            "Planner decomposition completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.reply_error_log_prefix(),
            "Planner decomposition failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.build_error_log_prefix(),
            "Failed to build planner agent"
        );
        assert_eq!(
            PlannerFrameworkCallKind::DecomposeGoal.missing_executor_log(),
            "No executor found for planner decomposition"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.success_log_prefix(),
            "Review completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.reply_error_log_prefix(),
            "Review failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.build_error_log_prefix(),
            "Failed to build planner agent for review"
        );
        assert_eq!(
            PlannerFrameworkCallKind::Review.missing_executor_log(),
            "No executor found for planner review"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.success_log_prefix(),
            "Follow-up tasks created"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.reply_error_log_prefix(),
            "Follow-up task creation failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.build_error_log_prefix(),
            "Failed to build planner agent for follow-up"
        );
        assert_eq!(
            PlannerFrameworkCallKind::FollowUp.missing_executor_log(),
            "No executor found for planner follow-up"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.success_log_prefix(),
            "Goal evaluation completed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.reply_error_log_prefix(),
            "Goal evaluation failed"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.build_error_log_prefix(),
            "Failed to build planner agent for goal evaluation"
        );
        assert_eq!(
            PlannerFrameworkCallKind::GoalEvaluation.missing_executor_log(),
            "No executor found for planner goal evaluation"
        );
    }

    #[test]
    fn worker_success_summary_preserves_normal_empty_output_fallback() {
        let summary = worker_success_summary(
            WorkerExecutionMode::TaskClaimed,
            "Implement API",
            String::new(),
        );

        assert_eq!(summary, "Task 'Implement API' completed");
    }

    #[test]
    fn worker_success_summary_preserves_retry_empty_output_fallback() {
        let summary =
            worker_success_summary(WorkerExecutionMode::Retry, "Implement API", String::new());

        assert_eq!(summary, "Task 'Implement API' completed on retry");
    }

    #[test]
    fn worker_success_summary_preserves_non_empty_output() {
        let summary = worker_success_summary(
            WorkerExecutionMode::TaskClaimed,
            "Implement API",
            "custom summary".to_string(),
        );

        assert_eq!(summary, "custom summary");
    }

    #[test]
    fn worker_execution_mode_preserves_trace_detail_and_error_messages() {
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.success_submit_review_detail("abcdef"),
            "abcdef"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.success_submit_review_detail("abcdef"),
            "retry_success"
        );
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.panic_error(),
            "Task execution panicked"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.panic_error(),
            "Retry task execution panicked"
        );
        assert_eq!(
            WorkerExecutionMode::TaskClaimed.timeout_error(),
            "Execution timeout (30 min)"
        );
        assert_eq!(
            WorkerExecutionMode::Retry.timeout_error(),
            "Retry execution timeout (30 min)"
        );
    }
}
