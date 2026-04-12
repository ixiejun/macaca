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

use macaca_framework::execution::ExecutionContext;
use macaca_framework::plan::PlanNotebook;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_kernel::executor::{ApplicationExecutor, ExecutorEvent, TaskResult};
use macaca_kernel::AgentInfo;
use macaca_proto::ApplicationId;

use crate::routes::{err, ErrorResponse};
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};
use crate::state::AppState;

fn planner_scope_session_id(app_id: &ApplicationId, session_id: Option<&str>) -> String {
    session_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("_macaca_app_{}", app_id.0))
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

async fn planner_notebook_mark_decomposition(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    goal_id: macaca_proto::TaskId,
    description: &str,
) {
    let sid = planner_scope_session_id(app_id, session_id);
    let mut notebook = PlanNotebook::new();
    let _ = load_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &mut notebook,
    )
    .await;

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
    let _ = save_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &notebook,
    )
    .await;
}

async fn planner_notebook_mark_review(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
    task_id: macaca_proto::TaskId,
    task_title: &str,
) {
    let sid = planner_scope_session_id(app_id, session_id);
    let mut notebook = PlanNotebook::new();
    let _ = load_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &mut notebook,
    )
    .await;
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
    let _ = save_module_state(
        state.sessions.framework_session_store.as_ref(),
        &sid,
        &notebook,
    )
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
    board.submit_for_review(&task_id, summary.clone()).await;
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
    board.mark_failed(&task_id, error.clone()).await;
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
    board.mark_failed(&task_id, error.into()).await;
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
            .and_then(|a| a.manifest.entry_agent.clone())
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
                let plan_space = Arc::new(macaca_task::TaskSpace::new(
                    app_id.clone(),
                    session_id.clone(),
                    Arc::clone(&store),
                ));
                let plan_loop =
                    macaca_task::PlanLoop::new(plan_space, macaca_task::PlanLoopConfig::default());
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
                    plan_loop.run(shutdown, event_tx).await;
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
                                    let (agent_names, agent_profiles): (Vec<String>, Vec<String>) =
                                        if let Some(executor) = state_for_consumer
                                            .executor_registry
                                            .get(&app_id_for_consumer)
                                            .await
                                        {
                                            let agents = executor.list_agents().await;
                                            let workers: Vec<_> = agents
                                                .iter()
                                                .filter(|a| {
                                                    a.name != entry_agent_for_loop
                                                        && a.name != plan_agent_name_for_loop
                                                })
                                                .collect();
                                            let names =
                                                workers.iter().map(|a| a.name.clone()).collect();
                                            let profiles = workers
                                                .iter()
                                                .map(|a| {
                                                    let caps = if a.capabilities.is_empty() {
                                                        "no capability metadata".to_string()
                                                    } else {
                                                        a.capabilities.join(", ")
                                                    };
                                                    format!("- {}: {}", a.name, caps)
                                                })
                                                .collect();
                                            (names, profiles)
                                        } else {
                                            (vec![], vec![])
                                        };
                                    let agents_list = if agent_names.is_empty() {
                                        "no worker agents registered".to_string()
                                    } else {
                                        agent_names.join(", ")
                                    };
                                    let agents_profile_text = if agent_profiles.is_empty() {
                                        "(none)".to_string()
                                    } else {
                                        agent_profiles.join("\n")
                                    };
                                    let prompt = format!(
                                        "A new project goal has been submitted. You MUST decompose it into \
                                         concrete tasks by calling the `create_todo` tool for EACH task.\n\n\
                                         Goal: {}\n\n\
                                         Available agents: {}.\n\
                                         Agent capability profiles:\n{}\n\n\
                                         INSTRUCTIONS:\n\
                                         1. Briefly analyze the goal (1-2 sentences).\n\
                                         2. Call `create_todo` for EACH task — you MUST create at least one task.\n\
                                         3. Set priorities (0-10) and acceptance_criteria for each task.\n\
                                         4. Assign each task to the MOST CAPABLE agent based on the capability profiles above.\n\
                                         5. Create foundational design/spec tasks first, then implementation tasks.\n\
                                         6. Use dependencies explicitly:\n\
                                            - depends_on: known task IDs\n\
                                            - depends_on_titles: task titles already created in this decomposition\n\
                                            - depends_on_agents: symbolic cross-agent dependencies (all_tasks/specific_task)\n\
                                         7. Do NOT just describe tasks in text — you MUST invoke the `create_todo` tool.\n\n\
                                         Start by creating the first task now.",
                                        description, agents_list, agents_profile_text
                                    );
                                    if let Some(executor) = state_for_consumer
                                        .executor_registry
                                        .get(&app_id_for_consumer)
                                        .await
                                    {
                                        update_agent_activity_by_name(
                                            &state_for_consumer,
                                            &plan_agent_name_for_loop,
                                            macaca_proto::AgentActivity::Working {
                                                context: format!(
                                                    "Decomposing goal: {}",
                                                    description
                                                        .chars()
                                                        .take(80)
                                                        .collect::<String>()
                                                ),
                                            },
                                        )
                                        .await;
                                        executor.broadcast_event(executor_task_started(
                                            goal_id,
                                            &plan_agent_name_for_loop,
                                        ));
                                        match crate::framework_runner::FrameworkRunner::build_traced_agent_with_goal(
                                            &state_for_consumer,
                                            &app_id_for_consumer,
                                            &plan_agent_name_for_loop,
                                            session_id.clone(),
                                            goal_id,
                                            Arc::clone(&executor),
                                            Some(goal_id),
                                        ).await {
                                            Ok(agent) => {
                                                use macaca_framework::agent::Agent;
                                                let msg = macaca_framework::message::Msg::user("plan_loop", prompt.as_str());
                                                match agent.reply(msg).await {
                                                    Ok(reply) => {
                                                        let output = reply.get_text();
                                                        executor.broadcast_event(
                                                            executor_task_completed(
                                                                goal_id,
                                                                &plan_agent_name_for_loop,
                                                                output.clone(),
                                                            ),
                                                        );
                                                        tracing::info!("Planner decomposition completed: {}", output.chars().take(100).collect::<String>());
                                                    }
                                                    Err(e) => {
                                                        executor.broadcast_event(
                                                            executor_task_failed(
                                                                goal_id,
                                                                &plan_agent_name_for_loop,
                                                                e.to_string(),
                                                            ),
                                                        );
                                                        tracing::error!("Planner decomposition failed: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                executor.broadcast_event(executor_task_failed(
                                                    goal_id,
                                                    &plan_agent_name_for_loop,
                                                    e.clone(),
                                                ));
                                                tracing::error!("Failed to build planner agent: {}", e);
                                            }
                                        }
                                        update_agent_activity_by_name(
                                            &state_for_consumer,
                                            &plan_agent_name_for_loop,
                                            macaca_proto::AgentActivity::Idle,
                                        )
                                        .await;
                                    } else {
                                        tracing::error!(
                                            "No executor found for planner decomposition"
                                        );
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
                                    // Note: don't wake workers here — planner hasn't created tasks yet.
                                    // Workers will be woken when tasks actually appear on their boards.
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
                                    if let Some(executor) = state_for_consumer
                                        .executor_registry
                                        .get(&app_id_for_consumer)
                                        .await
                                    {
                                        update_agent_activity_by_name(
                                            &state_for_consumer,
                                            &plan_agent_name_for_loop,
                                            macaca_proto::AgentActivity::Working {
                                                context: format!(
                                                    "Reviewing {} task: {}",
                                                    agent, title
                                                ),
                                            },
                                        )
                                        .await;
                                        executor.broadcast_event(executor_task_started(
                                            task_id,
                                            &plan_agent_name_for_loop,
                                        ));
                                        match crate::framework_runner::FrameworkRunner::build_worker_agent(
                                            &state_for_consumer,
                                            &app_id_for_consumer,
                                            &plan_agent_name_for_loop,
                                            session_id.clone(),
                                            task_id,
                                            Arc::clone(&executor),
                                        ).await {
                                            Ok(agent) => {
                                                use macaca_framework::agent::Agent;
                                                let msg = macaca_framework::message::Msg::user("plan_loop", prompt.as_str());
                                                match agent.reply(msg).await {
                                                    Ok(reply) => {
                                                        let output = reply.get_text();
                                                        executor.broadcast_event(
                                                            executor_task_completed(
                                                                task_id,
                                                                &plan_agent_name_for_loop,
                                                                output.clone(),
                                                            ),
                                                        );
                                                        tracing::info!("Review completed: {}", output.chars().take(100).collect::<String>());
                                                    }
                                                    Err(e) => {
                                                        executor.broadcast_event(
                                                            executor_task_failed(
                                                                task_id,
                                                                &plan_agent_name_for_loop,
                                                                e.to_string(),
                                                            ),
                                                        );
                                                        tracing::error!("Review failed: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                executor.broadcast_event(executor_task_failed(
                                                    task_id,
                                                    &plan_agent_name_for_loop,
                                                    e.clone(),
                                                ));
                                                tracing::error!("Failed to build planner agent for review: {}", e);
                                            }
                                        }
                                        update_agent_activity_by_name(
                                            &state_for_consumer,
                                            &plan_agent_name_for_loop,
                                            macaca_proto::AgentActivity::Idle,
                                        )
                                        .await;
                                    } else {
                                        tracing::error!("No executor found for planner review");
                                    }
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
                                let evaluator = macaca_task::GoalEvaluator::new(
                                    Arc::clone(&state_for_consumer.llm),
                                    state_for_consumer.config.default_model.clone(),
                                );
                                match evaluator
                                    .evaluate(
                                        &goal_description,
                                        &task_summaries,
                                        completed_count,
                                        failed_count,
                                    )
                                    .await
                                {
                                    Ok(macaca_task::GoalEvaluation::Satisfied { summary }) => {
                                        tracing::info!(goal_id = %goal_id, summary = %summary, "Goal satisfied");
                                        let space = macaca_task::TaskSpace::new(
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
                                            if let Some(executor) = state_for_consumer
                                                .executor_registry
                                                .get(&app_id_for_consumer)
                                                .await
                                            {
                                                update_agent_activity_by_name(
                                                    &state_for_consumer,
                                                    &plan_agent_name_for_loop,
                                                    macaca_proto::AgentActivity::Working {
                                                        context: format!(
                                                            "Planning follow-up work: {}",
                                                            reason
                                                                .chars()
                                                                .take(80)
                                                                .collect::<String>()
                                                        ),
                                                    },
                                                )
                                                .await;
                                                executor.broadcast_event(executor_task_started(
                                                    goal_id,
                                                    &plan_agent_name_for_loop,
                                                ));
                                                match crate::framework_runner::FrameworkRunner::build_traced_agent_with_goal(
                                                    &state_for_consumer,
                                                    &app_id_for_consumer,
                                                    &plan_agent_name_for_loop,
                                                    session_id.clone(),
                                                    goal_id,
                                                    Arc::clone(&executor),
                                                    Some(goal_id),
                                                ).await {
                                                    Ok(agent) => {
                                                        use macaca_framework::agent::Agent;
                                                        let msg = macaca_framework::message::Msg::user("plan_loop", prompt.as_str());
                                                        match agent.reply(msg).await {
                                                            Ok(reply) => {
                                                                let output = reply.get_text();
                                                                executor.broadcast_event(
                                                                    executor_task_completed(
                                                                        goal_id,
                                                                        &plan_agent_name_for_loop,
                                                                        output.clone(),
                                                                    ),
                                                                );
                                                                tracing::info!("Follow-up tasks created: {}", output.chars().take(100).collect::<String>());
                                                            }
                                                            Err(e) => {
                                                                executor.broadcast_event(
                                                                    executor_task_failed(
                                                                        goal_id,
                                                                        &plan_agent_name_for_loop,
                                                                        e.to_string(),
                                                                    ),
                                                                );
                                                                tracing::error!("Follow-up task creation failed: {}", e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        executor.broadcast_event(
                                                            executor_task_failed(
                                                                goal_id,
                                                                &plan_agent_name_for_loop,
                                                                e.clone(),
                                                            ),
                                                        );
                                                        tracing::error!("Failed to build planner agent for follow-up: {}", e);
                                                    }
                                                }
                                                update_agent_activity_by_name(
                                                    &state_for_consumer,
                                                    &plan_agent_name_for_loop,
                                                    macaca_proto::AgentActivity::Idle,
                                                )
                                                .await;
                                            } else {
                                                tracing::error!(
                                                    "No executor found for planner follow-up"
                                                );
                                            }
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
                                        let space = macaca_task::TaskSpace::new(
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
                                        let resume_reason = macaca_runtime::agentic_loop::ResumeReason::DelegateCompleted {
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
                                            .append(
                                                &sid,
                                                "loop_resumed",
                                                &entry_agent_for_loop,
                                                resumed_payload.clone(),
                                            )
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

                    let board = Arc::new(macaca_task::TaskBoard::new(
                        app_id.clone(),
                        agent_name.clone(),
                        session_id.clone(),
                        Arc::clone(&state.persist.todo_store),
                    ));
                    let worker_loop = macaca_task::WorkerLoop::new(
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
                        worker_loop.run(shutdown_clone, event_tx).await;
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
                                    match crate::framework_runner::FrameworkRunner::build_worker_agent(
                                        &state_for_worker, &app_id_for_worker, &agent_name_clone, task_session.clone(),
                                        task_id, Arc::clone(&executor_clone),
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
                                    match crate::framework_runner::FrameworkRunner::build_worker_agent(
                                        &state_for_worker, &app_id_for_worker, &agent_name_clone, task_session.clone(),
                                        task_id, Arc::clone(&executor_clone),
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
    let space = macaca_task::TaskSpace::new(app_id.clone(), session_id.clone(), Arc::clone(&store));
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
        select_entry_and_plan_agents, worker_success_summary, WorkerExecutionMode,
    };
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
