//! POST /api/chat/v2 route handler.
//!
//! Resolves entry agent, prepares session state, and dispatches to either the
//! WASM host-dispatch fast path or the framework `service.agent_execution` path.

use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use macaca_proto::{
    ApplicationHostCommand, ApplicationHostDispatchServiceCommand, ApplicationId,
    ApplicationImport, ApplicationServiceScope, TraceContext,
};
use tokio::sync::RwLock;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

use super::application_service_adapter::{
    notify_application_session_start, resolve_required_entry_agent_name,
};
use super::dto::ChatRequest;
use super::executor_adapter::{cleanup_app_state, ensure_app_executor};
use super::framework_chat_path::run_framework_chat_path;
use super::session_persistence_adapter::persist_initial_chat_session;
use super::sse_adapter::build_chat_sse;
use super::wasm_chat_path::run_wasm_chat_fast_path;
use super::wasm_dispatch_adapter::{
    application_declares_agents, new_session_preparation_for_chat, wasm_chat_dispatch_command,
    wasm_chat_export_payload, NewSessionPreparation,
};

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
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    // Entry agent must come from Application Service or manifest projection — never
    // a shell-level default role name (application-agnostic OS boundary).
    let entry_agent_name = resolve_required_entry_agent_name(&state, &app_id)
        .await
        .map_err(|message| err(StatusCode::BAD_REQUEST, message))?;

    // Session key
    let is_new_session = req.session_id.is_none();
    let session_key = req
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    notify_application_session_start(&state, &app_id, &session_key, &entry_agent_name).await;
    let request_model_hint = {
        let trimmed = req.model.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    };
    if let Some(model_hint) = &request_model_hint {
        state
            .sessions
            .llm_route_hints
            .write()
            .await
            .insert(session_key.clone(), model_hint.clone());
        tracing::info!(
            app_id = %app_id,
            session_id = %session_key,
            model_hint_present = true,
            "registered request-level LLM route hint for application execution"
        );
    }
    let app_defaults =
        crate::application_shell_adapter::manifest_llm_config(&state, &app_id).await;
    let system_model = (!state.config.default_model.is_empty())
        .then(|| state.config.default_model.clone());
    let request_route_metadata = crate::llm_route_shell_adapter::resolve_request_route_metadata(
        &state,
        &app_id,
        &session_key,
        &entry_agent_name,
        request_model_hint.as_deref(),
        app_defaults,
        system_model,
    )
    .await;

    // Clean up previous state when creating a new session
    let mut wasm_dispatch = wasm_chat_dispatch_command(
        &state,
        &app_id,
        TraceContext::new("web-chat-wasm-dispatch"),
        &req.prompt,
    )
    .await;
    if is_new_session {
        cleanup_app_state(&state, &app_id).await;
        let declares_wasm_agents = if wasm_dispatch.is_some() {
            application_declares_agents(&state, &app_id).await
        } else {
            false
        };
        match new_session_preparation_for_chat(wasm_dispatch.is_some(), declares_wasm_agents) {
            NewSessionPreparation::WasmOrchestrationExecutor => {
                ensure_app_executor(&state, &app_id)
                    .await
                    .map_err(|error| {
                        err(
                            StatusCode::FAILED_DEPENDENCY,
                            format!("Failed to prepare WASM orchestration executor: {error}"),
                        )
                    })?;
                crate::loop_manager::ensure_plan_and_worker_loops(
                    &state,
                    &app_id,
                    Some(session_key.clone()),
                )
                .await;
                tracing::info!(
                    app_id = %app_id,
                    session_id = %session_key,
                    "Prepared app-scoped executor and loops for WASM orchestration session"
                );
            }
            NewSessionPreparation::FrameworkExecutor => {
                match ensure_app_executor(&state, &app_id).await {
                    Ok(()) => {}
                    Err(error)
                        if crate::application_shell_adapter::is_registry_wasm_layer_app(
                            &state, &app_id,
                        )
                        .await =>
                    {
                        tracing::info!(
                            app_id = %app_id,
                            error = %error,
                            "Executor preparation denied for agentless app; switching to WASM host-dispatch path"
                        );
                        wasm_dispatch = Some(ApplicationHostDispatchServiceCommand {
                            trace: TraceContext::new("web-chat-wasm-dispatch-fallback"),
                            scope: ApplicationServiceScope::application(app_id),
                            host_command: {
                                let mut command = ApplicationHostCommand::with_trace(
                                    ApplicationImport::TraceEmit,
                                    wasm_chat_export_payload(&req.prompt),
                                    TraceContext::new("web-chat-wasm-dispatch-fallback-command"),
                                );
                                command
                                    .metadata
                                    .insert("wasm.export".into(), "app:start".into());
                                command
                            },
                        });
                    }
                    Err(error) => {
                        return Err(err(
                            StatusCode::FAILED_DEPENDENCY,
                            format!("Failed to prepare application executor: {error}"),
                        ));
                    }
                }
            }
            NewSessionPreparation::WasmHostDispatchOnly => {}
        }
        tracing::info!(app_id = %app_id, session_id = %session_key, "New session: cleaned up previous tasks, goals, agents and loops");
    }

    // SSE channels:
    // tx → rx: coordinator sends events here
    // stream_tx → stream_rx: SSE output reads from here
    // sse_tx: hot-swappable sender (initially stream_tx, for browser refresh recovery)
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
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

    // Session-local execution-control resume channel. The selected policy is
    // registered later by `service.agent_execution`, which swaps this sender for
    // the concrete run while preserving the browser-visible session handle.
    let pause_signal = Arc::new(AtomicBool::new(false));
    let (resume_tx, _resume_rx) =
        tokio::sync::mpsc::channel::<crate::runtime_resume::RuntimeResumeSignal>(4);

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

    persist_initial_chat_session(&state, &app_id, &session_key, &req.prompt).await;

    // Agentless WASM fast-path:
    // - do NOT bootstrap framework coordinator/executor loops,
    // - do NOT create MCP agent-session noise,
    // - stream deterministic trace stages directly for frontend trace panels.
    if let Some(dispatch) = wasm_dispatch {
        run_wasm_chat_fast_path(
            state,
            app_id,
            session_key.clone(),
            entry_agent_name,
            dispatch,
            request_model_hint,
            request_route_metadata,
            tx,
            rx,
            sse_tx,
            forwarder_stop,
        )
        .await;
    } else {
        run_framework_chat_path(
            state,
            app_id,
            session_key.clone(),
            entry_agent_name,
            req.prompt,
            tx,
            rx,
            sse_tx,
            forwarder_stop,
        )
        .await;
    }

    Ok(build_chat_sse(session_key, stream_rx))
}
