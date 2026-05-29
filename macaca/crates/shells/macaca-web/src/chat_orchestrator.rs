//! Chat orchestration: SSE streaming, agentic loop, workflow execution,
//! and process lifecycle (start/stop).

use std::collections::{BTreeMap, HashMap};
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

use macaca_app::{app_entry_agent_name, AppLoader};
use macaca_framework::execution::ExecutionContext;
use macaca_framework::session::{load_module_state, save_module_state};
use macaca_kernel::{executor::ExecutorEvent, AgentInfo};
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationHostDispatchServiceCommand,
    ApplicationId, ApplicationImport, ApplicationMetadataQueryCommand, ApplicationServiceScope,
    ApplicationSessionStartCommand, ApplicationSessionStopCommand, ApplicationStatusCommand,
    KernelServiceId, PackageRuntimeKind, ServiceBusSource, TraceContext,
    AGENT_EXECUTION_SERVICE_ID,
};

use crate::event_persistence::spawn_session_event_collector;
use crate::routes::{default_model, err, ErrorResponse};
use crate::runtime_event_bridge::emit_host_command_result_events;
use crate::session::{
    persist_session_snapshot, AgentTraceCollector, SessionMeta, StoredSession, StoredTurn,
    APP_SESSIONS_PREFIX, SESSION_PREFIX,
};
use crate::sse::convert_executor_event_to_sse;
use crate::state::AppState;

/// Build the normalized SSE response shape used by all chat execution paths.
///
/// Keeping one constructor avoids divergent stream concrete types between
/// framework and WASM fast-path branches and guarantees a stable first
/// `session_id` event for the frontend.
fn build_chat_sse(
    session_key: String,
    stream_rx: tokio::sync::mpsc::Receiver<Result<Event, Infallible>>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut stream_rx = stream_rx;
        yield Ok(Event::default()
            .event("session_id")
            .data(serde_json::json!({"session_id": session_key}).to_string()));
        while let Some(event) = stream_rx.recv().await {
            yield event;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn persist_execution_context(state: &Arc<AppState>, context: &ExecutionContext) {
    let _ = save_module_state(
        state.sessions.framework_session_store.as_ref(),
        &context.session_id,
        context,
    )
    .await;
}

/// Synchronize visible agent activity with a chat lifecycle edge.
///
/// The kernel status tracker remains the semantic owner of agent activity.  The
/// Web chat route only knows that it is starting or finishing a session, so this
/// adapter updates an app-declared agent by identity without inspecting any
/// application-specific payload, provider name, or business-domain data.
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

/// Project delegated executor lifecycle events into the kernel status tracker.
///
/// Delegated worker details are already persisted through EventLog and streamed
/// through SSE.  This helper keeps the coarse agent status panel consistent with
/// those same generic executor events, including WASM `macaca:agent/delegate`
/// paths that do not pass through the framework worker loop.
async fn sync_delegated_agent_activity_from_executor_event(
    state: &Arc<AppState>,
    event: &ExecutorEvent,
) {
    match event {
        ExecutorEvent::TaskStarted { task_id, agent } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Working {
                    context: format!("Executing delegated task {}", task_id),
                },
            )
            .await;
        }
        ExecutorEvent::TaskCompleted { agent, result, .. } if result.success => {
            update_agent_activity_by_name(state, agent, macaca_proto::AgentActivity::Idle).await;
        }
        ExecutorEvent::TaskCompleted { agent, result, .. } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Error {
                    message: result.output.clone(),
                },
            )
            .await;
        }
        ExecutorEvent::TaskFailed { agent, error, .. } => {
            update_agent_activity_by_name(
                state,
                agent,
                macaca_proto::AgentActivity::Error {
                    message: error.clone(),
                },
            )
            .await;
        }
        ExecutorEvent::AgentEvent { .. }
        | ExecutorEvent::TaskCancelled { .. }
        | ExecutorEvent::TaskProgress { .. }
        | ExecutorEvent::HookEvent { .. }
        | ExecutorEvent::Shutdown => {}
    }
}

/// Execute the visible chat entry agent through `service.agent_execution`.
///
/// Chat orchestration owns browser session setup, SSE stream lifetime, and
/// session persistence.  The service owns trusted context construction and
/// model/tool runtime execution, so the Web shell no longer builds a separate
/// main-thread agent for non-WASM chat sessions.
async fn run_chat_main_thread_via_agent_service(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
    prompt: String,
) -> Result<String, String> {
    let mut trace = TraceContext::new(format!(
        "chat-main-thread:{}:{}:{}",
        app_id.0, entry_agent_name, session_id
    ));
    trace.session_id = Some(session_id.to_string());
    trace.agent = Some(entry_agent_name.to_string());

    let mut command = AgentExecutionCommand::new(
        *app_id,
        session_id.to_string(),
        entry_agent_name,
        AgentExecutionIntent::ChatMainThread,
        prompt,
        trace,
    )
    .map_err(|error| error.to_string())?;
    command
        .metadata
        .insert("entrypoint".into(), "macaca.web.chat_orchestrator".into());

    let reply = state
        .service_runtime
        .call(
            &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
            ServiceBusSource::new("macaca.web.chat_orchestrator"),
            command
                .into_service_command()
                .map_err(|error| error.to_string())?,
        )
        .await
        .map_err(|error| error.to_string())?;
    let output = reply
        .output
        .ok_or_else(|| "agent execution service returned no chat output".to_string())?;
    let result: AgentExecutionResult =
        serde_json::from_value(output).map_err(|error| error.to_string())?;

    match result.status {
        AgentExecutionStatus::Completed => Ok(result
            .output
            .get("output")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| result.output.to_string())),
        status => Err(result
            .output
            .get("error")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("agent execution service returned {}", status.as_str()))),
    }
}

/// Persist the user-visible chat session envelope before any execution branch
/// starts producing SSE events.
///
/// Framework sessions and agentless WASM sessions both appear in the same
/// "Session Logs" surface, so the web shell must write the shared session
/// record and app-session index before branching into framework or WASM
/// execution. Keeping this helper at the route boundary avoids duplicating
/// persistence details inside engine-specific code paths.
async fn persist_initial_chat_session(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_key: &str,
    prompt: &str,
) {
    let now = Utc::now();
    let title = prompt.chars().take(50).collect::<String>();
    let session_key_db = format!("{}{}", SESSION_PREFIX, session_key);
    let initial_stored = StoredSession {
        meta: SessionMeta {
            session_id: session_key.to_string(),
            app_id: app_id.0.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 1,
            title: Some(title),
            status: "running".to_string(),
        },
        turns: vec![StoredTurn {
            role: "user".into(),
            content: prompt.to_string(),
            status: None,
            trace_steps: Vec::new(),
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

    // Maintain the per-app reverse index used by the left sidebar session log.
    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id.0, session_key);
    let _ = state
        .persist
        .session_store
        .set(&app_index_key, session_key.as_bytes())
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

/// Ensure one executor exists for the target application and that the executor
/// is scoped to application-declared agents only.
///
/// Why fail-closed:
/// - This API path is used by end-user chat entrypoints.
/// - If an application does not declare executable agents, silently falling
///   back to global coordinator agents would couple shell behavior to unrelated
///   framework internals and produce non-auditable execution chains.
/// - Returning an explicit error keeps behavior deterministic and debuggable.
async fn ensure_app_executor(state: &Arc<AppState>, app_id: &ApplicationId) -> Result<(), String> {
    if state.executor_registry.get(app_id).await.is_some() {
        return Ok(());
    }

    let metadata_command = ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-ensure-executor-metadata"),
        *app_id,
    );
    let (app_name, app_agent_names) = match metadata_command {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => (
                view.application.name,
                view.application
                    .agents
                    .into_iter()
                    .map(|agent| agent.name)
                    .collect::<Vec<_>>(),
            ),
            Err(error) => {
                tracing::warn!(
                    app_id = %app_id,
                    error = %error,
                    "Application metadata query failed while ensuring executor; using status fallback"
                );
                service_executor_metadata(state, app_id).await
            }
        },
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata command rejected while ensuring executor; using status fallback"
            );
            service_executor_metadata(state, app_id).await
        }
    };

    let (app_name, app_agent_names) = if app_agent_names.is_empty() {
        #[allow(deprecated)]
        let legacy = legacy_executor_metadata(state, app_id).await;
        if legacy.1.is_empty() {
            (app_name, app_agent_names)
        } else {
            legacy
        }
    } else {
        (app_name, app_agent_names)
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
    if app_agent_names.is_empty() {
        tracing::warn!(
            app_id = %app_id,
            "Application metadata exposed no app-scoped agents; refusing executor fallback"
        );
        return Err(format!(
            "application {app_id} has no executable app-scoped agents; chat execution is denied"
        ));
    }
    if app_agents.is_empty() {
        tracing::warn!(
            app_id = %app_id,
            declared_agent_count = app_agent_names.len(),
            "Application declared agents but none are currently available in kernel registry"
        );
        return Err(format!(
            "application {app_id} declared agents but none are available in kernel registry"
        ));
    }

    let _ = state
        .executor_registry
        .register_application(app_id.clone(), app_name, app_agents)
        .await;
    Ok(())
}

async fn service_executor_metadata(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> (String, Vec<String>) {
    match state
        .application_client
        .status(ApplicationStatusCommand {
            trace: TraceContext::new("web-chat-ensure-executor-status"),
            scope: ApplicationServiceScope::application(*app_id),
        })
        .await
    {
        Ok(views) => views
            .into_iter()
            .find(|view| view.id == *app_id)
            .map(|view| {
                (
                    view.name,
                    view.agents
                        .into_iter()
                        .map(|agent| agent.name)
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_else(|| (app_id.0.to_string(), Vec::new())),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application Service status failed while ensuring executor; using legacy registry fallback"
            );
            #[allow(deprecated)]
            legacy_executor_metadata(state, app_id).await
        }
    }
}

async fn legacy_executor_metadata(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> (String, Vec<String>) {
    #[allow(deprecated)]
    {
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
    }
}

async fn service_entry_agent_name(state: &Arc<AppState>, app_id: &ApplicationId) -> Option<String> {
    match ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-entry-agent-metadata"),
        *app_id,
    ) {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => return view.entry.agent_name,
            Err(error) => tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata query failed while resolving entry agent; using status fallback"
            ),
        },
        Err(error) => tracing::warn!(
            app_id = %app_id,
            error = %error,
            "Application metadata query rejected while resolving entry agent"
        ),
    }

    match state
        .application_client
        .status(ApplicationStatusCommand {
            trace: TraceContext::new("web-chat-entry-agent-status"),
            scope: ApplicationServiceScope::application(*app_id),
        })
        .await
    {
        Ok(views) => views
            .into_iter()
            .find(|view| view.id == *app_id)
            .and_then(|view| view.entry_agent),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application Service status failed while resolving entry agent"
            );
            None
        }
    }
}

/// Build a generic WASM host-dispatch command when the application is WASM-first.
///
/// This helper keeps routing app-agnostic:
/// - it relies only on sanitized metadata (`runtime_kind`, `abilities`, `agents`),
/// - it never checks app name or business identifiers,
/// - it maps chat input into neutral payload metadata for the guest export.
async fn wasm_chat_dispatch_command(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    trace: TraceContext,
    prompt: &str,
) -> Option<ApplicationHostDispatchServiceCommand> {
    let command = ApplicationMetadataQueryCommand::application(trace.clone(), *app_id).ok()?;
    let metadata = state.application_client.metadata(command).await.ok()?;
    let is_registry_wasm_runtime = is_registry_wasm_layer_app(state, app_id).await;
    let is_wasm_runtime = is_registry_wasm_runtime
        || metadata.application.runtime.runtime_kind == Some(PackageRuntimeKind::WasmComponent);
    if !is_wasm_runtime {
        return None;
    }
    let has_wasm_ability = metadata.abilities.iter().any(|ability| {
        ability.id == "ability.runtime.wasm" || ability.implementation.contains("wasm")
    });
    if !has_wasm_ability && !is_registry_wasm_runtime {
        return None;
    }

    let mut host_command = ApplicationHostCommand::with_trace(
        ApplicationImport::TraceEmit,
        wasm_chat_export_payload(prompt),
        trace.clone(),
    );
    host_command
        .metadata
        .insert("wasm.export".into(), "app:start".into());
    Some(ApplicationHostDispatchServiceCommand {
        trace,
        scope: ApplicationServiceScope::application(*app_id),
        host_command,
    })
}

/// Build the payload passed to a WASM application export from chat text.
///
/// This is intentionally schema-generic. If an app-owned coordinator supplies a
/// JSON object, the runtime preserves those typed fields for declarative
/// placeholders such as `${chat.symbol}`. Plain chat text remains plain
/// `input`; Macaca does not infer domain fields from prose.
fn wasm_chat_export_payload(prompt: &str) -> serde_json::Value {
    match serde_json::from_str::<serde_json::Value>(prompt) {
        Ok(serde_json::Value::Object(mut object)) => {
            object
                .entry("channel")
                .or_insert_with(|| serde_json::Value::String("chat".into()));
            serde_json::Value::Object(object)
        }
        _ => serde_json::json!({
            "input": prompt,
            "channel": "chat",
        }),
    }
}

/// Return whether the application declares app-scoped agents.
///
/// WASM applications with declared agents still execute through the WASM
/// runtime.  The agents are prepared as OS capabilities for task/delegation
/// imports, not as a replacement coordinator path.
async fn application_declares_agents(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    let Some(command) = ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-chat-agent-scope"),
        *app_id,
    )
    .ok() else {
        return false;
    };
    state
        .application_client
        .metadata(command)
        .await
        .map(|metadata| !metadata.application.agents.is_empty())
        .unwrap_or(false)
}

/// New-session preparation branch selected before chat execution starts.
///
/// The value is intentionally small and pure so Web tests can lock the routing
/// contract without constructing an entire `AppState`.  WASM applications are
/// always executed by the WASM host-dispatch path; declared agents only decide
/// whether OS orchestration services must be prepared for guest imports such as
/// `macaca:agent/delegate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewSessionPreparation {
    WasmOrchestrationExecutor,
    FrameworkExecutor,
    WasmHostDispatchOnly,
}

fn new_session_preparation_for_chat(
    has_wasm_dispatch: bool,
    declares_agents: bool,
) -> NewSessionPreparation {
    match (has_wasm_dispatch, declares_agents) {
        (true, true) => NewSessionPreparation::WasmOrchestrationExecutor,
        (true, false) => NewSessionPreparation::WasmHostDispatchOnly,
        (false, _) => NewSessionPreparation::FrameworkExecutor,
    }
}

/// Resolve bounded route metadata for one application execution request.
///
/// This helper is a Web-shell Adapter over the service-owned LLM route
/// strategy. It never decides provider semantics itself; it only asks the
/// shared router for the same precedence order used by framework agents and
/// stores the selected route as replayable session metadata. The prompt is not
/// passed into this function, which prevents route audit data from capturing
/// user content.
async fn resolve_request_route_metadata(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    request_model: Option<&str>,
) -> BTreeMap<String, String> {
    let app_defaults = {
        #[allow(deprecated)]
        let registry = state.registry.read().await;
        registry
            .get_app(app_id)
            .and_then(|app| app.manifest.llm_config.clone())
    };
    let selection = state
        .llm_router
        .resolve_selection(&macaca_llm::ModelSelectionRequest {
            request_model: request_model.map(str::to_owned),
            app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
            app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
            system_model: (!state.config.default_model.is_empty())
                .then_some(state.config.default_model.clone()),
            ..Default::default()
        });

    let mut metadata = BTreeMap::new();
    if let Some(model) = request_model.filter(|value| !value.trim().is_empty()) {
        metadata.insert("requested_model".into(), model.to_string());
    }
    match selection {
        Ok(selection) => {
            metadata.insert("provider_id".into(), selection.primary.provider);
            metadata.insert("model".into(), selection.primary.model);
            metadata.insert("source".into(), selection.source.into());
            metadata.insert(
                "fallback_count".into(),
                selection.fallbacks.len().to_string(),
            );
            tracing::info!(
                app_id = %app_id,
                session_id,
                source = selection.source,
                fallback_count = selection.fallbacks.len(),
                request_model_present = request_model.is_some(),
                "resolved request-level LLM route metadata"
            );
        }
        Err(error) => {
            metadata.insert("diagnostic".into(), "route_unavailable".into());
            metadata.insert("diagnostic_detail".into(), error.to_string());
            tracing::warn!(
                app_id = %app_id,
                session_id,
                error = %error,
                request_model_present = request_model.is_some(),
                "failed to resolve request-level LLM route metadata"
            );
        }
    }
    metadata
}

/// Compatibility fallback for installations where metadata runtime-kind view is
/// not yet populated but the discovered manifest layer is authoritative.
async fn is_registry_wasm_layer_app(state: &Arc<AppState>, app_id: &ApplicationId) -> bool {
    #[allow(deprecated)]
    {
        let registry = state.registry.read().await;
        registry
            .get_app(app_id)
            .map(|app| app.manifest.layer == macaca_app::AppLayer::L2Wasm)
            .unwrap_or(false)
    }
}

async fn notify_application_session_start(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
) {
    let scope = ApplicationServiceScope {
        application_id: Some(*app_id),
        application_name: None,
        session_id: Some(session_id.to_string()),
        agent_name: Some(entry_agent_name.to_string()),
    };
    let command = ApplicationSessionStartCommand {
        trace: TraceContext::new("web-chat-session-start"),
        scope,
    };
    match state.application_client.session_start(command).await {
        Ok(view) => tracing::info!(
            app_id = %view.application_id,
            session_id = %view.session_id,
            status = %view.status,
            "Application Service session envelope started"
        ),
        Err(error) => tracing::warn!(
            app_id = %app_id,
            session_id,
            error = %error,
            "Application Service session start failed; continuing coordinator path"
        ),
    }
}

async fn notify_application_session_stop(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: &str,
    entry_agent_name: &str,
    reason: &str,
) {
    let scope = ApplicationServiceScope {
        application_id: Some(*app_id),
        application_name: None,
        session_id: Some(session_id.to_string()),
        agent_name: Some(entry_agent_name.to_string()),
    };
    let command = ApplicationSessionStopCommand {
        trace: TraceContext::new("web-chat-session-stop"),
        scope,
        reason: Some(reason.to_string()),
    };
    if let Err(error) = state.application_client.session_stop(command).await {
        tracing::warn!(
            app_id = %app_id,
            session_id,
            error = %error,
            "Application Service session stop failed after coordinator completion"
        );
    }
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
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    // Determine entry agent
    let entry_agent_name = if let Some(name) = service_entry_agent_name(&state, &app_id).await {
        name
    } else {
        #[allow(deprecated)]
        {
            let registry = state.registry.read().await;
            registry
                .get_app(&app_id)
                .map(|app| {
                    app_entry_agent_name(&app.manifest)
                        .unwrap_or("coordinator")
                        .to_string()
                })
                .unwrap_or_else(|| "coordinator".to_string())
        }
    };

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
    let request_route_metadata = resolve_request_route_metadata(
        &state,
        &app_id,
        &session_key,
        request_model_hint.as_deref(),
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
                    Err(error) if is_registry_wasm_layer_app(&state, &app_id).await => {
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
    if let Some(mut dispatch) = wasm_dispatch {
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
            let mut exec_rx = executor.subscribe_to_events();
            let sse_tx_for_exec = Arc::clone(&sse_tx);
            let forwarder_stop_flag = Arc::clone(&forwarder_stop);
            let state_for_exec = Arc::clone(&state);
            tokio::spawn(async move {
                loop {
                    match exec_rx.recv().await {
                        Ok(exec_event) => {
                            if forwarder_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                tracing::debug!(
                                    "WASM executor event forwarder stopped (superseded)"
                                );
                                break;
                            }
                            sync_delegated_agent_activity_from_executor_event(
                                &state_for_exec,
                                &exec_event,
                            )
                            .await;
                            let sse_event = convert_executor_event_to_sse(exec_event);
                            let sender = sse_tx_for_exec.read().await;
                            if sender.send(sse_event).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(
                                "WASM executor event forwarder lagged by {} messages, continuing",
                                n
                            );
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
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
            {
                let mut flags = state_for_task.sessions.cancel_flags.write().await;
                flags.remove(&app_id_for_task.0.to_string());
            }
        });

        // Bridge task: forward producer events (rx) -> hot-swappable sse_tx -> stream_rx
        let sse_tx_for_bridge = Arc::clone(&sse_tx);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let sender = sse_tx_for_bridge.read().await;
                if sender.send(event).await.is_err() {
                    break;
                }
            }
        });

        return Ok(build_chat_sse(session_key, stream_rx));
    }

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
        let _ = state_for_task
            .mcp_runtime
            .cleanup_session(&session_key_for_task)
            .await;
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

    // Executor events forwarder: delegated agent traces → SSE stream
    // (EventLog persistence is handled by spawn_session_event_collector above)
    if let Some(mut exec_rx) = executor_events_rx {
        let sse_tx_for_exec = Arc::clone(&sse_tx);
        let forwarder_stop_flag = Arc::clone(&forwarder_stop);
        let state_for_exec = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                match exec_rx.recv().await {
                    Ok(exec_event) => {
                        // Check if this forwarder has been superseded
                        if forwarder_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            tracing::debug!("Executor event forwarder stopped (superseded)");
                            break;
                        }
                        sync_delegated_agent_activity_from_executor_event(
                            &state_for_exec,
                            &exec_event,
                        )
                        .await;
                        // Forward to SSE after status synchronization.
                        let sse_event = convert_executor_event_to_sse(exec_event);
                        let sender = sse_tx_for_exec.read().await;
                        if sender.send(sse_event).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            "Executor event forwarder lagged by {} messages, continuing",
                            n
                        );
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    Ok(build_chat_sse(session_key, stream_rx))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        new_session_preparation_for_chat, wasm_chat_export_payload, NewSessionPreparation,
    };

    #[test]
    fn wasm_with_declared_agents_prepares_orchestration_executor() {
        assert_eq!(
            new_session_preparation_for_chat(true, true),
            NewSessionPreparation::WasmOrchestrationExecutor
        );
    }

    #[test]
    fn agentless_wasm_keeps_host_dispatch_only() {
        assert_eq!(
            new_session_preparation_for_chat(true, false),
            NewSessionPreparation::WasmHostDispatchOnly
        );
    }

    #[test]
    fn agentless_wasm_updates_entry_agent_activity_without_app_specific_branch() {
        let source = include_str!("chat_orchestrator.rs");

        assert!(source.contains("update_agent_activity_by_name"));
        assert!(source.contains("sync_delegated_agent_activity_from_executor_event"));
        assert!(source.contains("Handling WASM session"));
        assert!(source.contains("macaca_proto::AgentActivity::Working"));
        assert!(source.contains("macaca_proto::AgentActivity::Idle"));
        assert!(source.contains("macaca_proto::AgentActivity::Error"));
        let app_specific_name = ["wasm-crypto", "-signal-app"].concat();
        assert!(!source.contains(&app_specific_name));
    }

    #[test]
    fn non_wasm_chat_uses_framework_executor() {
        assert_eq!(
            new_session_preparation_for_chat(false, true),
            NewSessionPreparation::FrameworkExecutor
        );
        assert_eq!(
            new_session_preparation_for_chat(false, false),
            NewSessionPreparation::FrameworkExecutor
        );
    }

    #[test]
    fn wasm_chat_payload_preserves_app_owned_typed_fields() {
        let payload = wasm_chat_export_payload(r#"{"input":"Analyze BTC/USDT","symbol":"BTC"}"#);

        assert_eq!(payload["input"], json!("Analyze BTC/USDT"));
        assert_eq!(payload["symbol"], json!("BTC"));
        assert_eq!(payload["channel"], json!("chat"));
    }

    #[test]
    fn wasm_chat_payload_keeps_plain_prompt_untyped() {
        let payload = wasm_chat_export_payload("Analyze BTC/USDT");

        assert_eq!(payload["input"], json!("Analyze BTC/USDT"));
        assert!(payload.get("symbol").is_none());
        assert_eq!(payload["channel"], json!("chat"));
    }

    #[test]
    fn chat_main_thread_enters_agent_execution_service_boundary() {
        let source = include_str!("chat_orchestrator.rs");
        let legacy_builder = ["FrameworkRunner::build_", "coordinator"].concat();

        assert!(source.contains("run_chat_main_thread_via_agent_service"));
        assert!(source.contains("AgentExecutionIntent::ChatMainThread"));
        assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(!source.contains(&legacy_builder));
    }

    #[test]
    fn chat_done_event_is_after_session_memory_capture_attempt() {
        let source = include_str!("chat_orchestrator.rs");
        let capture = "capture_successful_session_completion";
        let done = r#".event("done")"#;
        let capture_index = source
            .find(capture)
            .expect("chat completion should attempt sanitized memory capture");
        let done_index = source[capture_index..]
            .find(done)
            .expect("chat completion should emit done after memory capture")
            + capture_index;

        assert!(
            capture_index < done_index,
            "the public done event must remain a durable boundary after memory capture"
        );
    }
}
