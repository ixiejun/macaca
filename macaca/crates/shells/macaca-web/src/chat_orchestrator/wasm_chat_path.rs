//! WASM host-dispatch fast-path execution.
//!
//! Agentless or WASM-first applications stream deterministic trace stages through
//! `application_client.host_dispatch` without bootstrapping framework loops.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::response::sse::Event;
use macaca_proto::{
    ApplicationHostCommandStatus, ApplicationHostDispatchServiceCommand, ApplicationId,
};
use tokio::sync::RwLock;

use crate::event_persistence::spawn_session_event_collector;
use crate::runtime_event_bridge::emit_host_command_result_events;
use crate::session::{persist_session_snapshot, AgentTraceCollector};
use crate::state::AppState;

use super::agent_activity_adapter::update_agent_activity_by_name;
use super::executor_event_adapter::{spawn_executor_event_forwarder, spawn_sse_channel_bridge};
/// Run the WASM fast-path: bind session metadata and spawn host dispatch.
pub(crate) async fn run_wasm_chat_fast_path(
    state: Arc<AppState>,
    app_id: ApplicationId,
    session_key: String,
    entry_agent_name: String,
    mut dispatch: ApplicationHostDispatchServiceCommand,
    request_model_hint: Option<String>,
    request_route_metadata: BTreeMap<String, String>,
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    rx: tokio::sync::mpsc::Receiver<Result<Event, Infallible>>,
    sse_tx: Arc<RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
    forwarder_stop: Arc<AtomicBool>,
) {
    // The Web Shell owns the user-visible session id, while portable WASM
    // artifacts only know template values such as `${session.id}`.  Bind
    // the real session to the host-dispatch command here so downstream
    // imports like `ui.render` store surfaces under the same key that the
    // chat page will later query.
    dispatch.trace.session_id = Some(session_key.clone());
    dispatch.scope.session_id = Some(session_key.clone());
    if let Some(trace) = dispatch.host_command.trace.as_mut() {
        trace.session_id = Some(session_key.clone());
    }
    dispatch
        .host_command
        .metadata
        .insert("session.id".into(), session_key.clone());
    if let Some(model_hint) = request_model_hint.as_ref() {
        dispatch
            .host_command
            .metadata
            .insert("llm.request_model".into(), model_hint.clone());
    }
    for (key, value) in &request_route_metadata {
        dispatch
            .host_command
            .metadata
            .insert(format!("llm.route.{key}"), value.clone());
    }
    // WASM host dispatch can still delegate to app-scoped agents through
    // `macaca:agent/delegate`.  The fast path returns before the framework
    // coordinator setup below, so it must install the same executor event
    // bridges here; otherwise delegated tabs have only the final host
    // command summary and miss the detailed agent trace stream that YAML
    // apps already expose.
    if let Some(executor) = state.executor_registry.get(&app_id).await {
        spawn_session_event_collector(
            Arc::clone(&executor),
            Arc::clone(&state.persist.event_log),
            Arc::clone(&state.persist.run_tracer),
            session_key.clone(),
            AgentTraceCollector::new(),
        );
        let exec_rx = executor.subscribe_to_events();
        spawn_executor_event_forwarder(
            Arc::clone(&state),
            exec_rx,
            Arc::clone(&sse_tx),
            Arc::clone(&forwarder_stop),
            "wasm",
        );
    }
    let session_key_for_task = session_key.clone();
    let app_id_for_task = app_id;
    let entry_agent_name_for_task = entry_agent_name.clone();
    let state_for_task = Arc::clone(&state);
    let tx_for_task = tx.clone();
    let forwarder_stop_for_task = Arc::clone(&forwarder_stop);
    let request_route_metadata_for_task = request_route_metadata.clone();
    tokio::spawn(async move {
        update_agent_activity_by_name(
            &state_for_task,
            &entry_agent_name_for_task,
            macaca_proto::AgentActivity::Working {
                context: format!("Handling WASM session {}", session_key_for_task),
            },
        )
        .await;
        let _ = tx_for_task
            .send(Ok(Event::default().event("thinking").data(
                serde_json::json!({
                    "iteration": 1,
                    "phase": "wasm_host_dispatch",
                })
                .to_string(),
            )))
            .await;
        tracing::info!(
            session_id = %session_key_for_task,
            app_id = %app_id_for_task,
            "Starting WASM chat dispatch path (agentless runtime)"
        );
        match state_for_task
            .application_client
            .host_dispatch(dispatch)
            .await
        {
            Ok(output) => {
                emit_host_command_result_events(
                    &state_for_task,
                    &session_key_for_task,
                    "wasm_host_dispatch",
                    &output.output,
                )
                .await;
                let summary = match output.status {
                    ApplicationHostCommandStatus::Ok => "WASM execution completed",
                    ApplicationHostCommandStatus::Unavailable { .. } => {
                        "WASM execution unavailable"
                    }
                    ApplicationHostCommandStatus::DisabledByPolicy { .. } => {
                        "WASM execution denied by policy"
                    }
                    ApplicationHostCommandStatus::Unsupported { .. } => {
                        "WASM execution unsupported"
                    }
                    ApplicationHostCommandStatus::RuntimeUnavailable { .. } => {
                        "WASM runtime unavailable"
                    }
                    ApplicationHostCommandStatus::Rejected { .. } => "WASM execution rejected",
                };
                let content = format!(
                    "{summary}\n{}",
                    serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string())
                );
                persist_session_snapshot(
                    &state_for_task.persist.session_store,
                    &session_key_for_task,
                    &app_id_for_task,
                    Some("completed"),
                    Some(content.clone()),
                    None,
                    None,
                    None,
                )
                .await;
                let _ = tx_for_task
                    .send(Ok(Event::default().event("assistant").data(
                        serde_json::json!({
                            "content": content,
                        })
                        .to_string(),
                    )))
                    .await;
                let _ = tx_for_task
                    .send(Ok(Event::default().event("done").data(
                        serde_json::json!({
                            "status":"completed",
                            "mode":"wasm_agentless_dispatch",
                            "llm_route": request_route_metadata_for_task,
                        })
                        .to_string(),
                    )))
                    .await;
                update_agent_activity_by_name(
                    &state_for_task,
                    &entry_agent_name_for_task,
                    macaca_proto::AgentActivity::Idle,
                )
                .await;
            }
            Err(error) => {
                let error_msg = format!("WASM host dispatch failed: {error}");
                persist_session_snapshot(
                    &state_for_task.persist.session_store,
                    &session_key_for_task,
                    &app_id_for_task,
                    Some("error"),
                    Some(error_msg.clone()),
                    None,
                    None,
                    None,
                )
                .await;
                let _ = tx_for_task
                    .send(Ok(Event::default().event("error").data(
                        serde_json::json!({
                            "error": error_msg.clone(),
                        })
                        .to_string(),
                    )))
                    .await;
                update_agent_activity_by_name(
                    &state_for_task,
                    &entry_agent_name_for_task,
                    macaca_proto::AgentActivity::Error { message: error_msg },
                )
                .await;
            }
        }
        // The agentless WASM path owns its own stream lifecycle.  Once the
        // terminal SSE event has been sent, remove the active session and
        // stop the executor forwarder so the last `stream_tx` clone drops;
        // otherwise Axum keep-alive continues writing empty heartbeats even
        // though business execution is already complete.
        forwarder_stop_for_task.store(true, std::sync::atomic::Ordering::Relaxed);
        {
            let mut sessions = state_for_task.sessions.active_sessions.write().await;
            sessions.remove(&session_key_for_task);
        }
        state_for_task
            .sessions
            .execution_control_local_notifications
            .remove(&session_key_for_task)
            .await;
        {
            let mut flags = state_for_task.sessions.cancel_flags.write().await;
            flags.remove(&app_id_for_task.0.to_string());
        }
    });

    spawn_sse_channel_bridge(rx, Arc::clone(&sse_tx));
}
