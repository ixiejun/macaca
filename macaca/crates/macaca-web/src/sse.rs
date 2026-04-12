//! SSE event conversion, broadcasting, and plan decision persistence.

use std::convert::Infallible;
use std::sync::Arc;

use axum::response::sse::Event;
use serde::{Deserialize, Serialize};

use macaca_kernel::executor::ExecutorEvent;
use macaca_persist::{PersistStore, RedbStore};
use macaca_proto::ApplicationId;

use crate::state::AppState;

/// Separate key prefix for plan decision events — stored independently per session.
pub(crate) const PLAN_DECISIONS_PREFIX: &str = "plan_decisions/";

/// A single decision event emitted by the PlanLoop or WorkerLoop.
/// Stored independently per session to avoid read-modify-write races.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct PlanDecisionEvent {
    pub decision_type: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
}

/// Append a plan decision to its dedicated per-session key (read-append-write).
pub(crate) async fn save_plan_decision(
    store: &Arc<RedbStore>,
    app_id: &ApplicationId,
    decision: PlanDecisionEvent,
) {
    let key = format!("{}{}", PLAN_DECISIONS_PREFIX, app_id);
    let mut decisions: Vec<PlanDecisionEvent> = match store.get(&key).await {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => Vec::new(),
    };
    decisions.push(decision);
    if let Ok(data) = serde_json::to_vec(&decisions) {
        let _ = store.set(&key, &data).await;
    }
}

/// Load all plan decisions for an application.
pub(crate) async fn load_plan_decisions(
    store: &Arc<RedbStore>,
    app_id: &ApplicationId,
) -> Vec<PlanDecisionEvent> {
    let key = format!("{}{}", PLAN_DECISIONS_PREFIX, app_id);
    match store.get(&key).await {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Convert ExecutorEvent to SSE Event for frontend display.
/// Each event includes an `agent_tab` field for frontend to group events by agent.
pub(crate) fn convert_executor_event_to_sse(event: ExecutorEvent) -> Result<Event, Infallible> {
    match event {
        ExecutorEvent::TaskStarted { task_id, agent } => {
            Ok(Event::default().event("delegated_task_start").data(
                serde_json::json!({
                    "task_id": task_id.to_string(),
                    "agent": agent,
                    "agent_tab": agent,
                })
                .to_string(),
            ))
        }
        ExecutorEvent::AgentEvent {
            task_id,
            agent,
            event: agent_event,
        } => {
            // Forward the internal agent execution event
            let event_type = match &agent_event {
                macaca_proto::AgentExecutionEvent::Thinking { .. } => "delegated_thinking",
                macaca_proto::AgentExecutionEvent::ToolCall { .. } => "delegated_tool_call",
                macaca_proto::AgentExecutionEvent::ToolResult { .. } => "delegated_tool_result",
                macaca_proto::AgentExecutionEvent::Assistant { .. } => "delegated_assistant",
                macaca_proto::AgentExecutionEvent::CcTrace { .. } => "delegated_cc_trace",
                macaca_proto::AgentExecutionEvent::Completed { .. } => "delegated_completed",
            };
            Ok(Event::default().event(event_type).data(
                serde_json::json!({
                    "task_id": task_id.to_string(),
                    "agent": agent,
                    "agent_tab": agent,
                    "event": agent_event,
                })
                .to_string(),
            ))
        }
        ExecutorEvent::TaskCompleted {
            task_id,
            agent,
            result,
        } => Ok(Event::default().event("delegated_task_complete").data(
            serde_json::json!({
                "task_id": task_id.to_string(),
                "agent": agent,
                "agent_tab": agent,
                "success": result.success,
                "output": result.output,
            })
            .to_string(),
        )),
        ExecutorEvent::TaskFailed {
            task_id,
            agent,
            error,
        } => Ok(Event::default().event("delegated_task_error").data(
            serde_json::json!({
                "task_id": task_id.to_string(),
                "agent": agent,
                "agent_tab": agent,
                "error": error,
            })
            .to_string(),
        )),
        ExecutorEvent::TaskCancelled { task_id } => {
            Ok(Event::default().event("delegated_task_cancelled").data(
                serde_json::json!({
                    "task_id": task_id.to_string(),
                })
                .to_string(),
            ))
        }
        ExecutorEvent::TaskProgress {
            task_id,
            step,
            output,
        } => Ok(Event::default().event("delegated_task_progress").data(
            serde_json::json!({
                "task_id": task_id.to_string(),
                "step": step,
                "output": output,
            })
            .to_string(),
        )),
        ExecutorEvent::Shutdown => Ok(Event::default()
            .event("executor_shutdown")
            .data("{}".to_string())),
        ExecutorEvent::HookEvent { event: hook_event } => {
            // Convert HookEvent to SSE for coordinator notification
            match hook_event {
                macaca_kernel::executor::fork_manager::HookEvent::DelegateCompleted {
                    fork_id,
                    task_id,
                    success,
                    output,
                } => Ok(Event::default().event("hook_delegate_completed").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "task_id": task_id.to_string(),
                        "success": success,
                        "output": output,
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
                macaca_kernel::executor::fork_manager::HookEvent::DelegateFailed {
                    fork_id,
                    task_id,
                    error,
                } => Ok(Event::default().event("hook_delegate_failed").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "task_id": task_id.to_string(),
                        "error": error,
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
                macaca_kernel::executor::fork_manager::HookEvent::ForkValidated {
                    fork_id,
                    result,
                } => Ok(Event::default().event("hook_fork_validated").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "result": format!("{:?}", result),
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
                macaca_kernel::executor::fork_manager::HookEvent::ForkMerged { fork_id } => {
                    Ok(Event::default().event("hook_fork_merged").data(
                        serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "agent_tab": "hook",
                        })
                        .to_string(),
                    ))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkCreated {
                    fork_id,
                    application_id,
                    agent_name,
                } => Ok(Event::default().event("hook_fork_created").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "application_id": application_id.to_string(),
                        "agent_name": agent_name,
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
                macaca_kernel::executor::fork_manager::HookEvent::ForkWaiting {
                    fork_id,
                    delegate_task_id,
                } => Ok(Event::default().event("hook_fork_waiting").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "delegate_task_id": delegate_task_id.to_string(),
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
                macaca_kernel::executor::fork_manager::HookEvent::ForkResumed {
                    fork_id,
                    delegate_result,
                } => Ok(Event::default().event("hook_fork_resumed").data(
                    serde_json::json!({
                        "fork_id": fork_id.to_string(),
                        "task_id": delegate_result.task_id.to_string(),
                        "success": delegate_result.success,
                        "agent_tab": "hook",
                    })
                    .to_string(),
                )),
            }
        }
    }
}

/// Send an SSE event to all active sessions belonging to a given application.
///
/// PlanLoop/WorkerLoop events are app-scoped (one loop per app), but SSE
/// connections are session-scoped. This broadcasts the same event to every
/// open browser tab that is watching the app.
///
/// `log_payload` is persisted to the EventLog for every matching session before
/// the SSE send, ensuring durability even if the browser connection drops.
pub(crate) async fn broadcast_to_app_sessions(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    event: Event,
    log_payload: serde_json::Value,
) {
    let sessions = state.sessions.active_sessions.read().await;
    let total = sessions.len();
    let matching: Vec<_> = sessions.values().filter(|s| &s.app_id == app_id).collect();
    tracing::info!(
        total_sessions = total,
        matching_sessions = matching.len(),
        app_id = %app_id,
        "Broadcasting plan_decision to app sessions"
    );
    // Append to EventLog BEFORE SSE send (durability guarantee).
    for session in &matching {
        state
            .persist
            .event_log
            .append(
                &session.session_id,
                "plan_decision",
                "plan_loop",
                log_payload.clone(),
            )
            .await;
    }
    for session in &matching {
        let tx = session.sse_tx.read().await;
        match tx.try_send(Ok(event.clone())) {
            Ok(()) => tracing::info!(session_id = %session.session_id, "plan_decision sent"),
            Err(e) => {
                tracing::warn!(session_id = %session.session_id, error = %e, "plan_decision send failed")
            }
        }
    }
}
