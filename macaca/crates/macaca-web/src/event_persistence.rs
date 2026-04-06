//! Event persistence — writes executor events to EventLog.
//!
//! All EventLog write logic lives here, NOT in routes.rs.
//! routes.rs calls `spawn_session_event_collector()` with the session_id;
//! this module handles the rest.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::task::JoinHandle;

use macaca_kernel::executor::app_executor::ApplicationExecutor;
use macaca_kernel::executor::ExecutorEvent;
use macaca_persist::EventLog;

/// Spawn a per-session event collector that subscribes to executor events
/// and writes them to the EventLog keyed by `session_id`.
///
/// This is a single subscriber — no race conditions between registration
/// and writing because session_id is known upfront.
///
/// Also feeds the `AgentTraceCollector` for backward-compatible session save.
pub fn spawn_session_event_collector(
    executor: Arc<ApplicationExecutor>,
    event_log: Arc<EventLog>,
    session_id: String,
    collector: Arc<crate::routes::AgentTraceCollector>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut evt_rx = executor.subscribe_to_events();
        let mut task_to_agent: HashMap<String, String> = HashMap::new();

        loop {
            match evt_rx.recv().await {
                Ok(event) => {
                    // 1. Feed the AgentTraceCollector (for session save compatibility).
                    match &event {
                        ExecutorEvent::TaskStarted { task_id, agent } => {
                            task_to_agent.insert(task_id.to_string(), agent.clone());
                            collector.on_task_started(&task_id.to_string(), agent).await;
                        }
                        ExecutorEvent::AgentEvent { task_id, agent, event: agent_event } => {
                            collector.on_agent_event(&task_id.to_string(), agent, agent_event).await;
                        }
                        ExecutorEvent::TaskCompleted { task_id, result } => {
                            collector.on_task_completed(
                                &task_id.to_string(), result.success,
                                Some(result.output.clone()), None,
                            ).await;
                        }
                        ExecutorEvent::TaskFailed { task_id, error } => {
                            collector.on_task_completed(
                                &task_id.to_string(), false, None, Some(error.clone()),
                            ).await;
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
                        ExecutorEvent::AgentEvent { task_id, agent, event: ref agent_evt } => {
                            let sub = match agent_evt {
                                macaca_proto::AgentExecutionEvent::Thinking { .. } => "delegated_thinking",
                                macaca_proto::AgentExecutionEvent::ToolCall { .. } => "delegated_tool_call",
                                macaca_proto::AgentExecutionEvent::ToolResult { .. } => "delegated_tool_result",
                                macaca_proto::AgentExecutionEvent::Assistant { .. } => "delegated_assistant",
                                macaca_proto::AgentExecutionEvent::CcTrace { .. } => "delegated_cc_trace",
                                macaca_proto::AgentExecutionEvent::Completed { .. } => "delegated_done",
                            };
                            (sub, serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "event": agent_evt,
                            }))
                        },
                        ExecutorEvent::TaskCompleted { task_id, result } => {
                            let agent = task_to_agent.get(&task_id.to_string()).cloned().unwrap_or_default();
                            ("delegated_task_complete", serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "output": result.output,
                                "success": result.success,
                            }))
                        },
                        ExecutorEvent::TaskFailed { task_id, error } => {
                            let agent = task_to_agent.get(&task_id.to_string()).cloned().unwrap_or_default();
                            ("delegated_task_error", serde_json::json!({
                                "task_id": task_id.to_string(),
                                "agent": agent,
                                "error": error,
                            }))
                        },
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

                    event_log.append(&session_id, evt_type, "executor", evt_payload).await;

                    // Clean up local state on terminal events.
                    match &event {
                        ExecutorEvent::TaskCompleted { task_id, .. }
                        | ExecutorEvent::TaskFailed { task_id, .. }
                        | ExecutorEvent::TaskCancelled { task_id } => {
                            task_to_agent.remove(&task_id.to_string());
                        }
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "Session event collector: broadcast lagged");
                }
            }
        }
    })
}
