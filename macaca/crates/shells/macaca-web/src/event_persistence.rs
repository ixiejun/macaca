//! Event persistence — writes executor events to EventLog.
//!
//! All EventLog write logic lives here, NOT in routes.rs.
//! routes.rs calls `spawn_session_event_collector()` with the session_id;
//! this module handles the rest.

use std::sync::Arc;

use tokio::task::JoinHandle;

use macaca_runtime_host::executor::app_executor::ApplicationExecutor;
use macaca_runtime_host::executor::ExecutorEvent;
use macaca_persist::{AppendEventCommand, EventLog};

use crate::proto_event_visitors::delegated_persisted_event_name;
use crate::run_trace::{phase, status, RunTracer};

/// Spawn a per-session event collector that subscribes to executor events
/// and writes them to the EventLog keyed by `session_id`.
///
/// This is a single subscriber — no race conditions between registration
/// and writing because session_id is known upfront.
///
/// Also feeds the `AgentTraceCollector` for backward-compatible session save.
pub(crate) fn spawn_session_event_collector(
    executor: Arc<ApplicationExecutor>,
    event_log: Arc<EventLog>,
    run_tracer: Arc<RunTracer>,
    session_id: String,
    collector: Arc<crate::session::AgentTraceCollector>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut evt_rx = executor.subscribe_to_events();
        loop {
            match evt_rx.recv().await {
                Ok(event) => {
                    // 1. Feed the AgentTraceCollector (for session save compatibility).
                    match &event {
                        ExecutorEvent::TaskStarted { task_id, agent } => {
                            collector.on_task_started(&task_id.to_string(), agent).await;
                        }
                        ExecutorEvent::AgentEvent {
                            task_id,
                            agent,
                            event: agent_event,
                        } => {
                            collector
                                .on_agent_event(&task_id.to_string(), agent, agent_event)
                                .await;
                        }
                        ExecutorEvent::TaskCompleted {
                            task_id, result, ..
                        } => {
                            collector
                                .on_task_completed(
                                    &task_id.to_string(),
                                    result.success,
                                    Some(result.output.clone()),
                                    None,
                                )
                                .await;
                        }
                        ExecutorEvent::TaskFailed { task_id, error, .. } => {
                            collector
                                .on_task_completed(
                                    &task_id.to_string(),
                                    false,
                                    None,
                                    Some(error.clone()),
                                )
                                .await;
                        }
                        _ => {}
                    }

                    // 2. Write to EventLog (the durable persistence layer).
                    let (evt_type, evt_payload) = match &event {
                        ExecutorEvent::TaskStarted { task_id, agent } => (
                            "delegated_task_start",
                            serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                            }),
                        ),
                        ExecutorEvent::AgentEvent {
                            task_id,
                            agent,
                            event: ref agent_evt,
                        } => (
                            delegated_persisted_event_name(agent_evt),
                            serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "event": agent_evt,
                            }),
                        ),
                        ExecutorEvent::TaskCompleted {
                            task_id,
                            agent,
                            result,
                        } => (
                            "delegated_task_complete",
                            serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "output": result.output,
                                "success": result.success,
                            }),
                        ),
                        ExecutorEvent::TaskFailed {
                            task_id,
                            agent,
                            error,
                        } => (
                            "delegated_task_error",
                            serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "error": error,
                            }),
                        ),
                        ExecutorEvent::TaskCancelled { task_id } => (
                            "delegated_task_cancelled",
                            serde_json::json!({"task_id": task_id.to_string()}),
                        ),
                        ExecutorEvent::TaskProgress { task_id, step, .. } => (
                            "delegated_task_progress",
                            serde_json::json!({"task_id": task_id.to_string(), "step": step}),
                        ),
                        ExecutorEvent::HookEvent { .. } | ExecutorEvent::Shutdown => continue,
                    };

                    event_log
                        .append_command(AppendEventCommand::new(
                            &session_id,
                            evt_type,
                            "executor",
                            evt_payload,
                        ))
                        .await;

                    match &event {
                        ExecutorEvent::TaskStarted { task_id, agent } => {
                            run_tracer
                                .emit(
                                    &session_id,
                                    phase::DELEGATE_TASK_START,
                                    "executor",
                                    status::INFO,
                                    Some(format!("agent={agent}")),
                                    Some(task_id.to_string()),
                                    None,
                                    None,
                                )
                                .await;
                        }
                        ExecutorEvent::TaskCompleted {
                            task_id, result, ..
                        } => {
                            run_tracer
                                .emit(
                                    &session_id,
                                    phase::DELEGATE_TASK_COMPLETE,
                                    "executor",
                                    if result.success {
                                        status::OK
                                    } else {
                                        status::ERROR
                                    },
                                    None,
                                    Some(task_id.to_string()),
                                    None,
                                    Some(serde_json::json!({"success": result.success})),
                                )
                                .await;
                        }
                        ExecutorEvent::TaskFailed { task_id, error, .. } => {
                            run_tracer
                                .emit(
                                    &session_id,
                                    phase::DELEGATE_TASK_FAILED,
                                    "executor",
                                    status::ERROR,
                                    Some(error.clone()),
                                    Some(task_id.to_string()),
                                    None,
                                    None,
                                )
                                .await;
                        }
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "Session event collector: broadcast lagged");
                }
            }
        }
    })
}
