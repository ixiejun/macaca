//! WorkerLoop startup and WorkerEvent consumer (task-event adapter).
//!
//! Spawns per-agent WorkerLoops and routes claimed/retry tasks through the
//! worker execution adapter (`service.agent_execution`).

use std::sync::Arc;

use axum::response::sse::Event;
use macaca_proto::ApplicationId;

use super::execution_control_adapter::session_loop_coordinator;
use crate::session_loop_shell_adapter::register_worker_loops_via_execution_control;
use crate::sse::{broadcast_to_app_sessions, save_plan_decision, PlanDecisionEvent};
use crate::state::AppState;
use super::worker_execution_adapter::{execute_worker_task_via_agent_service, WorkerExecutionMode};


/// Start WorkerLoops for worker agents when not already running (idempotent).
pub(crate) async fn ensure_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &Option<String>,
    entry_agent_name: &str,
    plan_agent_name: &str,
) {
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
                                    execute_worker_task_via_agent_service(
                                        &state_for_worker,
                                        &board_clone,
                                        &executor_clone,
                                        &app_id_for_worker,
                                        task_session.as_deref(),
                                        task_id,
                                        &agent_name_clone,
                                        &title,
                                        prompt,
                                        WorkerExecutionMode::TaskClaimed,
                                    )
                                    .await;
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
                                    execute_worker_task_via_agent_service(
                                        &state_for_worker,
                                        &board_clone,
                                        &executor_clone,
                                        &app_id_for_worker,
                                        task_session.as_deref(),
                                        task_id,
                                        &agent_name_clone,
                                        &title,
                                        prompt,
                                        WorkerExecutionMode::Retry,
                                    )
                                    .await;
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
                let worker_count = worker_wakers.len();
                state
                    .loops
                    .worker_loop_wakers
                    .write()
                    .await
                    .insert(app_id.clone(), worker_wakers);

                // Mirror worker-loop registration into execution control so wake/shutdown
                // paths share the same auditable service boundary as PlanLoop lifecycle.
                let coordinator = session_loop_coordinator(state);
                register_worker_loops_via_execution_control(
                    &coordinator,
                    app_id.clone(),
                    session_id.clone(),
                    worker_count,
                )
                .await;

                tracing::info!(app_id = %app_id, worker_count, "WorkerLoops started for app");
            }
        }
    }
}
