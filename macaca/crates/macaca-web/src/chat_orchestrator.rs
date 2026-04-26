//! Chat orchestration: SSE streaming, agentic loop, workflow execution,
//! and process lifecycle (start/stop).

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;

use macaca_app::AppLoader;
use macaca_framework::execution::ExecutionContext;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_kernel::AgentInfo;
use macaca_persist::PersistStore;
use macaca_proto::ApplicationId;

use crate::routes::{default_model, err, ErrorResponse};
use crate::session::{SessionMeta, StoredSession, StoredTurn, APP_SESSIONS_PREFIX, SESSION_PREFIX};
use crate::sse::convert_executor_event_to_sse;
use crate::state::AppState;

async fn persist_execution_context(state: &Arc<AppState>, context: &ExecutionContext) {
    let _ = save_module_state(
        state.sessions.framework_session_store.as_ref(),
        &context.session_id,
        context,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Request/Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct ChatRequest {
    pub app_id: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// Optional session_id for continuing a conversation, or null for new session
    #[serde(default)]
    pub session_id: Option<String>,
    /// Execution engine: "legacy" (default) or "framework" (ReActAgent-based).
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct StopRequest {
    pub app_id: String,
}

// ---------------------------------------------------------------------------
// POST /api/chat/stop
// ---------------------------------------------------------------------------

/// Shared cleanup logic: stop loops, shutdown executor, reset agents, cancel tasks/goals.
///
/// Used by both `post_chat_stop` (explicit user stop) and `post_chat_v2` (new session creation).
async fn cleanup_app_state(state: &Arc<AppState>, app_id: &ApplicationId) {
    // 1. Unregister executor entirely so the next session can rebuild a fresh worker.
    let _ = state.executor_registry.unregister(app_id).await;

    // 2. Force all agent activities to Idle immediately
    {
        let agents = state.kernel.list_agents().await;
        for agent in &agents {
            state
                .kernel
                .update_agent_activity(&agent.id, macaca_proto::AgentActivity::Idle)
                .await;
        }
    }

    // 3. Cancel ALL non-terminal tasks
    {
        let all_todos = state.persist.todo_store.list_all_todos(app_id).await;
        for mut task in all_todos {
            if !matches!(
                task.status,
                macaca_proto::TodoStatus::Completed
                    | macaca_proto::TodoStatus::Cancelled
                    | macaca_proto::TodoStatus::Failed
            ) {
                task.status = macaca_proto::TodoStatus::Cancelled;
                task.updated_at = chrono::Utc::now();
                state.persist.todo_store.save_todo(&task).await;
            }
        }
        // Also cancel all non-terminal goals to prevent PlanLoop from re-decomposing old goals
        let goals = state.persist.todo_store.list_goals(app_id).await;
        let mut cancelled_goals = 0u32;
        for mut goal in goals {
            if !matches!(
                goal.status,
                macaca_proto::TodoGoalStatus::Completed
                    | macaca_proto::TodoGoalStatus::Cancelled
                    | macaca_proto::TodoGoalStatus::Failed
            ) {
                goal.status = macaca_proto::TodoGoalStatus::Cancelled;
                state.persist.todo_store.save_goal(&goal).await;
                cancelled_goals += 1;
            }
        }
        if cancelled_goals > 0 {
            tracing::info!(app_id = %app_id, cancelled_goals, "Cancelled non-terminal goals during cleanup");
        }
    }

    // 4. Signal PlanLoop shutdown and REMOVE handle so it can be restarted
    {
        let mut handles = state.loops.plan_loop_handles.write().await;
        if let Some(flag) = handles.remove(app_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    {
        let mut wakers = state.loops.plan_loop_wakers.write().await;
        wakers.remove(app_id);
    }

    // 5. Signal all WorkerLoop shutdowns and REMOVE handles
    {
        let mut handles = state.loops.worker_loop_handles.write().await;
        if let Some(flags) = handles.remove(app_id) {
            for flag in &flags {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
    {
        let mut wakers = state.loops.worker_loop_wakers.write().await;
        wakers.remove(app_id);
    }

    tracing::info!(app_id = %app_id, "Application state cleaned up: loops stopped, agents idle, tasks cancelled");
}

async fn ensure_app_executor(state: &Arc<AppState>, app_id: &ApplicationId) {
    if state.executor_registry.get(app_id).await.is_some() {
        return;
    }

    let (app_name, app_agent_names) = {
        let registry = state.registry.read().await;
        if let Some(app) = registry.get_app(app_id).cloned() {
            let names = AppLoader::resolve_agent_configs(&app.manifest, &app.path)
                .map(|configs| configs.into_iter().map(|config| config.name).collect())
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        app_id = %app_id,
                        error = %error,
                        "Failed to resolve app agent names while ensuring executor"
                    );
                    Vec::new()
                });
            (app.name, names)
        } else {
            (app_id.0.to_string(), Vec::new())
        }
    };

    let all_agents = state.kernel.list_agents().await;
    let app_agents: Vec<AgentInfo> = all_agents
        .into_iter()
        .filter(|agent| app_agent_names.is_empty() || app_agent_names.contains(&agent.name))
        .map(|agent| AgentInfo {
            id: agent.id.0.to_string(),
            name: agent.name,
            capabilities: agent.capabilities.into_iter().map(|c| c.name).collect(),
            current_load: 0,
            max_load: 4,
            available: true,
        })
        .collect();
    if app_agents.is_empty() {
        tracing::warn!(
            app_id = %app_id,
            "No app-scoped agents resolved while ensuring executor"
        );
    }

    let _ = state
        .executor_registry
        .register_application(app_id.clone(), app_name, app_agents)
        .await;
}

/// POST /api/chat/stop — terminate all running processes for an application.
///
/// Sets the cancel flag for the coordinator loop, shuts down the executor,
/// and signals all PlanLoop/WorkerLoop instances to stop.
pub(crate) async fn post_chat_stop(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StopRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    // 1. Set coordinator cancel flag
    {
        let flags = state.sessions.cancel_flags.read().await;
        if let Some(flag) = flags.get(&req.app_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    // 2. Shared cleanup: executor, agents, tasks, loops
    cleanup_app_state(&state, &app_id).await;

    // 2a. Mark framework execution contexts as stopped for active sessions of this app.
    let stopped_session_ids: Vec<String> = state
        .sessions
        .active_sessions
        .read()
        .await
        .values()
        .filter(|s| s.app_id == app_id)
        .map(|s| s.session_id.clone())
        .collect();
    for sid in stopped_session_ids {
        let mut ctx = ExecutionContext::new(sid.clone(), app_id.0.to_string(), "unknown");
        let _ = load_module_state(
            state.sessions.framework_session_store.as_ref(),
            &sid,
            &mut ctx,
        )
        .await;
        ctx.mark_stopped(Some("user_stop_all_processes".into()));
        persist_execution_context(&state, &ctx).await;
    }

    // 3. Broadcast stop event to all sessions for this app
    let event = Event::default()
        .event("stopped")
        .data(serde_json::json!({"reason": "User terminated all processes"}).to_string());
    crate::sse::broadcast_to_app_sessions(
        &state,
        &app_id,
        event,
        serde_json::json!({"event": "terminated", "reason": "user_action"}),
    )
    .await;

    tracing::info!(app_id = %app_id, "All processes terminated");

    Ok(Json(serde_json::json!({
        "status": "terminated",
    })))
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// POST /api/chat (legacy shim)
// ---------------------------------------------------------------------------

/// Legacy compatibility shim.
///
/// `/api/chat` route has been removed from router registration, and framework
/// execution is now the only supported business path. If invoked directly,
/// forward to the framework handler.
#[deprecated(note = "Use post_chat_v2; legacy /api/chat is removed.")]
pub(crate) async fn post_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    post_chat_v2(State(state), Json(req)).await
}

// post_chat_v2 — Framework-engine based coordinator (ReActAgent)
// ---------------------------------------------------------------------------

/// Framework-based chat handler using `ReActAgent` instead of `AgenticLoop`.
///
/// This is the migration target for the coordinator loop. Activated via
/// `engine=framework` in the `ChatRequest`.
pub(crate) async fn post_chat_v2(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    use macaca_framework::agent::Agent;
    use macaca_framework::message::Msg;

    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    // Determine entry agent
    let entry_agent_name = {
        let registry = state.registry.read().await;
        registry
            .get_app(&app_id)
            .and_then(|a| a.manifest.entry_agent.clone())
            .unwrap_or_else(|| "coordinator".to_string())
    };

    // Session key
    let is_new_session = req.session_id.is_none();
    let session_key = req
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Clean up previous state when creating a new session
    if is_new_session {
        cleanup_app_state(&state, &app_id).await;
        ensure_app_executor(&state, &app_id).await;
        tracing::info!(app_id = %app_id, session_id = %session_key, "New session: cleaned up previous tasks, goals, agents and loops");
    }

    // SSE channels:
    // tx → rx: coordinator sends events here
    // stream_tx → stream_rx: SSE output reads from here
    // sse_tx: hot-swappable sender (initially stream_tx, for browser refresh recovery)
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let (stream_tx, stream_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Hot-swappable SSE sender — initialized with stream_tx (NOT tx, which would create a loop)
    let sse_tx: Arc<RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>> =
        Arc::new(RwLock::new(stream_tx));

    // Register cancel flag
    {
        let mut flags = state.sessions.cancel_flags.write().await;
        flags.insert(req.app_id.clone(), Arc::clone(&cancel_flag));
    }

    // Pause/resume channel for create_goal coordination
    let pause_signal = Arc::new(AtomicBool::new(false));
    let (resume_tx, resume_rx) =
        tokio::sync::mpsc::channel::<macaca_runtime::agentic_loop::ResumeReason>(4);

    // Stop old forwarder if re-entering the same session
    let forwarder_stop = Arc::new(AtomicBool::new(false));
    {
        let mut sessions = state.sessions.active_sessions.write().await;
        if let Some(old) = sessions.get(&session_key) {
            old.forwarder_stop
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        sessions.insert(
            session_key.clone(),
            crate::state::ActiveSession {
                session_id: session_key.clone(),
                app_id: app_id.clone(),
                pause_signal: Arc::clone(&pause_signal),
                resume_tx,
                sse_tx: Arc::clone(&sse_tx),
                forwarder_stop: Arc::clone(&forwarder_stop),
            },
        );
    }

    // Subscribe to executor events for delegated agent tracking
    let executor_events_rx = {
        if let Some(executor) = state.executor_registry.get(&app_id).await {
            Some(executor.subscribe_to_events())
        } else {
            None
        }
    };

    // Ensure PlanLoop + WorkerLoops are running
    crate::loop_manager::ensure_plan_and_worker_loops(&state, &app_id, Some(session_key.clone()))
        .await;

    // Build the framework coordinator agent
    let coordinator_result = crate::framework_runner::FrameworkRunner::build_coordinator(
        &state,
        &app_id,
        &entry_agent_name,
        Some(session_key.clone()),
        tx.clone(),
        pause_signal,
        resume_rx,
    )
    .await;

    let (coordinator, cancel_token) = match coordinator_result {
        Ok(c) => c,
        Err(e) => {
            return Err(err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build coordinator: {e}"),
            ));
        }
    };

    // Create initial session (status: running)
    let now = Utc::now();
    let title = req.prompt.chars().take(50).collect::<String>();
    let session_key_db = format!("{}{}", SESSION_PREFIX, &session_key);
    let initial_stored = StoredSession {
        meta: SessionMeta {
            session_id: session_key.clone(),
            app_id: app_id.0.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 1,
            title: Some(title.clone()),
            status: "running".to_string(),
        },
        turns: vec![StoredTurn {
            role: "user".into(),
            content: req.prompt.clone(),
            status: None,
            trace_steps: Vec::new(),
            cc_trace_steps: Vec::new(),
            meta: None,
            agent_traces: HashMap::new(),
        }],
        messages: vec![],
    };
    if let Ok(data) = serde_json::to_vec(&initial_stored) {
        let _ = state
            .persist
            .session_store
            .set(&session_key_db, &data)
            .await;
    }
    // Add to app sessions index (per-session key format, matching post_chat convention)
    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id.0, session_key);
    let _ = state
        .persist
        .session_store
        .set(&app_index_key, session_key.as_bytes())
        .await;

    // Persist framework-level execution context for session/trace/resume.
    let mut execution_context = ExecutionContext::new(
        session_key.clone(),
        app_id.0.to_string(),
        entry_agent_name.clone(),
    );
    execution_context.mark_running(Some("coordinator_started".into()));
    persist_execution_context(&state, &execution_context).await;

    let prompt = req.prompt.clone();
    let session_key_for_task = session_key.clone();
    let state_for_task = Arc::clone(&state);

    // Spawn the coordinator task
    tokio::spawn(async move {
        tracing::info!(
            engine = "framework",
            agent = entry_agent_name,
            session_id = %session_key_for_task,
            "Starting framework coordinator"
        );

        let mut exec_ctx = ExecutionContext::new(
            session_key_for_task.clone(),
            app_id.0.to_string(),
            entry_agent_name.clone(),
        );
        let _ = load_module_state(
            state_for_task.sessions.framework_session_store.as_ref(),
            &session_key_for_task,
            &mut exec_ctx,
        )
        .await;
        exec_ctx.mark_running(Some("coordinator_reply_started".into()));
        persist_execution_context(&state_for_task, &exec_ctx).await;

        if let Some(manifest) = state_for_task
            .kernel
            .get_agent_by_name(&entry_agent_name)
            .await
        {
            state_for_task
                .kernel
                .update_agent_activity(
                    &manifest.id,
                    macaca_proto::AgentActivity::Working {
                        context: format!("Handling session {}", session_key_for_task),
                    },
                )
                .await;
        }

        let user_msg = Msg::user("user", prompt.as_str());
        let result = coordinator.reply(user_msg).await;

        // Process result
        let (final_content, status) = match result {
            Ok(reply) => {
                let text = reply.get_text();
                tracing::info!(
                    engine = "framework",
                    output_len = text.len(),
                    "Framework coordinator completed"
                );
                exec_ctx.mark_completed(Some("coordinator_completed".into()));
                persist_execution_context(&state_for_task, &exec_ctx).await;
                (text, "completed")
            }
            Err(e) => {
                let error_msg = format!("Agent error: {e}");
                tracing::error!(engine = "framework", error = %e, "Framework coordinator failed");
                if let Some(manifest) = state_for_task
                    .kernel
                    .get_agent_by_name(&entry_agent_name)
                    .await
                {
                    state_for_task
                        .kernel
                        .update_agent_activity(
                            &manifest.id,
                            macaca_proto::AgentActivity::Error {
                                message: e.to_string(),
                            },
                        )
                        .await;
                }
                exec_ctx.mark_error(Some(e.to_string()));
                persist_execution_context(&state_for_task, &exec_ctx).await;
                // Send error SSE event
                let _ = tx
                    .send(Ok(Event::default()
                        .event("error")
                        .data(serde_json::json!({"error": error_msg}).to_string())))
                    .await;
                (error_msg, "error")
            }
        };

        // Update session with final result
        let now = Utc::now();
        let session_key_db = format!("{}{}", SESSION_PREFIX, &session_key_for_task);
        if let Ok(Some(data)) = state_for_task
            .persist
            .session_store
            .get(&session_key_db)
            .await
        {
            if let Ok(mut stored) = serde_json::from_slice::<StoredSession>(&data) {
                stored.meta.updated_at = now;
                stored.meta.status = status.to_string();
                stored.turns.push(StoredTurn {
                    role: "assistant".into(),
                    content: final_content,
                    status: Some(status.to_string()),
                    trace_steps: Vec::new(),
                    cc_trace_steps: Vec::new(),
                    meta: None,
                    agent_traces: HashMap::new(),
                });
                if let Ok(data) = serde_json::to_vec(&stored) {
                    let _ = state_for_task
                        .persist
                        .session_store
                        .set(&session_key_db, &data)
                        .await;
                }
            }
        }

        if status != "error" {
            if let Some(manifest) = state_for_task
                .kernel
                .get_agent_by_name(&entry_agent_name)
                .await
            {
                state_for_task
                    .kernel
                    .update_agent_activity(&manifest.id, macaca_proto::AgentActivity::Idle)
                    .await;
            }
        }

        // Cleanup
        {
            let mut sessions = state_for_task.sessions.active_sessions.write().await;
            sessions.remove(&session_key_for_task);
        }
        {
            let mut flags = state_for_task.sessions.cancel_flags.write().await;
            flags.remove(&app_id.0.to_string());
        }
    });

    // Bridge task: forward coordinator events (rx) → hot-swappable sse_tx → stream_rx
    let sse_tx_for_bridge = Arc::clone(&sse_tx);
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let sender = sse_tx_for_bridge.read().await;
            if sender.send(event).await.is_err() {
                break;
            }
        }
    });

    // Executor events forwarder: delegated agent traces → SSE stream + EventLog
    if let Some(mut exec_rx) = executor_events_rx {
        let sse_tx_for_exec = Arc::clone(&sse_tx);
        let event_log_for_exec = Arc::clone(&state.persist.event_log);
        let session_for_exec = session_key.clone();
        let forwarder_stop_flag = Arc::clone(&forwarder_stop);
        tokio::spawn(async move {
            while let Ok(exec_event) = exec_rx.recv().await {
                // Check if this forwarder has been superseded
                if forwarder_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    tracing::debug!("Executor event forwarder stopped (superseded)");
                    break;
                }
                // Persist to EventLog
                let (evt_type, evt_payload) = match &exec_event {
                    macaca_kernel::executor::ExecutorEvent::TaskStarted { task_id, agent } => (
                        "delegated_task_start",
                        serde_json::json!({ "task_id": task_id.to_string(), "agent": agent }),
                    ),
                    macaca_kernel::executor::ExecutorEvent::AgentEvent {
                        task_id,
                        agent,
                        event: agent_evt,
                    } => {
                        let sub = match agent_evt {
                            macaca_proto::AgentExecutionEvent::Thinking { .. } => {
                                "delegated_thinking"
                            }
                            macaca_proto::AgentExecutionEvent::ToolCall { .. } => {
                                "delegated_tool_call"
                            }
                            macaca_proto::AgentExecutionEvent::ToolResult { .. } => {
                                "delegated_tool_result"
                            }
                            macaca_proto::AgentExecutionEvent::Assistant { .. } => {
                                "delegated_assistant"
                            }
                            macaca_proto::AgentExecutionEvent::CcTrace { .. } => {
                                "delegated_cc_trace"
                            }
                            macaca_proto::AgentExecutionEvent::Completed { .. } => "delegated_done",
                        };
                        (
                            sub,
                            serde_json::json!({ "task_id": task_id.to_string(), "agent": agent, "event": agent_evt }),
                        )
                    }
                    macaca_kernel::executor::ExecutorEvent::TaskCompleted {
                        task_id,
                        agent,
                        result,
                    } => (
                        "delegated_task_complete",
                        serde_json::json!({ "task_id": task_id.to_string(), "agent": agent, "output": result.output, "success": result.success }),
                    ),
                    macaca_kernel::executor::ExecutorEvent::TaskFailed {
                        task_id,
                        agent,
                        error,
                    } => (
                        "delegated_task_error",
                        serde_json::json!({ "task_id": task_id.to_string(), "agent": agent, "error": error }),
                    ),
                    _ => continue,
                };
                event_log_for_exec
                    .append(&session_for_exec, evt_type, "executor", evt_payload)
                    .await;

                // Forward to SSE
                let sse_event = convert_executor_event_to_sse(exec_event);
                let sender = sse_tx_for_exec.read().await;
                if sender.send(sse_event).await.is_err() {
                    // SSE closed but keep persisting to EventLog
                }
            }
        });
    }

    let stream = async_stream::stream! {
        let mut stream_rx = stream_rx;
        // Emit session_id as the first event
        yield Ok(Event::default()
            .event("session_id")
            .data(serde_json::json!({"session_id": session_key}).to_string()));

        while let Some(event) = stream_rx.recv().await {
            yield event;
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
