//! Framework / service.agent_execution chat path.
//!
//! Boots PlanLoop/WorkerLoop, persists framework execution context, and runs the
//! visible entry agent through `service.agent_execution` while streaming SSE.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::response::sse::Event;
use chrono::Utc;
use macaca_sdk::framework::execution::ExecutionContext;
use macaca_sdk::framework::session::load_module_state;
use macaca_proto::{ApplicationId, McpCleanupCommand, McpServiceScope, TraceContext};
use tokio::sync::RwLock;

use crate::event_persistence::spawn_session_event_collector;
use crate::session::{AgentTraceCollector, StoredSession, StoredTurn, SESSION_PREFIX};
use crate::state::AppState;

use super::agent_execution_adapter::run_chat_main_thread_via_agent_service;
use super::application_service_adapter::notify_application_session_stop;
use super::executor_event_adapter::{spawn_executor_event_forwarder, spawn_sse_channel_bridge};
use super::session_persistence_adapter::persist_execution_context;
/// Run the framework coordinator path via `service.agent_execution`.
pub(crate) async fn run_framework_chat_path(
    state: Arc<AppState>,
    app_id: ApplicationId,
    session_key: String,
    entry_agent_name: String,
    prompt: String,
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    rx: tokio::sync::mpsc::Receiver<Result<Event, Infallible>>,
    sse_tx: Arc<RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
    forwarder_stop: Arc<AtomicBool>,
) {
    // Subscribe to executor events for delegated agent tracking
    let executor_for_collector = state.executor_registry.get(&app_id).await;
    let executor_events_rx = {
        if let Some(ref executor) = executor_for_collector {
            Some(executor.subscribe_to_events())
        } else {
            None
        }
    };

    // Spawn the persistent event collector (EventLog + RunTracer + AgentTraceCollector)
    let trace_collector = AgentTraceCollector::new();
    if let Some(ref executor) = executor_for_collector {
        spawn_session_event_collector(
            Arc::clone(executor),
            Arc::clone(&state.persist.event_log),
            Arc::clone(&state.persist.run_tracer),
            session_key.clone(),
            Arc::clone(&trace_collector),
        );
    }

    // Ensure PlanLoop + WorkerLoops are running
    crate::loop_manager::ensure_plan_and_worker_loops(&state, &app_id, Some(session_key.clone()))
        .await;

    // Persist framework-level execution context for session/trace/resume.
    let mut execution_context = ExecutionContext::new(
        session_key.clone(),
        app_id.0.to_string(),
        entry_agent_name.clone(),
    );
    execution_context.mark_running(Some("coordinator_started".into()));
    persist_execution_context(&state, &execution_context).await;

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

        let result = run_chat_main_thread_via_agent_service(
            &state_for_task,
            &app_id,
            &session_key_for_task,
            &entry_agent_name,
            prompt,
        )
        .await;

        // Process result
        let (final_content, status) = match result {
            Ok(text) => {
                tracing::info!(
                    engine = "service.agent_execution",
                    output_len = text.len(),
                    "Chat main-thread service execution completed"
                );
                exec_ctx.mark_completed(Some("coordinator_completed".into()));
                persist_execution_context(&state_for_task, &exec_ctx).await;
                (text, "completed")
            }
            Err(e) => {
                let error_msg = format!("Agent error: {e}");
                tracing::error!(engine = "service.agent_execution", error = %e, "Chat main-thread service execution failed");
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

        if status != "error" {
            let _ = tx
                .send(Ok(Event::default().event("assistant").data(
                    serde_json::json!({ "content": final_content.clone() }).to_string(),
                )))
                .await;
        }

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
                    content: final_content.clone(),
                    status: Some(status.to_string()),
                    trace_steps: Vec::new(),
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
            // Treat the public `done` SSE event as the durable completion boundary.
            // The memory write is intentionally performed before `done` so local
            // monitors can rely on a completed stream meaning that sanitized
            // long-term memory capture has already been attempted.  The capture
            // helper owns error downgrading, so an unavailable memory backend
            // remains observable without turning successful chat execution into
            // an application-level failure.
            let mut trace = TraceContext::new(format!(
                "chat-session-memory-capture:{session_key_for_task}"
            ));
            trace.session_id = Some(session_key_for_task.clone());
            trace.agent = Some(entry_agent_name.clone());
            crate::session_memory_capture::capture_successful_session_completion(
                Arc::clone(&state_for_task.memory_client),
                app_id,
                session_key_for_task.clone(),
                entry_agent_name.clone(),
                final_content,
                trace,
            )
            .await;

            let _ = tx
                .send(Ok(Event::default().event("done").data(
                    serde_json::json!({
                        "status": "completed",
                        "mode": "service_agent_execution",
                    })
                    .to_string(),
                )))
                .await;

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
        notify_application_session_stop(
            &state_for_task,
            &app_id,
            &session_key_for_task,
            &entry_agent_name,
            status,
        )
        .await;
        // Route C: session teardown must flow through `service.mcp` so the shell
        // does not retain a second owner of MCP subprocess leases.
        let cleanup_command = McpCleanupCommand {
            trace: TraceContext::new(format!("chat-session-mcp-cleanup:{session_key_for_task}")),
            scope: McpServiceScope::agent_session(
                app_id,
                session_key_for_task.clone(),
                entry_agent_name.clone(),
            )
            .unwrap_or_default(),
        };
        if let Err(error) = state_for_task.mcp_client.cleanup(cleanup_command).await {
            tracing::warn!(
                session_id = %session_key_for_task,
                agent = %entry_agent_name,
                error = %error,
                "mcp service cleanup failed during chat session teardown"
            );
        }
        {
            let mut sessions = state_for_task.sessions.active_sessions.write().await;
            sessions.remove(&session_key_for_task);
        }
        {
            let mut flags = state_for_task.sessions.cancel_flags.write().await;
            flags.remove(&app_id.0.to_string());
        }
    });

    spawn_sse_channel_bridge(rx, Arc::clone(&sse_tx));

    // Executor events forwarder: delegated agent traces → SSE stream
    // (EventLog persistence is handled by spawn_session_event_collector above)
    if let Some(exec_rx) = executor_events_rx {
        spawn_executor_event_forwarder(
            Arc::clone(&state),
            exec_rx,
            Arc::clone(&sse_tx),
            Arc::clone(&forwarder_stop),
            "framework",
        );
    }

}
