//! API route handlers for the Agent OS web server.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use macaca_kernel::executor::ExecutorEvent;
use macaca_persist::{PersistStore, RedbStore};
use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, AgentId};
use macaca_sdk::AgentPersona;
use macaca_tools::TraceEvent;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// LLM Error Diagnosis Helper
// ---------------------------------------------------------------------------

/// Diagnose LLM errors and provide actionable feedback.
fn diagnose_llm_error(err: &macaca_proto::MacacaError) -> String {
    let err_str = err.to_string();

    // Check for network errors
    if err_str.contains("error sending request") || err_str.contains("connection") {
        return format!(
            "Network error: Unable to reach LLM API. Check:\n\
             1. Internet connection\n\
             2. Firewall/proxy settings\n\
             3. API endpoint accessibility\n\
             Original: {err_str}"
        );
    }

    // Check for API key issues
    if err_str.contains("401") || err_str.contains("unauthorized")
        || err_str.contains("authentication") || err_str.contains("API key")
        || err_str.contains("not set") {
        return format!(
            "Authentication error: Invalid or missing API key.\n\
             Check DASHSCOPE_API_KEY environment variable.\n\
             Original: {err_str}"
        );
    }

    // Check for rate limiting
    if err_str.contains("429") || err_str.contains("rate limit") || err_str.contains("quota") {
        return format!(
            "Rate limit exceeded: API quota or rate limit reached.\n\
             Wait before retrying or check API usage.\n\
             Original: {err_str}"
        );
    }

    // Check for invalid request
    if err_str.contains("400") || err_str.contains("bad request") {
        return format!(
            "Invalid request: Malformed request to LLM.\n\
             Check model name and request format.\n\
             Original: {err_str}"
        );
    }

    // Check for server errors
    if err_str.contains("500") || err_str.contains("502") || err_str.contains("503")
        || err_str.contains("server error") {
        return format!(
            "Server error: LLM provider server issue.\n\
             Try again later.\n\
             Original: {err_str}"
        );
    }

    // Check for timeout
    if err_str.contains("timeout") || err_str.contains("timed out") {
        return format!(
            "Request timeout: LLM took too long to respond.\n\
             Check network latency or try a faster model.\n\
             Original: {err_str}"
        );
    }

    // Default: return original error with context
    format!("LLM call failed: {err_str}")
}

// ---------------------------------------------------------------------------
// GET /api/status
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub agent_count: usize,
    pub app_count: usize,
    pub llm_provider: String,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let agent_count = state.kernel.agent_count().await;
    let apps = state.runtime.list_apps().await;

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").into(),
        agent_count,
        app_count: apps.len(),
        llm_provider: state.llm.name().into(),
    })
}

// ---------------------------------------------------------------------------
// GET /api/apps
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub agent_count: usize,
    pub description: String,
    pub icon: String,
}

pub async fn get_apps(State(state): State<Arc<AppState>>) -> Json<Vec<AppInfo>> {
    let apps = state.runtime.list_apps().await;
    let agent_count = state.kernel.agent_count().await;
    let registry = state.registry.read().await;

    let infos = apps
        .into_iter()
        .map(|(id, name, status)| {
            // Try to get description from registry (app.yaml)
            let (description, icon) = registry.get_app_by_name(&name)
                .map(|app| {
                    let desc = app.manifest.description
                        .as_deref()
                        .unwrap_or("An Agent OS application.");
                    (desc.to_string(), "cube".to_string())
                })
                .unwrap_or_else(|| ("An Agent OS application.".to_string(), "cube".to_string()));

            AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count,
                description,
                icon,
            }
        })
        .collect();
    Json(infos)
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id — Get single app info
// ---------------------------------------------------------------------------

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<AppInfo>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid app_id".into() })))?;
    let app_id = macaca_proto::ApplicationId(app_uuid);

    // Get description from registry
    let description = {
        let registry = state.registry.read().await;
        registry
            .get_app(&app_id)
            .map(|a| a.manifest.description.clone().unwrap_or_else(|| "An Agent OS application.".to_string()))
            .unwrap_or_else(|| "An Agent OS application.".to_string())
    };
    let icon = "cube".to_string();

    let apps = state.runtime.list_apps().await;
    for (id, name, status) in apps {
        if id == app_id {
            return Ok(Json(AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count: state.kernel.agent_count().await,
                description,
                icon,
            }));
        }
    }

    Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: "App not found".into() })))
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/agents — Get agents for an app
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    /// Current activity (what the agent is doing right now).
    pub activity: AgentActivityInfo,
    pub capabilities: Vec<String>,
    pub is_active: bool,
    /// Current task description (if any).
    pub current_task: Option<String>,
}

/// Serializable agent activity info.
#[derive(Serialize)]
pub struct AgentActivityInfo {
    /// Activity type: idle, thinking, executing_tool, waiting, error.
    pub r#type: String,
    /// Additional context (tool name, thinking context, etc.).
    pub context: Option<String>,
    /// Secondary context (tool purpose, wait reason, etc.).
    pub detail: Option<String>,
}

impl From<macaca_proto::AgentActivity> for AgentActivityInfo {
    fn from(activity: macaca_proto::AgentActivity) -> Self {
        match activity {
            macaca_proto::AgentActivity::Idle => Self {
                r#type: "idle".into(),
                context: None,
                detail: None,
            },
            macaca_proto::AgentActivity::Working { context } => Self {
                r#type: "working".into(),
                context: Some(context),
                detail: None,
            },
            macaca_proto::AgentActivity::Error { message } => Self {
                r#type: "error".into(),
                context: Some(message),
                detail: None,
            },
            macaca_proto::AgentActivity::Thinking { context } => Self {
                r#type: "thinking".into(),
                context: Some(context),
                detail: None,
            },
        }
    }
}

pub async fn get_app_agents(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<Vec<AgentInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid app_id".into() })))?;
    let app_id = macaca_proto::ApplicationId(app_uuid);

    // Get agent IDs for this app
    let agent_ids = state.runtime.app_agents(&app_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: e.to_string() })))?;

    // Get manifests from kernel
    let manifests = state.kernel.list_agents().await;

    // Get runtime statuses
    let statuses = state.kernel.list_agent_statuses_for(&agent_ids).await;
    let status_map: std::collections::HashMap<String, _> = statuses
        .into_iter()
        .map(|s| (s.agent_id.0.to_string(), s))
        .collect();

    let agents: Vec<AgentInfo> = manifests
        .into_iter()
        .filter(|m| agent_ids.contains(&m.id))
        .map(|m| {
            let id_str = m.id.0.to_string();
            let (activity, current_task) = status_map.get(&id_str)
                .map(|s| (s.activity.clone().into(), s.current_task.clone()))
                .unwrap_or_else(|| (macaca_proto::AgentActivity::Idle.into(), None));

            AgentInfo {
                id: id_str,
                name: m.name.clone(),
                state: format!("{:?}", m.state),
                activity,
                capabilities: m.capabilities.into_iter().map(|c| c.name).collect(),
                is_active: m.state == macaca_proto::AgentState::Running,
                current_task,
            }
        })
        .collect();

    Ok(Json(agents))
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/agents/stream — SSE stream of agent status updates
// ---------------------------------------------------------------------------

/// Simplified agent status for frontend (IDLE, WORKING, ERROR)
#[derive(Serialize, Clone)]
pub struct SimpleAgentStatus {
    pub id: String,
    pub name: String,
    pub status: String, // "IDLE" | "WORKING" | "ERROR"
    pub detail: Option<String>,
}

pub async fn stream_agent_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    let app_uuid_result: Result<uuid::Uuid, _> = app_id.parse();
    let state_clone = Arc::clone(&state);

    let stream = async_stream::stream! {
        // Handle parse error inside the stream
        let app_uuid = match app_uuid_result {
            Ok(u) => u,
            Err(_) => {
                yield Ok(Event::default().data(r#"{"error":"Invalid app_id"}"#));
                return;
            }
        };
        let app_id = macaca_proto::ApplicationId(app_uuid);

        loop {
            // Get agent IDs for this app
            let agent_ids = match state_clone.runtime.app_agents(&app_id).await {
                Ok(ids) => ids,
                Err(_) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(r#"{"error":"App not found"}"#));
                    return;
                }
            };

            // Get manifests
            let manifests = state_clone.kernel.list_agents().await;
            let statuses = state_clone.kernel.list_agent_statuses_for(&agent_ids).await;
            let status_map: std::collections::HashMap<String, _> = statuses
                .into_iter()
                .map(|s| (s.agent_id.0.to_string(), s))
                .collect();

            // Build simplified status
            let agents: Vec<SimpleAgentStatus> = manifests
                .into_iter()
                .filter(|m| agent_ids.contains(&m.id))
                .map(|m| {
                    let id_str = m.id.0.to_string();
                    let (status, detail) = status_map.get(&id_str)
                        .map(|s| {
                            let (st, det) = match &s.activity {
                                macaca_proto::AgentActivity::Idle => ("IDLE".to_string(), None),
                                macaca_proto::AgentActivity::Working { context } => ("WORKING".to_string(), Some(context.clone())),
                                macaca_proto::AgentActivity::Thinking { context } => ("THINKING".to_string(), Some(context.clone())),
                                macaca_proto::AgentActivity::Error { message } => ("ERROR".to_string(), Some(message.clone())),
                            };
                            (st, det)
                        })
                        .unwrap_or(("IDLE".to_string(), None));

                    SimpleAgentStatus {
                        id: id_str,
                        name: m.name,
                        status,
                        detail,
                    }
                })
                .collect();

            let json = serde_json::to_string(&agents).unwrap_or_else(|_| "[]".to_string());
            yield Ok(Event::default().data(json));

            // Wait 500ms before next update
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    };

    Sse::new(stream)
}

// ---------------------------------------------------------------------------
// POST /api/apps/reload — Reload apps from disk
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ReloadResponse {
    pub discovered_count: usize,
    pub apps: Vec<AppInfo>,
}

pub async fn reload_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Reload the registry
    let mut registry = state.registry.write().await;
    let discovered = registry.reload()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to reload apps: {e}")))?;

    // Build app info from discovered apps
    let apps = state.runtime.list_apps().await;
    let agent_count = state.kernel.agent_count().await;

    let registry = state.registry.read().await;
    let app_infos: Vec<AppInfo> = apps
        .into_iter()
        .map(|(id, name, status)| {
            let (description, icon) = registry
                .get_app_by_name(&name)
                .map(|app| {
                    let desc = app.manifest.description
                        .as_deref()
                        .unwrap_or("An Agent OS application.");
                    (desc.to_string(), "cube".to_string())
                })
                .unwrap_or_else(|| ("An Agent OS application.".to_string(), "cube".to_string()));
            AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count,
                description,
                icon,
            }
        })
        .collect();
    drop(registry);

    Ok(Json(ReloadResponse {
        discovered_count: discovered.len(),
        apps: app_infos,
    }))
}

// ---------------------------------------------------------------------------
// GET /api/sessions/:app_id — Get session history
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExecutionTokens {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssistantExecutionMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<ExecutionTokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterations: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools_used: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredTraceStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Individual step in a delegated agent's trace.
/// Generic structure that works for any agent, not hardcoded to specific names.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AgentTraceStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteration: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    // For cc_trace type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Trace for a single delegated agent execution.
/// task_id uniquely identifies this specific task execution.
#[derive(Serialize, Deserialize, Clone)]
pub struct AgentTrace {
    pub task_id: String,
    pub agent: String,
    pub status: String, // "running" | "completed" | "error"
    #[serde(default)]
    pub steps: Vec<AgentTraceStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Agent Trace Collector for SSE Stream
// ---------------------------------------------------------------------------

/// Collects agent traces during SSE stream processing.
/// Shared between SSE stream and session saving.
pub struct AgentTraceCollector {
    traces: RwLock<std::collections::HashMap<String, Vec<AgentTrace>>>,
    /// Maps task_id to agent name for looking up agent when TaskCompleted/TaskFailed is received
    task_to_agent: RwLock<std::collections::HashMap<String, String>>,
}

impl AgentTraceCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            traces: RwLock::new(std::collections::HashMap::new()),
            task_to_agent: RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Called when executor emits TaskStarted - creates new trace
    pub async fn on_task_started(&self, task_id: &str, agent: &str) {
        tracing::debug!(task_id = %task_id, agent = %agent, "AgentTraceCollector: on_task_started called");
        let mut traces = self.traces.write().await;
        let mut task_to_agent = self.task_to_agent.write().await;

        // Store task_id -> agent mapping
        task_to_agent.insert(task_id.to_string(), agent.to_string());

        let agent_traces = traces.entry(agent.to_string()).or_insert_with(Vec::new);
        agent_traces.push(AgentTrace {
            task_id: task_id.to_string(),
            agent: agent.to_string(),
            status: "running".to_string(),
            steps: Vec::new(),
            output: None,
            error: None,
        });
        tracing::debug!(task_id = %task_id, agent = %agent, trace_count = %agent_traces.len(), "AgentTraceCollector: trace created");
    }

    /// Called when executor emits AgentEvent - adds step to trace
    pub async fn on_agent_event(&self, task_id: &str, agent: &str, event: &macaca_proto::AgentExecutionEvent) {
        tracing::debug!(task_id = %task_id, agent = %agent, event_type = ?std::mem::discriminant(event), "AgentTraceCollector: on_agent_event called");
        let mut traces = self.traces.write().await;
        if let Some(agent_traces) = traces.get_mut(agent) {
            if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                tracing::debug!(task_id = %task_id, agent = %agent, "AgentTraceCollector: found trace, adding step");
                let step = match event {
                    macaca_proto::AgentExecutionEvent::Thinking { iteration, content } => {
                        AgentTraceStep {
                            step_type: "thinking".to_string(),
                            iteration: Some(*iteration),
                            content: content.clone(),
                            ..Default::default()
                        }
                    }
                    macaca_proto::AgentExecutionEvent::ToolCall { tool_name, tool_input, .. } => {
                        AgentTraceStep {
                            step_type: "tool_call".to_string(),
                            tool_name: Some(tool_name.clone()),
                            tool_input: Some(tool_input.clone()),
                            ..Default::default()
                        }
                    }
                    macaca_proto::AgentExecutionEvent::ToolResult { tool_name, output, is_error } => {
                        crate::metrics::record_tool_execution(tool_name, !is_error.unwrap_or(false));
                        AgentTraceStep {
                            step_type: "tool_result".to_string(),
                            tool_name: Some(tool_name.clone()),
                            output: Some(output.clone()),
                            is_error: is_error.clone(),
                            ..Default::default()
                        }
                    }
                    macaca_proto::AgentExecutionEvent::Assistant { content } => {
                        AgentTraceStep {
                            step_type: "assistant".to_string(),
                            content: Some(content.clone()),
                            ..Default::default()
                        }
                    }
                    macaca_proto::AgentExecutionEvent::CcTrace { thinking, text, tool_name, tool_input, tool_result, is_error } => {
                        AgentTraceStep {
                            step_type: "cc_trace".to_string(),
                            thinking: thinking.clone(),
                            text: text.clone(),
                            tool_name: tool_name.clone(),
                            tool_input: tool_input.clone(),
                            tool_result: tool_result.clone(),
                            is_error: is_error.clone(),
                            ..Default::default()
                        }
                    }
                    macaca_proto::AgentExecutionEvent::Completed { success, error } => {
                        AgentTraceStep {
                            step_type: "completed".to_string(),
                            success: Some(*success),
                            error: error.clone(),
                            ..Default::default()
                        }
                    }
                };
                trace.steps.push(step);
            }
        }
    }

    /// Called when executor emits TaskCompleted/TaskFailed - update trace status
    /// Note: TaskCompleted/TaskFailed don't have agent field, so we look it up from task_to_agent mapping
    pub async fn on_task_completed(&self, task_id: &str, success: bool, output: Option<String>, error: Option<String>) {
        let agent = {
            let task_to_agent = self.task_to_agent.read().await;
            task_to_agent.get(task_id).cloned()
        };

        if let Some(agent) = agent {
            let mut traces = self.traces.write().await;
            if let Some(agent_traces) = traces.get_mut(&agent) {
                if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                    trace.status = if success { "completed".to_string() } else { "error".to_string() };
                    trace.output = output;
                    trace.error = error;
                }
            }
        }
    }

    /// Get all collected traces for session storage
    pub async fn get_all(&self) -> std::collections::HashMap<String, Vec<AgentTrace>> {
        self.traces.read().await.clone()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredTurn {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace_steps: Vec<StoredTraceStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cc_trace_steps: Vec<TraceEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<AssistantExecutionMeta>,
    /// Delegated agent traces keyed by agent name.
    /// This is a generic structure that works for any application with any number of agents.
    /// Key: agent name (e.g., "backend", "frontend", "tester" - dynamic, not hardcoded)
    /// Value: array of traces for that agent (supports multiple task executions per agent)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub agent_traces: std::collections::HashMap<String, Vec<AgentTrace>>,
}

fn ensure_running_assistant_turn(turns: &mut Vec<StoredTurn>) -> &mut StoredTurn {
    let has_running_idx = turns.iter().rposition(|turn| {
        turn.role == "assistant"
            && matches!(turn.status.as_deref(), Some("running") | Some("pending"))
    });

    let idx = match has_running_idx {
        Some(idx) => idx,
        None => {
            turns.push(StoredTurn {
                role: "assistant".into(),
                content: String::new(),
                status: Some("running".into()),
                trace_steps: Vec::new(),
                cc_trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            });
            turns.len() - 1
        }
    };

    &mut turns[idx]
}

fn session_status_from_executor_event(event: &ExecutorEvent) -> Option<&'static str> {
    match event {
        ExecutorEvent::TaskStarted { .. } => Some("running"),
        ExecutorEvent::TaskCompleted { .. } => Some("completed"),
        ExecutorEvent::TaskFailed { .. } => Some("failed"),
        ExecutorEvent::TaskCancelled { .. } => Some("cancelled"),
        ExecutorEvent::HookEvent { event: hook_event } => {
            use macaca_kernel::executor::fork_manager::HookEvent;
            match hook_event {
                HookEvent::ForkMerged { .. } => Some("completed"),
                HookEvent::DelegateFailed { .. } => Some("failed"),
                HookEvent::DelegateCompleted { .. } => Some("running"),
                HookEvent::ForkCreated { .. } => Some("running"),
                HookEvent::ForkValidated { .. } => Some("running"),
                _ => None,
            }
        }
        _ => None,
    }
}

async fn persist_session_snapshot(
    store: &Arc<RedbStore>,
    session_id: &str,
    app_id: &ApplicationId,
    status: Option<&str>,
    content: Option<String>,
    trace_steps: Option<Vec<StoredTraceStep>>,
    cc_trace_steps: Option<Vec<TraceEvent>>,
    agent_traces: Option<std::collections::HashMap<String, Vec<AgentTrace>>>,
    meta: Option<AssistantExecutionMeta>,
) {
    // Acquire per-session write lock to prevent concurrent read-modify-write
    // from overwriting each other's data (e.g., periodic saver vs snapshot closure).
    // We use a simple approach: lock the session_key_db string as a file-level mutex.
    static SESSION_LOCKS: std::sync::OnceLock<tokio::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = std::sync::OnceLock::new();
    let locks = SESSION_LOCKS.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let lock = {
        let mut map = locks.lock().await;
        map.entry(session_id.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
    };
    let _guard = lock.lock().await;

    let session_key_db = format!("{}{}", SESSION_PREFIX, session_id);
    let existing = match store.get(&session_key_db).await {
        Ok(Some(data)) => match serde_json::from_slice::<StoredSession>(&data) {
            Ok(s) => s,
            Err(_) => return,
        },
        _ => return,
    };

    let mut turns = stored_turns_or_messages(&existing);
    let mut next_status = existing.meta.status.clone();

    if let Some(status) = status {
        next_status = status.to_string();
        let turn = ensure_running_assistant_turn(&mut turns);
        turn.status = Some(status.to_string());
        if let Some(content) = content {
            turn.content = content;
        }
        if let Some(trace_steps) = trace_steps {
            turn.trace_steps = trace_steps;
        }
        if let Some(cc_trace_steps) = cc_trace_steps {
            turn.cc_trace_steps = cc_trace_steps;
        }
        if let Some(agent_traces) = agent_traces {
            turn.agent_traces = agent_traces;
        }
        if let Some(meta) = meta {
            turn.meta = Some(meta);
        }
    }

    let stored = StoredSession {
        meta: SessionMeta {
            session_id: session_id.to_string(),
            app_id: app_id.0.to_string(),
            created_at: existing.meta.created_at,
            updated_at: Utc::now(),
            message_count: existing.meta.message_count,
            title: existing.meta.title,
            status: next_status,
        },
        messages: existing.messages,
        turns,
    };

    if let Ok(data) = serde_json::to_vec(&stored) {
        let _ = store.set(&session_key_db, &data).await;
    }
}


#[derive(Serialize)]
pub struct SessionResponse {
    pub app_id: String,
    pub messages: Vec<SessionMessage>,
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Json<SessionResponse> {
    let sessions = state.sessions.read().await;
    let messages = sessions
        .get(&app_id)
        .map(|hist| {
            hist.iter()
                .filter_map(|msg| {
                    // Only include user and assistant messages (not system/tool)
                    match msg.role {
                        macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => Some(SessionMessage {
                            role: format!("{:?}", msg.role).to_lowercase(),
                            content: msg.content.clone(),
                        }),
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Json(SessionResponse {
        app_id,
        messages,
    })
}

fn build_turns_from_messages(messages: &[LlmMessage]) -> Vec<StoredTurn> {
    messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User => Some(StoredTurn {
                role: "user".into(),
                content: msg.content.clone(),
                status: None,
                trace_steps: Vec::new(),
                cc_trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            macaca_proto::LlmRole::Assistant => Some(StoredTurn {
                role: "assistant".into(),
                content: msg.content.clone(),
                status: Some("completed".into()),
                trace_steps: Vec::new(),
                cc_trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            _ => None,
        })
        .collect()
}

fn stored_turns_or_messages(stored: &StoredSession) -> Vec<StoredTurn> {
    if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    }
}

struct AssistantRunResult {
    pub content: String,
    usage: (u32, u32, u32),
    iterations: usize,
    tools_used: Vec<String>,
    status: String,
    trace_steps: Vec<StoredTraceStep>,
    cc_trace_steps: Vec<TraceEvent>,
    /// Delegated agent traces collected during execution.
    /// Key: agent name (dynamic, not hardcoded)
    /// Value: array of traces for that agent
    agent_traces: std::collections::HashMap<String, Vec<AgentTrace>>,
}

// ---------------------------------------------------------------------------
// Persistent Session Storage Types
// ---------------------------------------------------------------------------

/// Session metadata for listing and indexing.
#[derive(Serialize, Deserialize, Clone)]
pub struct SessionMeta {
    pub session_id: String,
    pub app_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub title: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "draft".to_string()
}

/// Full session with messages for storage.
#[derive(Serialize, Deserialize)]
struct StoredSession {
    meta: SessionMeta,
    messages: Vec<LlmMessage>,
    #[serde(default)]
    turns: Vec<StoredTurn>,
}

// Key prefixes for redb storage
const SESSION_PREFIX: &str = "session/";
const APP_SESSIONS_PREFIX: &str = "app_sessions/";
/// Separate key prefix for agent traces — stored independently from session
/// to avoid read-modify-write races with session updates.
const AGENT_TRACES_PREFIX: &str = "agent_traces/";
/// Separate key prefix for plan decision events — stored independently per session.
const PLAN_DECISIONS_PREFIX: &str = "plan_decisions/";

/// Save agent traces to a dedicated key (simple overwrite, no read-modify-write).
async fn save_agent_traces(
    store: &Arc<RedbStore>,
    session_id: &str,
    traces: std::collections::HashMap<String, Vec<AgentTrace>>,
) {
    if traces.is_empty() {
        return;
    }
    let key = format!("{}{}", AGENT_TRACES_PREFIX, session_id);
    if let Ok(data) = serde_json::to_vec(&traces) {
        let _ = store.set(&key, &data).await;
    }
}

/// Load agent traces from the dedicated key.
async fn load_agent_traces(
    store: &Arc<RedbStore>,
    session_id: &str,
) -> std::collections::HashMap<String, Vec<AgentTrace>> {
    let key = format!("{}{}", AGENT_TRACES_PREFIX, session_id);
    match store.get(&key).await {
        Ok(Some(data)) => {
            serde_json::from_slice(&data).unwrap_or_default()
        }
        _ => std::collections::HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Plan Decision Event Persistence
// ---------------------------------------------------------------------------

/// A single decision event emitted by the PlanLoop or WorkerLoop.
/// Stored independently per session to avoid read-modify-write races.
#[derive(Serialize, Deserialize, Clone)]
pub struct PlanDecisionEvent {
    pub decision_type: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
}

/// Append a plan decision to its dedicated per-session key (read-append-write).
async fn save_plan_decision(
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
async fn load_plan_decisions(
    store: &Arc<RedbStore>,
    app_id: &ApplicationId,
) -> Vec<PlanDecisionEvent> {
    let key = format!("{}{}", PLAN_DECISIONS_PREFIX, app_id);
    match store.get(&key).await {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Real-time Session Update Helper
// ---------------------------------------------------------------------------

/// Update session status in real-time based on ExecutorEvent.
/// This is called during SSE streaming to keep session status up-to-date.
async fn update_session_realtime(
    store: &Arc<RedbStore>,
    session_id: &str,
    app_id: &ApplicationId,
    event: &ExecutorEvent,
) {
    let Some(status) = session_status_from_executor_event(event) else {
        return;
    };

    // Note: agent_traces are NOT written here to avoid overwriting
    // the periodic saver's data. The collector + periodic saver is
    // the single source of truth for agent traces.
    persist_session_snapshot(
        store,
        session_id,
        app_id,
        Some(status),
        None,
        None,
        None,
        None, // agent_traces: None — let periodic saver handle it
        None,
    ).await;
}


// ---------------------------------------------------------------------------
// GET /api/sessions — List all persistent sessions
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SessionListItem {
    pub session_id: String,
    pub app_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub title: Option<String>,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Query parameters for list_sessions
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct SessionListQuery {
    #[serde(default)]
    status: Option<String>,
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    let keys = state.session_store
        .list_keys(SESSION_PREFIX)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list sessions: {e}")))?;

    let mut sessions = Vec::new();
    for key in keys {
        if let Some(data) = state.session_store
            .get(&key)
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get session: {e}")))?
        {
            if let Ok(stored) = serde_json::from_slice::<StoredSession>(&data) {
                sessions.push(SessionListItem {
                    session_id: stored.meta.session_id,
                    app_id: stored.meta.app_id,
                    created_at: stored.meta.created_at,
                    updated_at: stored.meta.updated_at,
                    message_count: stored.meta.message_count,
                    title: stored.meta.title,
                    status: stored.meta.status,
                });
            }
        }
    }

    // Sort by updated_at descending
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(Json(sessions))
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/sessions — List sessions for a specific app
// ---------------------------------------------------------------------------

pub async fn list_app_sessions(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    // Get all session metadata keys for this app
    let prefix = format!("{}{}/", APP_SESSIONS_PREFIX, app_id);
    let keys = state.session_store
        .list_keys(&prefix)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list app sessions: {e}")))?;

    let mut sessions = Vec::new();
    for key in keys {
        // key format: app_sessions/{app_id}/{session_id}
        if let Some(session_id) = key.strip_prefix(&prefix) {
            let session_key = format!("{}{}", SESSION_PREFIX, session_id);
            if let Some(data) = state.session_store
                .get(&session_key)
                .await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get session: {e}")))?
            {
                if let Ok(stored) = serde_json::from_slice::<StoredSession>(&data) {
                    sessions.push(SessionListItem {
                        session_id: stored.meta.session_id,
                        app_id: stored.meta.app_id,
                        created_at: stored.meta.created_at,
                        updated_at: stored.meta.updated_at,
                        message_count: stored.meta.message_count,
                        title: stored.meta.title,
                        status: stored.meta.status,
                    });
                }
            }
        }
    }

    // Sort by updated_at descending
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(Json(sessions))
}

// ---------------------------------------------------------------------------
// GET /api/sessions/:session_id — Get a specific persistent session
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SessionDetail {
    pub session_id: String,
    pub app_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<SessionMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub turns: Vec<StoredTurn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_decisions: Vec<PlanDecisionEvent>,
    /// URL to fetch events from the EventLog for this session.
    pub events_url: String,
    /// Total number of events persisted in EventLog for this session.
    pub events_count: usize,
}

pub async fn get_session_by_id(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<SessionDetail>, (StatusCode, Json<ErrorResponse>)> {
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state.session_store
        .get(&key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get session: {e}")))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse session: {e}")))?;

    // Also populate in-memory cache so subsequent messages continue the conversation
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), stored.messages.clone());
    }

    let messages = stored.messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => Some(SessionMessage {
                role: format!("{:?}", msg.role).to_lowercase(),
                content: msg.content.clone(),
            }),
            _ => None,
        })
        .collect();

    let mut turns = if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    };

    // Rebuild agent traces from EventLog (the authoritative source).
    // Group delegated_* events by agent to construct per-agent trace histories.
    // Rebuild agent traces from EventLog (the authoritative, durable source).
    // EventLog is written by the independent event_collector_handle task,
    // which survives browser disconnects. This always overwrites any
    // stored turns' agent_traces since EventLog is the single source of truth.
    {
        let events = state.event_log.query(&session_id, 0, 10000).await;
        let mut agent_traces: std::collections::HashMap<String, Vec<AgentTrace>> = std::collections::HashMap::new();
        let mut task_to_agent: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for event in &events {
            let payload = &event.payload;
            let agent = payload.get("agent").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let task_id = payload.get("task_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            match event.event_type.as_str() {
                "delegated_task_start" => {
                    if agent.is_empty() || task_id.is_empty() { continue; }
                    task_to_agent.insert(task_id.clone(), agent.clone());
                    let trace = AgentTrace {
                        task_id: task_id.clone(),
                        agent: agent.clone(),
                        status: "running".to_string(),
                        steps: Vec::new(),
                        output: None,
                        error: None,
                    };
                    agent_traces.entry(agent).or_default().push(trace);
                }
                "delegated_thinking" | "delegated_tool_call" | "delegated_tool_result" |
                "delegated_assistant" | "delegated_cc_trace" | "delegated_done" => {
                    if agent.is_empty() { continue; }
                    if let Some(traces) = agent_traces.get_mut(&agent) {
                        if let Some(trace) = traces.last_mut() {
                            let step_type = event.event_type.strip_prefix("delegated_").unwrap_or(&event.event_type);
                            let evt = payload.get("event").cloned().unwrap_or(serde_json::json!({}));
                            let step = AgentTraceStep {
                                step_type: step_type.to_string(),
                                iteration: evt.get("iteration").and_then(|v| v.as_u64()).map(|v| v as usize),
                                content: evt.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                tool_name: evt.get("tool_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                tool_input: evt.get("tool_input").cloned(),
                                output: evt.get("output").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                is_error: evt.get("is_error").and_then(|v| v.as_bool()),
                                thinking: evt.get("thinking").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                text: evt.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                tool_result: evt.get("tool_result").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                call_id: evt.get("call_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                success: evt.get("success").and_then(|v| v.as_bool()),
                                error: evt.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            };
                            trace.steps.push(step);
                        }
                    }
                }
                "delegated_task_complete" => {
                    let resolved_agent = if !agent.is_empty() { agent.clone() } else { task_to_agent.get(&task_id).cloned().unwrap_or_default() };
                    if resolved_agent.is_empty() { continue; }
                    if let Some(traces) = agent_traces.get_mut(&resolved_agent) {
                        if let Some(trace) = traces.iter_mut().rfind(|t| t.task_id == task_id) {
                            trace.status = "completed".to_string();
                            trace.output = payload.get("output").and_then(|v| v.as_str()).map(|s| s.to_string());
                        }
                    }
                }
                "delegated_task_error" => {
                    let resolved_agent = if !agent.is_empty() { agent.clone() } else { task_to_agent.get(&task_id).cloned().unwrap_or_default() };
                    if resolved_agent.is_empty() { continue; }
                    if let Some(traces) = agent_traces.get_mut(&resolved_agent) {
                        if let Some(trace) = traces.iter_mut().rfind(|t| t.task_id == task_id) {
                            trace.status = "error".to_string();
                            trace.error = payload.get("error").and_then(|v| v.as_str()).map(|s| s.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        if !agent_traces.is_empty() {
            if let Some(assistant_turn) = turns.iter_mut().rev().find(|t| t.role == "assistant") {
                assistant_turn.agent_traces = agent_traces;
            }
        }

        // Rebuild coordinator trace_steps from EventLog (source of truth).
        // Coordinator events: thinking, tool_call, tool_result, content
        let mut coordinator_traces: Vec<StoredTraceStep> = Vec::new();
        for event in &events {
            if event.source != "coordinator" { continue; }
            let payload = &event.payload;
            match event.event_type.as_str() {
                "thinking" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "thinking".into(),
                        iteration: payload.get("iteration").and_then(|v| v.as_u64()).map(|v| v as usize),
                        content: payload.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        tool_name: None, tool_input: None, output: None,
                    });
                }
                "tool_call" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "tool_call".into(),
                        iteration: None,
                        tool_name: payload.get("tool_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        tool_input: payload.get("tool_input").cloned(),
                        content: None, output: None,
                    });
                }
                "tool_result" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "tool_result".into(),
                        iteration: None,
                        tool_name: payload.get("tool_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        output: payload.get("output").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        content: None, tool_input: None,
                    });
                }
                "content" => {
                    // Final content is the turn content, not a trace step
                }
                _ => {}
            }
        }
        if !coordinator_traces.is_empty() {
            if let Some(assistant_turn) = turns.iter_mut().rev().find(|t| t.role == "assistant") {
                assistant_turn.trace_steps = coordinator_traces;
            }
        }
    }

    // Load plan decision events from independent storage.
    let plan_decisions = if let Ok(app_uuid) = uuid::Uuid::parse_str(&stored.meta.app_id) {
        load_plan_decisions(&state.session_store, &ApplicationId(app_uuid)).await
    } else {
        Vec::new()
    };

    // EventLog metadata for frontend migration.
    let events_count = state.event_log.count(&session_id).await;
    let events_url = format!("/api/sessions/{}/events", session_id);

    Ok(Json(SessionDetail {
        session_id: stored.meta.session_id,
        app_id: stored.meta.app_id,
        created_at: stored.meta.created_at,
        updated_at: stored.meta.updated_at,
        messages,
        model: turns
            .iter()
            .rev()
            .find_map(|turn| turn.meta.as_ref().and_then(|meta| meta.model.clone())),
        turns,
        status: stored.meta.status,
        plan_decisions,
        events_url,
        events_count,
    }))
}

// ---------------------------------------------------------------------------
// DELETE /api/sessions/:session_id — Delete a session
// ---------------------------------------------------------------------------

pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the session to find the app_id
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state.session_store
        .get(&key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get session: {e}")))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse session: {e}")))?;

    // Delete the session data
    state.session_store
        .delete(&key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete session: {e}")))?;

    // Delete the app session index entry
    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, stored.meta.app_id, session_id);
    state.session_store
        .delete(&app_index_key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete session index: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// GET /api/skills
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
}

pub async fn get_skills(State(state): State<Arc<AppState>>) -> Json<Vec<SkillInfo>> {
    let catalog = state.catalog.read().await;
    let skills = catalog
        .catalog()
        .into_iter()
        .map(|e| SkillInfo {
            name: e.name,
            description: e.description,
        })
        .collect();
    Json(skills)
}

// ---------------------------------------------------------------------------
// POST /api/chat — SSE streaming agentic loop with real-time trace
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ChatRequest {
    pub app_id: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    /// Optional session_id for continuing a conversation, or null for new session
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_model() -> String {
    // Placeholder for serde default. The actual model is resolved at runtime
    // from AppState.default_model in the handler.
    String::new()
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn err(status: StatusCode, msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: msg }))
}

/// Convert ExecutorEvent to SSE Event for frontend display.
/// Each event includes an `agent_tab` field for frontend to group events by agent.
fn convert_executor_event_to_sse(event: ExecutorEvent) -> Result<Event, Infallible> {
    match event {
        ExecutorEvent::TaskStarted { task_id, agent } => {
            Ok(Event::default()
                .event("delegated_task_start")
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "agent": agent,
                    "agent_tab": agent,
                }).to_string()))
        }
        ExecutorEvent::AgentEvent { task_id, agent, event: agent_event } => {
            // Forward the internal agent execution event
            let event_type = match &agent_event {
                macaca_proto::AgentExecutionEvent::Thinking { .. } => "delegated_thinking",
                macaca_proto::AgentExecutionEvent::ToolCall { .. } => "delegated_tool_call",
                macaca_proto::AgentExecutionEvent::ToolResult { .. } => "delegated_tool_result",
                macaca_proto::AgentExecutionEvent::Assistant { .. } => "delegated_assistant",
                macaca_proto::AgentExecutionEvent::CcTrace { .. } => "delegated_cc_trace",
                macaca_proto::AgentExecutionEvent::Completed { .. } => "delegated_completed",
            };
            Ok(Event::default()
                .event(event_type)
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "agent": agent,
                    "agent_tab": agent,
                    "event": agent_event,
                }).to_string()))
        }
        ExecutorEvent::TaskCompleted { task_id, result } => {
            Ok(Event::default()
                .event("delegated_task_complete")
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "success": result.success,
                    "output": result.output,
                    "agent_tab": "result",
                }).to_string()))
        }
        ExecutorEvent::TaskFailed { task_id, error } => {
            Ok(Event::default()
                .event("delegated_task_error")
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "error": error,
                    "agent_tab": "error",
                }).to_string()))
        }
        ExecutorEvent::TaskCancelled { task_id } => {
            Ok(Event::default()
                .event("delegated_task_cancelled")
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                }).to_string()))
        }
        ExecutorEvent::TaskProgress { task_id, step, output } => {
            Ok(Event::default()
                .event("delegated_task_progress")
                .data(serde_json::json!({
                    "task_id": task_id.to_string(),
                    "step": step,
                    "output": output,
                }).to_string()))
        }
        ExecutorEvent::Shutdown => {
            Ok(Event::default()
                .event("executor_shutdown")
                .data("{}".to_string()))
        }
        ExecutorEvent::HookEvent { event: hook_event } => {
            // Convert HookEvent to SSE for coordinator notification
            match hook_event {
                macaca_kernel::executor::fork_manager::HookEvent::DelegateCompleted { fork_id, task_id, success, output } => {
                    Ok(Event::default()
                        .event("hook_delegate_completed")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "task_id": task_id.to_string(),
                            "success": success,
                            "output": output,
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::DelegateFailed { fork_id, task_id, error } => {
                    Ok(Event::default()
                        .event("hook_delegate_failed")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "task_id": task_id.to_string(),
                            "error": error,
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkValidated { fork_id, result } => {
                    Ok(Event::default()
                        .event("hook_fork_validated")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "result": format!("{:?}", result),
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkMerged { fork_id } => {
                    Ok(Event::default()
                        .event("hook_fork_merged")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkCreated { fork_id, application_id, agent_name } => {
                    Ok(Event::default()
                        .event("hook_fork_created")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "application_id": application_id.to_string(),
                            "agent_name": agent_name,
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkWaiting { fork_id, delegate_task_id } => {
                    Ok(Event::default()
                        .event("hook_fork_waiting")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "delegate_task_id": delegate_task_id.to_string(),
                            "agent_tab": "hook",
                        }).to_string()))
                }
                macaca_kernel::executor::fork_manager::HookEvent::ForkResumed { fork_id, delegate_result } => {
                    Ok(Event::default()
                        .event("hook_fork_resumed")
                        .data(serde_json::json!({
                            "fork_id": fork_id.to_string(),
                            "task_id": delegate_result.task_id.to_string(),
                            "success": delegate_result.success,
                            "agent_tab": "hook",
                        }).to_string()))
                }
            }
        }
    }
}

pub async fn root_not_found() -> (StatusCode, Json<ErrorResponse>) {
    err(
        StatusCode::NOT_FOUND,
        "Agent OS API server does not host a web UI at /".into(),
    )
}

pub async fn stream_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state.session_store
        .get(&key)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get session: {e}")))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse session: {e}")))?;

    let app_id = ApplicationId(uuid::Uuid::parse_str(&stored.meta.app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let maybe_executor = state.executor_registry.get(&app_id).await;

    // Check if coordinator is actively running (has an active_session entry)
    let active = state.active_sessions.read().await.contains_key(&session_id);

    // If coordinator is active, hot-swap its sse_tx so events flow to this new connection
    let mut coordinator_rx = if active {
        let (new_tx, new_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
        // Replace sse_tx → bridge now forwards coordinator events to new_rx
        let sessions = state.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            *session.sse_tx.write().await = new_tx;
            tracing::info!(session_id = %session_id, "SSE tx hot-swapped for browser reconnect");
        }
        Some(new_rx)
    } else {
        None
    };

    let stream = async_stream::stream! {
        use tokio::sync::broadcast;

        // NOTE: Do NOT check stored.meta.status here.
        // Coordinator may have completed (status='completed') while executor
        // workers are still running delegated tasks. Always subscribe to
        // executor events and let the broadcast channel close naturally.

        let Some(executor) = maybe_executor else {
            yield Ok(Event::default().event("session_end").data("{}"));
            return;
        };

        let mut executor_rx = executor.subscribe_to_events();

        if let Some(ref mut coord_rx) = coordinator_rx {
            // Merged mode: coordinator events + executor events
            loop {
                tokio::select! {
                    // Coordinator events (thinking, content, done, loop_paused, etc.)
                    result = coord_rx.recv() => {
                        match result {
                            Some(event) => yield event,
                            None => break, // coordinator finished
                        }
                    }
                    // Executor events (delegated agent traces)
                    result = executor_rx.recv() => {
                        match result {
                            Ok(event) => yield convert_executor_event_to_sse(event),
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
        } else {
            // No active coordinator — just stream executor events
            loop {
                match executor_rx.recv().await {
                    Ok(event) => yield convert_executor_event_to_sse(event),
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// POST /api/chat — returns an SSE stream of agentic loop events.
///
/// SSE event types:
/// - `thinking`     — LLM is processing (iteration number)
#[derive(Deserialize)]
pub struct StopRequest {
    pub app_id: String,
}

/// POST /api/chat/stop — terminate all running processes for an application.
///
/// Sets the cancel flag for the coordinator loop, shuts down the executor,
/// and signals all PlanLoop/WorkerLoop instances to stop.
pub async fn post_chat_stop(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StopRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    let mut stopped = Vec::new();

    // 1. Set coordinator cancel flag
    {
        let flags = state.cancel_flags.read().await;
        if let Some(flag) = flags.get(&req.app_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            stopped.push("coordinator");
        }
    }

    // 2. Shutdown executor (stops all delegated agent tasks)
    if let Some(executor) = state.executor_registry.get(&app_id).await {
        executor.shutdown().await;
        stopped.push("executor");
    }

    // 3. Signal PlanLoop shutdown and REMOVE handle so it can be restarted
    {
        let mut handles = state.plan_loop_handles.write().await;
        if let Some(flag) = handles.remove(&app_id) {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            stopped.push("plan_loop");
        }
    }
    // Also remove the waker so a fresh one is created on restart
    {
        let mut wakers = state.plan_loop_wakers.write().await;
        wakers.remove(&app_id);
    }

    // 4. Signal all WorkerLoop shutdowns and REMOVE handles
    {
        let mut handles = state.worker_loop_handles.write().await;
        if let Some(flags) = handles.remove(&app_id) {
            for flag in &flags {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            stopped.push("worker_loops");
        }
    }

    // 5. Broadcast stop event to all sessions for this app
    let event = Event::default()
        .event("stopped")
        .data(serde_json::json!({"reason": "User terminated all processes"}).to_string());
    broadcast_to_app_sessions(
        &state,
        &app_id,
        event,
        serde_json::json!({"event": "terminated", "reason": "user_action"}),
    ).await;

    tracing::info!(app_id = %app_id, stopped = ?stopped, "All processes terminated");

    Ok(Json(serde_json::json!({
        "status": "terminated",
        "stopped": stopped,
    })))
}

/// - `tool_call`    — tool invocation (name + input)
/// - `tool_result`  — tool execution result (name + output)
/// - `assistant`    — intermediate assistant text
/// - `content`      — final assistant response
/// - `done`         — summary (model, tokens, iterations, tools_used)
/// - `error`        — error occurred
pub async fn post_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    // 1. Parse app_id and find the app directory.
    let app_uuid: uuid::Uuid = req
        .app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    let app_dirs = state.app_dirs.read().await;
    let app_dir = app_dirs
        .get(&app_id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "App not found".into()))?
        .clone();
    drop(app_dirs);

    // 2. Build system prompt from app configuration.
    // Get app manifest from registry to read workflow configuration
    let discovered_app = {
        let registry = state.registry.read().await;
        registry.get_app(&app_id).cloned()
    };

    // Determine coordinator from workflow config or default
    // Find the entry agent from the "smart-router" workflow, or fallback to
    // the first agent in the manifest. The entry agent receives all user messages.
    let coordinator = discovered_app
        .as_ref()
        .and_then(|a| a.manifest.workflows.as_ref())
        .and_then(|w| {
            // Prefer "smart-router" workflow, then any workflow with "coordinator" in steps
            w.get("smart-router")
                .or_else(|| w.values().find(|wf| wf.steps.iter().any(|s| s.agent == "coordinator")))
        })
        .and_then(|wf| wf.steps.first())
        .map(|s| s.agent.as_str())
        .unwrap_or("coordinator");

    let persona_dir = app_dir.join(format!("personas/{coordinator}"));
    let system_prompt = if persona_dir.exists() {
        let persona = AgentPersona::load_from_directory(&persona_dir)
            .await
            .map_err(|e| {
                err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Persona load failed: {e}"),
                )
            })?;
        // Load additional workflow prompt from file if exists
        let workflow_prompt_path = persona_dir.join("prompts/workflow.md");
        let workflow_context = if workflow_prompt_path.exists() {
            tokio::fs::read_to_string(&workflow_prompt_path).await.unwrap_or_default()
        } else {
            String::new()
        };
        persona.to_system_prompt(if workflow_context.is_empty() { None } else { Some(&workflow_context) })
    } else {
        // Generic fallback - no app-specific persona
        "You are an AI assistant in Macaca OS with access to tools.\n\
         Use available tools to accomplish the user's tasks.\n\
         Respond helpfully and concisely.".into()
    };

    // 3. Append skill catalog (knowledge skills context).
    let catalog = state.catalog.read().await;
    let catalog_prompt = catalog.catalog_prompt();
    let mut full_system = if catalog_prompt.is_empty() {
        system_prompt
    } else {
        format!("{system_prompt}\n\n{catalog_prompt}")
    };

    // Inject workspace paths for coordinator
    {
        let workspaces = state.app_workspaces.read().await;
        if let Some(ws) = workspaces.get(&app_id) {
            full_system.push_str(&format!(
                "\n\n## Workspace Paths\n\
                 - Shared workspace: {}\n\
                 - Your private workspace: {}\n\
                 Create project files in the shared workspace. Use your private workspace for temporary/scratch files only.",
                ws.shared.display(),
                ws.agent_workspace(coordinator).display(),
            ));
        }
    }

    // 4. Build messages with conversation history.
    // Use provided session_id or generate a new UUID for new sessions
    let session_key = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let sessions = state.sessions.read().await;
    let history = sessions.get(&session_key);

    let mut messages = if let Some(hist) = history {
        let mut msgs = vec![LlmMessage::system(full_system)];
        msgs.extend(hist.clone());
        msgs
    } else {
        vec![LlmMessage::system(full_system)]
    };

    messages.push(LlmMessage::user(req.prompt.clone()));

    // 5. Create SSE channel, cancel flag, and spawn the agentic loop task.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Hot-swappable SSE sender for browser refresh recovery.
    // The bridge task reads from rx and forwards to sse_tx.
    // stream_session_events can replace sse_tx on reconnect.
    let sse_tx: Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>> =
        Arc::new(tokio::sync::RwLock::new(tx.clone()));
    let sse_tx_for_spawn = Arc::clone(&sse_tx);

    // Register the cancel flag so /api/chat/stop can set it.
    {
        let mut flags = state.cancel_flags.write().await;
        flags.insert(req.app_id.clone(), Arc::clone(&cancel_flag));
    }

    // Subscribe to executor events for delegated agent tracking
    let executor_events_rx = {
        if let Some(executor) = state.executor_registry.get(&app_id).await {
            Some(executor.subscribe_to_events())
        } else {
            None
        }
    };

    // Create agent trace collector for SSE stream and session persistence
    let agent_trace_collector = AgentTraceCollector::new();

    // Use default model from state if not specified
    let model = if req.model.is_empty() {
        state.default_model.clone()
    } else {
        req.model.clone()
    };
    let prompt = req.prompt.clone();
    let tool_defs = state.tools.to_definitions();
    let state_clone = Arc::clone(&state);
    let cancel = Arc::clone(&cancel_flag);
    let app_id_for_cleanup = req.app_id.clone();

    // Determine workflow to use (from app.yaml entrypoint)
    let workflow_name = discovered_app
        .as_ref()
        .and_then(|app| app.manifest.entrypoint.as_ref())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "smart-router".to_string());

    // Clone needed data for the async task
    let app_dir_clone = app_dir.clone();
    let discovered_app_clone = discovered_app.clone();

    // Clone collector for session saving
    let collector_for_save = Arc::clone(&agent_trace_collector);

    // === Session 实时保存改进 ===
    // 在 agent 执行之前立即创建 Session（状态: running）
    // 这样即使刷新浏览器，Session 也会显示在 sidebar 中
    let session_key_for_initial = session_key.clone();
    let session_key_db = format!("{}{}", SESSION_PREFIX, &session_key_for_initial);
    let now = Utc::now();
    let title = prompt.chars().take(50).collect::<String>();
    let initial_stored = StoredSession {
        meta: SessionMeta {
            session_id: session_key_for_initial.clone(),
            app_id: app_id.0.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            title: Some(title),
            status: "running".to_string(), // 设置为 running 状态
        },
        messages: vec![LlmMessage::user(prompt.clone())],
        turns: vec![],
    };
    if let Ok(data) = serde_json::to_vec(&initial_stored) {
        let _ = state.session_store.set(&session_key_db, &data).await;
        let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id.0, &session_key_for_initial);
        let _ = state.session_store.set(&app_index_key, session_key_for_initial.as_bytes()).await;
    }
    // === End Session 实时保存改进 ===

    // Clone session_key for use in spawn (since it will be moved)
    let session_key_for_spawn = session_key.clone();

    tokio::spawn(async move {
        // Spawn the session event collector — a single subscriber that handles
        // both EventLog writes and AgentTraceCollector feeding.
        // All EventLog write logic lives in event_persistence.rs, NOT here.
        let collector_for_events = Arc::clone(&collector_for_save);
        let event_collector_handle = if let Some(executor) = state_clone.executor_registry.get(&app_id).await {
            Some(crate::event_persistence::spawn_session_event_collector(
                executor,
                Arc::clone(&state_clone.event_log),
                session_key_for_spawn.clone(),
                collector_for_events,
            ))
        } else {
            None
        };

        // Set session_id on delegate tool so delegated tasks inherit the current session
        *state_clone.delegate_session_id.write().await = Some(session_key_for_spawn.clone());

        let result = execute_workflow_steps(
            &state_clone,
            &app_id,
            &app_dir_clone,
            &discovered_app_clone,
            &workflow_name,
            prompt.clone(),
            &model,
            tool_defs,
            &tx,
            &cancel,
            session_key_for_spawn.clone(),
            Arc::clone(&collector_for_save),
            Arc::clone(&sse_tx_for_spawn),
        ).await;

        // Clear delegate session_id after execution completes
        *state_clone.delegate_session_id.write().await = None;

        match result {
            Ok(run) => {
                // Send done event.
                let done_payload = serde_json::json!({
                    "model": model,
                    "tokens": {
                        "prompt": run.usage.0,
                        "completion": run.usage.1,
                        "total": run.usage.2,
                    },
                    "iterations": run.iterations,
                    "tools_used": run.tools_used,
                });
                state_clone.event_log.append(&session_key_for_spawn, "done", "coordinator", done_payload.clone()).await;
                let _ = tx
                    .send(Ok(Event::default().event("done").data(
                        done_payload.to_string(),
                    )))
                    .await;

                // Save conversation history to in-memory cache.
                let mut sessions = state_clone.sessions.write().await;
                let hist = sessions.entry(session_key_for_spawn.clone()).or_insert_with(Vec::new);
                hist.push(LlmMessage::user(prompt.clone()));
                hist.push(LlmMessage::assistant(run.content.clone()));
                let hist_snapshot = hist.clone();

                // Persist session to redb store.
                let session_id = session_key_for_spawn.clone();
                let app_id = app_id_for_cleanup.clone();
                let store = Arc::clone(&state_clone.session_store);
                let session_key_db = format!("{}{}", SESSION_PREFIX, session_id);

                // Create or update session in persistent store.
                let title = prompt.chars().take(50).collect::<String>();
                let now = Utc::now();
                let existing = store
                    .get(&session_key_db)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|data| serde_json::from_slice::<StoredSession>(&data).ok());
                let mut turns = existing
                    .as_ref()
                    .map(stored_turns_or_messages)
                    .unwrap_or_default();
                // Update the existing running assistant turn in-place (preserving
                // agent_traces written by the periodic saver) instead of deleting
                // and re-creating, which would cause trace data loss.
                let agent_traces = collector_for_save.get_all().await;
                if let Some(pos) = turns.iter().rposition(|t| {
                    t.role == "assistant"
                        && matches!(t.status.as_deref(), Some("running") | Some("pending"))
                }) {
                    // Update existing running turn to completed
                    let turn = &mut turns[pos];
                    turn.content = run.content.clone();
                    turn.status = Some(run.status.clone());
                    turn.trace_steps = run.trace_steps.clone();
                    turn.cc_trace_steps = run.cc_trace_steps.clone();
                    turn.meta = Some(AssistantExecutionMeta {
                        model: Some(model.clone()),
                        tokens: Some(ExecutionTokens {
                            prompt: run.usage.0,
                            completion: run.usage.1,
                            total: run.usage.2,
                        }),
                        iterations: Some(run.iterations),
                        tools_used: run.tools_used.clone(),
                    });
                    turn.agent_traces = agent_traces;
                    // Ensure user turn exists before this assistant turn
                    if pos == 0 || turns[pos - 1].role != "user" || turns[pos - 1].content != prompt {
                        turns.insert(pos, StoredTurn {
                            role: "user".into(),
                            content: prompt.clone(),
                            status: None,
                            trace_steps: Vec::new(),
                            cc_trace_steps: Vec::new(),
                            meta: None,
                            agent_traces: std::collections::HashMap::new(),
                        });
                    }
                } else {
                    // No running turn found — create new pair (first-time save)
                    turns.push(StoredTurn {
                        role: "user".into(),
                        content: prompt.clone(),
                        status: None,
                        trace_steps: Vec::new(),
                        cc_trace_steps: Vec::new(),
                        meta: None,
                        agent_traces: std::collections::HashMap::new(),
                    });
                    turns.push(StoredTurn {
                        role: "assistant".into(),
                        content: run.content.clone(),
                        status: Some(run.status.clone()),
                        trace_steps: run.trace_steps.clone(),
                        cc_trace_steps: run.cc_trace_steps.clone(),
                        meta: Some(AssistantExecutionMeta {
                            model: Some(model.clone()),
                            tokens: Some(ExecutionTokens {
                                prompt: run.usage.0,
                                completion: run.usage.1,
                                total: run.usage.2,
                            }),
                            iterations: Some(run.iterations),
                            tools_used: run.tools_used.clone(),
                        }),
                        agent_traces,
                    });
                }
                let meta = SessionMeta {
                    session_id: session_id.clone(),
                    app_id: app_id.clone(),
                    created_at: existing.as_ref().map(|stored| stored.meta.created_at).unwrap_or(now),
                    updated_at: now,
                    message_count: hist_snapshot.len(),
                    title: existing
                        .as_ref()
                        .and_then(|stored| stored.meta.title.clone())
                        .or(Some(title)),
                    status: existing
                        .as_ref()
                        .map(|stored| stored.meta.status.clone())
                        .unwrap_or_else(|| "draft".to_string()),
                };
                let stored = StoredSession {
                    meta: meta.clone(),
                    messages: hist_snapshot,
                    turns,
                };

                // Save session data
                if let Ok(data) = serde_json::to_vec(&stored) {
                    let _ = store.set(&session_key_db, &data).await;
                    // Save app index
                    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id, session_id);
                    let _ = store.set(&app_index_key, session_id.as_bytes()).await;
                }
            }
            Err(e) => {
                let _ = tx
                    .send(Ok(Event::default()
                        .event("error")
                        .data(serde_json::json!({"error": e}).to_string())))
                    .await;

                let mut sessions = state_clone.sessions.write().await;
                let hist = sessions.entry(session_key_for_spawn.clone()).or_insert_with(Vec::new);
                hist.push(LlmMessage::user(prompt.clone()));
                hist.push(LlmMessage::assistant(format!("Error: {e}")));
                let hist_snapshot = hist.clone();

                let session_id = session_key_for_spawn.clone();
                let app_id = app_id_for_cleanup.clone();
                let store = Arc::clone(&state_clone.session_store);
                let session_key_db = format!("{}{}", SESSION_PREFIX, session_id);
                let existing = store
                    .get(&session_key_db)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|data| serde_json::from_slice::<StoredSession>(&data).ok());
                let mut turns = existing
                    .as_ref()
                    .map(stored_turns_or_messages)
                    .unwrap_or_default();
                // Update existing running turn to error state (preserve agent_traces
                // from periodic saver). Same pattern as success path.
                if let Some(pos) = turns.iter().rposition(|t| {
                    t.role == "assistant"
                        && matches!(t.status.as_deref(), Some("running") | Some("pending"))
                }) {
                    let turn = &mut turns[pos];
                    turn.content = format!("Error: {e}");
                    turn.status = Some("error".into());
                    turn.meta = Some(AssistantExecutionMeta {
                        model: Some(model.clone()),
                        tokens: None,
                        iterations: None,
                        tools_used: Vec::new(),
                    });
                    // agent_traces: keep whatever periodic saver wrote
                    if pos == 0 || turns[pos - 1].role != "user" || turns[pos - 1].content != prompt {
                        turns.insert(pos, StoredTurn {
                            role: "user".into(),
                            content: prompt.clone(),
                            status: None,
                            trace_steps: Vec::new(),
                            cc_trace_steps: Vec::new(),
                            meta: None,
                            agent_traces: std::collections::HashMap::new(),
                        });
                    }
                } else {
                    turns.push(StoredTurn {
                        role: "user".into(),
                        content: prompt.clone(),
                        status: None,
                        trace_steps: Vec::new(),
                        cc_trace_steps: Vec::new(),
                        meta: None,
                        agent_traces: std::collections::HashMap::new(),
                    });
                    turns.push(StoredTurn {
                        role: "assistant".into(),
                        content: format!("Error: {e}"),
                        status: Some("error".into()),
                        trace_steps: Vec::new(),
                        cc_trace_steps: Vec::new(),
                        meta: Some(AssistantExecutionMeta {
                            model: Some(model.clone()),
                            tokens: None,
                            iterations: None,
                            tools_used: Vec::new(),
                        }),
                        agent_traces: std::collections::HashMap::new(),
                    });
                }
                let now = Utc::now();
                let title = prompt.chars().take(50).collect::<String>();
                let stored = StoredSession {
                    meta: SessionMeta {
                        session_id: session_id.clone(),
                        app_id: app_id.clone(),
                        created_at: existing.as_ref().map(|stored| stored.meta.created_at).unwrap_or(now),
                        updated_at: now,
                        message_count: hist_snapshot.len(),
                        title: existing
                            .as_ref()
                            .and_then(|stored| stored.meta.title.clone())
                            .or(Some(title)),
                        status: existing
                            .as_ref()
                            .map(|stored| stored.meta.status.clone())
                            .unwrap_or_else(|| "draft".to_string()),
                    },
                    messages: hist_snapshot,
                    turns,
                };
                if let Ok(data) = serde_json::to_vec(&stored) {
                    let _ = store.set(&session_key_db, &data).await;
                    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id, session_id);
                    let _ = store.set(&app_index_key, session_id.as_bytes()).await;
                }
            }
        }

        // NOTE: event_collector_handle is NOT aborted here.
        // It continues running and writing to EventLog as long as
        // executor workers are active (survives coordinator completion).
        // It will stop naturally when the executor broadcast channel closes.
        drop(event_collector_handle);

        // Clean up cancel flag.
        let mut flags = state_clone.cancel_flags.write().await;
        flags.remove(&app_id_for_cleanup);
    });

    // Convert receiver into a Stream for SSE, merging with executor events if available.
    let session_store_for_realtime = Arc::clone(&state.session_store);
    let session_id_for_realtime = session_key.clone();
    let app_id_for_realtime = app_id.clone();
    // Note: periodic trace saver is spawned INSIDE the coordinator's spawned task
    // (not here in the SSE stream scope) so it lives as long as the coordinator,
    // surviving browser refresh/SSE reconnection.

    // Bridge: forward coordinator events from rx to the hot-swappable sse_tx.
    // This lets the coordinator keep using its original tx unchanged, while
    // browser refresh can swap sse_tx to a new sender via stream_session_events.
    let bridge_sse_tx = Arc::clone(&sse_tx);
    let bridge_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let sender = bridge_sse_tx.read().await;
            let _ = sender.send(event).await;
        }
    });

    // Create the SSE stream channel that the bridge forwards to.
    // The initial sse_tx already points to the original tx (set during active_sessions registration).
    // We need a NEW channel pair: sse_tx sends into it, stream_rx reads from it.
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    // Replace sse_tx with stream_tx so the bridge forwards coordinator events here
    *sse_tx.write().await = stream_tx;

    let stream = async_stream::stream! {
        // Fork: receive from both channels
        use tokio::sync::broadcast;

        if let Some(mut executor_rx) = executor_events_rx {
            tracing::info!("SSE stream: executor_rx is available, subscribing to executor events");
            // Both channels available - merge them
            // Track when main channel is closed and when we've seen final executor event
            let mut rx_closed = false;
            // Track pending delegated tasks - when empty and rx closed, we're done
            let mut pending_delegated_tasks: std::collections::HashSet<String> = std::collections::HashSet::new();

            loop {
                // If main channel closed and no pending tasks, exit
                if rx_closed && pending_delegated_tasks.is_empty() {
                    break;
                }

                // First, drain any available executor events (non-blocking)
                loop {
                    match executor_rx.try_recv() {
                        Ok(event) => {
                            // Track task lifecycle
                            match &event {
                                ExecutorEvent::TaskStarted { task_id, .. } => {
                                    pending_delegated_tasks.insert(task_id.to_string());
                                }
                                ExecutorEvent::TaskCompleted { task_id, .. } |
                                ExecutorEvent::TaskFailed { task_id, .. } |
                                ExecutorEvent::TaskCancelled { task_id } => {
                                    pending_delegated_tasks.remove(&task_id.to_string());
                                }
                                _ => {}
                            }

                            // === 实时保存 Session 状态 ===
                            // Only update session status here. agent_traces are
                            // exclusively written by the periodic saver to avoid
                            // multi-writer race conditions.
                            update_session_realtime(
                                &session_store_for_realtime,
                                &session_id_for_realtime,
                                &app_id_for_realtime,
                                &event,
                            ).await;

                            // EventLog append is handled by the independent
                            // event_collector_handle task (survives browser disconnect).
                            // SSE stream only converts and forwards to the browser.

                            // Convert ExecutorEvent to SSE Event
                            let sse_event = convert_executor_event_to_sse(event);
                            yield sse_event;
                        }
                        Err(broadcast::error::TryRecvError::Empty) => break, // No more events
                        Err(broadcast::error::TryRecvError::Closed) => return,
                        Err(broadcast::error::TryRecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "SSE stream: broadcast channel lagged, events lost");
                            continue;
                        }
                    }
                }

                // Check if we should exit after draining executor events
                if rx_closed && pending_delegated_tasks.is_empty() {
                    break;
                }

                // Now wait on main channel with a timeout to periodically check executor events
                tokio::select! {
                    // Regular SSE events from agent execution
                    result = stream_rx.recv(), if !rx_closed => {
                        match result {
                            Some(event) => yield event,
                            None => {
                                // Main channel closed, but keep receiving executor events
                                rx_closed = true;
                            }
                        }
                    }
                    // Periodic check for executor events (every 100ms)
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Loop back to drain executor events
                    }
                }
            }
        } else {
            // No executor events - just use bridged channel
            while let Some(event) = stream_rx.recv().await {
                yield event;
            }
        }
        // Note: periodic saver and bridge are NOT aborted here.
        // They live as long as the coordinator (spawned task), not the SSE stream,
        // so browser refresh doesn't kill them.
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Find an agent ID by name for a specific app.
/// Returns the first agent matching the name.
async fn find_agent_by_name(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> Option<AgentId> {
    let agent_ids = state.runtime.app_agents(app_id).await.ok()?;
    let manifests = state.kernel.list_agents().await;
    manifests
        .into_iter()
        .find(|m| agent_ids.contains(&m.id) && m.name == agent_name)
        .map(|m| m.id)
}

/// Execute a multi-step workflow with different agents for each step.
/// Each step uses its own agent with the appropriate persona.
async fn execute_workflow_steps(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    app_dir: &std::path::Path,
    discovered_app: &Option<macaca_app::registry::DiscoveredApp>,
    workflow_name: &str,
    initial_prompt: String,
    model: &str,
    tool_defs: Vec<macaca_proto::ToolDefinition>,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel: &Arc<AtomicBool>,
    session_id: String,
    agent_trace_collector: Arc<AgentTraceCollector>,
    sse_tx: Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
) -> Result<AssistantRunResult, String> {

    // Get workflow definition
    let workflow = discovered_app
        .as_ref()
        .and_then(|app| app.manifest.workflows.as_ref())
        .and_then(|w| w.get(workflow_name))
        .cloned();

    let workflow = match workflow {
        Some(w) => w,
        None => {
            // Fallback to single-agent mode if no workflow defined
            return run_agentic_stream_with_agent(
                state, app_id, app_dir, "coordinator",
                initial_prompt, model, tool_defs, tx, cancel,
                session_id,
                Arc::clone(&agent_trace_collector),
                Arc::clone(&sse_tx),
            ).await;
        }
    };

    // Build step execution order (handle dependencies)
    let steps = topological_sort_steps(&workflow.steps);

    let mut step_results: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut total_usage = (0u32, 0u32, 0u32);
    let mut all_tools_used = Vec::new();
    let mut final_content = String::new();
    let mut total_iterations = 0usize;

    // Execute each step
    for step in steps {
        // Check cancellation
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(Ok(Event::default()
                .event("stopped")
                .data(serde_json::json!({"reason": "User cancelled"}).to_string()))).await;
            break;
        }

        // Send step start event
        let _ = tx.send(Ok(Event::default()
            .event("step_start")
            .data(serde_json::json!({
                "step": step.name,
                "agent": step.agent,
            }).to_string()))).await;

        // Find agent by name
        let agent_id = match find_agent_by_name(state, app_id, &step.agent).await {
            Some(id) => Some(id),
            None => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(serde_json::json!({
                        "error": format!("Agent not found: {}", step.agent)
                    }).to_string()))).await;
                continue;
            }
        };

        // Build prompt for this step (include previous step results if dependencies exist)
        let mut step_prompt = initial_prompt.clone();
        if !step.depends_on.is_empty() {
            let context: Vec<String> = step.depends_on.iter()
                .filter_map(|dep| step_results.get(dep))
                .cloned()
                .collect();
            if !context.is_empty() {
                step_prompt = format!(
                    "{}\n\n## Previous Step Results:\n{}",
                    initial_prompt,
                    context.join("\n\n---\n\n")
                );
            }
        }

        // Add step-specific prompt template if provided
        if let Some(ref template) = step.prompt_template {
            step_prompt = format!("{}\n\n## Step Instructions:\n{}", step_prompt, template);
        }

        // Execute step with the appropriate agent
        let result = run_agentic_stream_with_agent_for_step(
            state, app_dir, &step.agent,
            step_prompt, model, tool_defs.clone(),
            agent_id, tx, cancel,
            session_id.clone(),
            app_id.clone(),
            Arc::clone(&agent_trace_collector),
            Arc::clone(&sse_tx),
        ).await;

        match result {
            Ok(run) => {
                total_usage.0 += run.usage.0;
                total_usage.1 += run.usage.1;
                total_usage.2 += run.usage.2;
                all_tools_used.extend(run.tools_used);
                total_iterations += run.iterations;
                final_content = run.content.clone();

                // Store step result for dependent steps
                step_results.insert(step.name.clone(), run.content.clone());

                // Send step complete event
                let _ = tx.send(Ok(Event::default()
                    .event("step_complete")
                    .data(serde_json::json!({
                        "step": step.name,
                        "agent": step.agent,
                    }).to_string()))).await;
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(serde_json::json!({
                        "step": step.name,
                        "agent": step.agent,
                        "error": e
                    }).to_string()))).await;
            }
        }
    }

    Ok(AssistantRunResult {
        content: final_content,
        usage: total_usage,
        iterations: total_iterations,
        tools_used: all_tools_used,
        status: "completed".to_string(),
        trace_steps: Vec::new(),
        cc_trace_steps: Vec::new(),
        agent_traces: std::collections::HashMap::new(),
    })
}

/// Topologically sort workflow steps based on dependencies.
fn topological_sort_steps(steps: &[macaca_app::model::WorkflowStep]) -> Vec<macaca_app::model::WorkflowStep> {
    use std::collections::{HashSet, VecDeque};

    let mut in_degree: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut graph: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();

    for step in steps {
        in_degree.entry(step.name.as_str()).or_insert(0);
        for dep in &step.depends_on {
            graph.entry(dep.as_str()).or_insert_with(Vec::new).push(step.name.as_str());
            *in_degree.entry(step.name.as_str()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut sorted = Vec::new();
    let step_map: std::collections::HashMap<&str, macaca_app::model::WorkflowStep> = steps.iter()
        .map(|s| (s.name.as_str(), s.clone()))
        .collect();

    while let Some(node) = queue.pop_front() {
        if let Some(step) = step_map.get(node) {
            sorted.push(step.clone());
        }
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
    }

    sorted
}

/// Run agentic stream with a specific agent by name.
async fn run_agentic_stream_with_agent(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    app_dir: &std::path::Path,
    agent_name: &str,
    prompt: String,
    model: &str,
    tool_defs: Vec<macaca_proto::ToolDefinition>,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel: &Arc<AtomicBool>,
    session_id: String,
    agent_trace_collector: Arc<AgentTraceCollector>,
    sse_tx: Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
) -> Result<AssistantRunResult, String> {
    let agent_id = find_agent_by_name(state, app_id, agent_name).await;
    run_agentic_stream_with_agent_for_step(
        state, app_dir, agent_name, prompt, model, tool_defs,
        agent_id, tx, cancel, session_id, app_id.clone(),
        agent_trace_collector, sse_tx,
    ).await
}

/// Run agentic stream with a specific agent ID.
pub(crate) async fn run_agentic_stream_with_agent_for_step(
    state: &Arc<AppState>,
    app_dir: &std::path::Path,
    agent_name: &str,
    prompt: String,
    model: &str,
    tool_defs: Vec<macaca_proto::ToolDefinition>,
    agent_id: Option<AgentId>,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel: &Arc<AtomicBool>,
    session_id: String,
    app_id: macaca_proto::ApplicationId,
    agent_trace_collector: Arc<AgentTraceCollector>,
    sse_tx: Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
) -> Result<AssistantRunResult, String> {
    // Get agent's persona
    let persona_dir = app_dir.join(format!("personas/{agent_name}"));
    let system_prompt = if persona_dir.exists() {
        match AgentPersona::load_from_directory(&persona_dir).await {
            Ok(persona) => persona.to_system_prompt(None),
            Err(_) => "You are an AI assistant in Macaca OS.".into(),
        }
    } else {
        "You are an AI assistant in Macaca OS with access to tools.".into()
    };

    // Add skill catalog context
    let catalog = state.catalog.read().await;
    let catalog_prompt = catalog.catalog_prompt();
    let mut full_system = if catalog_prompt.is_empty() {
        system_prompt
    } else {
        format!("{system_prompt}\n\n{catalog_prompt}")
    };

    // Inject workspace paths dynamically
    {
        let workspaces = state.app_workspaces.read().await;
        if let Some(ws) = workspaces.get(&app_id) {
            full_system.push_str(&format!(
                "\n\n## Workspace Paths\n\
                 - Shared workspace: {}\n\
                 - Your private workspace: {}\n\
                 Create project files in the shared workspace. Use your private workspace for temporary/scratch files only.",
                ws.shared.display(),
                ws.agent_workspace(agent_name).display(),
            ));
        }
    }

    let messages = vec![
        LlmMessage::system(full_system),
        LlmMessage::user(prompt),
    ];

    run_agentic_stream(state, agent_id.clone(), messages, model, tool_defs, tx, cancel, session_id, app_id, agent_trace_collector, sse_tx).await
}

/// Run the manual agentic loop, sending SSE events at each step.
/// Updates agent status during execution.
/// Supports pause/resume for Fork-Join workflow.
///
async fn run_agentic_stream(
    state: &Arc<AppState>,
    agent_id: Option<macaca_proto::AgentId>,
    mut messages: Vec<LlmMessage>,
    model: &str,
    tool_defs: Vec<macaca_proto::ToolDefinition>,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    cancel: &Arc<AtomicBool>,
    session_id: String,
    app_id: macaca_proto::ApplicationId,
    agent_trace_collector: Arc<AgentTraceCollector>,
    sse_tx: Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<Result<Event, Infallible>>>>,
) -> Result<AssistantRunResult, String> {
    use macaca_runtime::agentic_loop::ResumeReason;
    use macaca_proto::ForkId;

    // Build coordinator todo tools for this session (session-scoped)
    let coordinator_space = Arc::new(macaca_task::TaskSpace::new(
        app_id.clone(), Some(session_id.clone()), Arc::clone(&state.todo_store),
    ));
    // Build callback to lazily start PlanLoop + WorkerLoops when a goal is created
    let plan_loop_state = Arc::clone(&state);
    let plan_loop_app_id = app_id.clone();
    let on_goal_created: macaca_tools::OnGoalCreated = Arc::new(move || {
        let state = Arc::clone(&plan_loop_state);
        let app_id = plan_loop_app_id.clone();
        tokio::spawn(async move {
            crate::routes::ensure_plan_and_worker_loops(&state, &app_id).await;
            // Wake PlanLoop immediately so it picks up the new goal
            let wakers = state.plan_loop_wakers.read().await;
            if let Some(w) = wakers.get(&app_id) {
                w.wake();
            }
        });
    });

    // Coordinator only gets goal-level tools (not create_todo/claim/review — those are for planner/workers)
    let todo_tools: Vec<Box<dyn macaca_tools::Tool>> = vec![
        Box::new(macaca_tools::CheckTodoProgressTool { space: Arc::clone(&coordinator_space) }),
        Box::new(macaca_tools::ReassignTaskTool { space: Arc::clone(&coordinator_space) }),
        Box::new(macaca_tools::CreateGoalTool { space: coordinator_space, on_created: Some(on_goal_created) }),
    ];

    // Merge todo tool definitions into the tool_defs for LLM
    let mut all_tool_defs = tool_defs;
    for t in &todo_tools {
        all_tool_defs.push(macaca_proto::ToolDefinition {
            name: t.name().to_owned(),
            description: t.description().to_owned(),
            parameters: t.parameters_schema(),
        });
    }

    let options = LlmOptions {
        model: model.to_string(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        tools: Some(all_tool_defs),
        ..Default::default()
    };

    let max_iterations = 50;
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut total_tokens = 0u32;
    let mut tools_used = Vec::new();
    let mut final_content = String::new();
    let mut iterations = 0usize;
    let mut claude_code_failed = false; // Track if claude_code_execute has failed.
    let mut status = "completed".to_string();
    let mut trace_steps = Vec::new();
    let mut cc_trace_steps: Vec<TraceEvent> = Vec::new();
    // Create pause/resume mechanism for Fork-Join workflow
    let (resume_tx, mut resume_rx) = tokio::sync::mpsc::channel::<ResumeReason>(1);
    let pause_signal = Arc::new(AtomicBool::new(false));

    // Register coordinator in active_sessions so hook_consumer can send resume signals
    // and stream_session_events can hot-swap the SSE sender on browser refresh
    state.active_sessions.write().await.insert(session_id.clone(), crate::state::ActiveSession {
        session_id: session_id.clone(),
        app_id: app_id.clone(),
        pause_signal: Arc::clone(&pause_signal),
        resume_tx: resume_tx.clone(),
        sse_tx: Arc::clone(&sse_tx),
    });
    tracing::info!(session_id = %session_id, "Coordinator registered in active_sessions");

    // Session snapshot persistence for refresh recovery.
    // EventLog now persists all trace content, so this only updates the status field.
    let session_store_for_snapshots = Arc::clone(&state.session_store);
    let session_id_for_snapshots = session_id.clone();
    let app_id_for_snapshots = app_id.clone();
    let persist_running_snapshot = |status: &str| {
        let store = Arc::clone(&session_store_for_snapshots);
        let session_id = session_id_for_snapshots.clone();
        let app_id = app_id_for_snapshots.clone();
        let status = status.to_string();
        tokio::spawn(async move {
            persist_session_snapshot(
                &store,
                &session_id,
                &app_id,
                Some(&status),
                None,
                None,
                None,
                None,
                None,
            ).await;
        });
    };

    for i in 1..=max_iterations {
        iterations = i;

        // ===== PAUSE/RESUME CHECK FOR FORK-JOIN WORKFLOW =====
        while pause_signal.load(Ordering::SeqCst) {
            // Send paused event to SSE
            let paused_payload = serde_json::json!({
                "session_id": session_id,
                "iteration": i,
                "reason": "waiting_for_delegate"
            });
            state.event_log.append(&session_id, "loop_paused", "coordinator", paused_payload.clone()).await;
            let _ = tx.send(Ok(Event::default()
                .event("loop_paused")
                .data(paused_payload.to_string())))
                .await;

            // Wait for resume signal
            if let Some(reason) = resume_rx.recv().await {
                match reason {
                    ResumeReason::DelegateCompleted { task_id, success, output } => {
                        // Inject result into messages
                        let result_msg = format!(
                            "[Delegate Task {} Completed]\nSuccess: {}\nOutput: {}",
                            task_id, success, output
                        );
                        messages.push(LlmMessage::user(result_msg));

                        // Send resumed event
                        let resumed_payload = serde_json::json!({
                            "session_id": session_id,
                            "task_id": task_id,
                            "success": success
                        });
                        state.event_log.append(&session_id, "loop_resumed", "coordinator", resumed_payload.clone()).await;
                        let _ = tx.send(Ok(Event::default()
                            .event("loop_resumed")
                            .data(resumed_payload.to_string())))
                            .await;
                    }
                    ResumeReason::DelegateFailed { task_id, error } => {
                        let result_msg = format!(
                            "[Delegate Task {} Failed]\nError: {}",
                            task_id, error
                        );
                        messages.push(LlmMessage::user(result_msg));

                        let resumed_fail_payload = serde_json::json!({
                            "session_id": session_id,
                            "task_id": task_id,
                            "success": false,
                            "error": error
                        });
                        state.event_log.append(&session_id, "loop_resumed", "coordinator", resumed_fail_payload.clone()).await;
                        let _ = tx.send(Ok(Event::default()
                            .event("loop_resumed")
                            .data(resumed_fail_payload.to_string())))
                            .await;
                    }
                    _ => {}
                }
                pause_signal.store(false, Ordering::SeqCst);
                break;
            }
        }
        // ===== END PAUSE/RESUME CHECK =====

        // Check cancellation before each iteration.
        if cancel.load(Ordering::Relaxed) {
            let _ = tx
                .send(Ok(Event::default()
                    .event("stopped")
                    .data(serde_json::json!({"reason": "User cancelled"}).to_string())))
                .await;
            final_content = "Execution stopped by user.".into();
            status = "stopped".into();
            break;
        }

        // Send thinking event.
        let thinking_payload = serde_json::json!({"iteration": i});
        state.event_log.append(&session_id, "thinking", "coordinator", thinking_payload.clone()).await;
        let _ = tx
            .send(Ok(Event::default()
                .event("thinking")
                .data(thinking_payload.to_string())))
            .await;

        // Update agent status to thinking
        if let Some(ref id) = agent_id {
            state.kernel.update_agent_activity(
                id,
                macaca_proto::AgentActivity::Thinking {
                    context: format!("Processing iteration {}", i),
                },
            ).await;
        }

        trace_steps.push(StoredTraceStep {
            step_type: "thinking".into(),
            iteration: Some(i),
            tool_name: None,
            tool_input: None,
            output: None,
            content: None,
        });
        persist_running_snapshot("running");

        // Call LLM.
        let response = match state.llm.chat(messages.clone(), &options).await {
            Ok(resp) => resp,
            Err(e) => {
                // Improve error diagnosis
                let err_msg = diagnose_llm_error(&e);
                tracing::error!(error = %err_msg, "LLM call failed");
                // Send accumulated trace steps before returning error
                for step in &trace_steps {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event(match step.step_type.as_str() {
                                "thinking" => "thinking",
                                "assistant" => "assistant",
                                "tool_call" => "tool_call",
                                "tool_result" => "tool_result",
                                _ => "tool_result",
                            })
                            .data(serde_json::json!({
                                "iteration": step.iteration,
                                "tool_name": step.tool_name,
                                "tool_input": step.tool_input,
                                "output": step.output,
                                "content": step.content,
                            }).to_string())))
                        .await;
                }
                for cc_step in &cc_trace_steps {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("cc_trace")
                            .data(serde_json::json!({
                                "type": cc_step.event_type,
                                "tool_name": cc_step.tool_name,
                                "tool_input": cc_step.tool_input,
                                "tool_result": cc_step.tool_result,
                                "thinking": cc_step.thinking,
                                "text": cc_step.text,
                                "is_error": cc_step.is_error,
                            }).to_string())))
                        .await;
                }
                return Err(err_msg);
            }
        };

        // Accumulate token usage.
        prompt_tokens += response.usage.prompt_tokens;
        completion_tokens += response.usage.completion_tokens;
        total_tokens += response.usage.total_tokens;

        // Check for tool calls.
        let has_tool_calls = response
            .tool_calls
            .as_ref()
            .map(|tc| !tc.is_empty())
            .unwrap_or(false);

        if has_tool_calls {
            let tool_calls = response.tool_calls.as_ref().unwrap();

            // Append assistant message with tool calls to conversation.
            messages.push(LlmMessage::assistant_with_tool_calls(
                response.content.clone(),
                tool_calls.clone(),
            ));

            // Stream intermediate assistant text if present.
            if !response.content.is_empty() {
                let _ = tx
                    .send(Ok(Event::default()
                        .event("assistant")
                        .data(
                            serde_json::json!({"content": response.content}).to_string(),
                        )))
                    .await;
                trace_steps.push(StoredTraceStep {
                    step_type: "assistant".into(),
                    iteration: None,
                    tool_name: None,
                    tool_input: None,
                    output: None,
                    content: Some(response.content.clone()),
                });
                persist_running_snapshot("running");
            }

            // Execute each tool call and stream events.
            for tc in tool_calls {
                // Check cancellation before each tool execution.
                if cancel.load(Ordering::Relaxed) {
                    messages.push(LlmMessage::tool_result(
                        tc.id.clone(),
                        "Execution cancelled by user".to_string(),
                    ));
                    continue;
                }

                // ENFORCEMENT: If claude_code_execute failed and agent tries
                // to write code via file_write, block it and force stop.
                if claude_code_failed && tc.name == "file_write" {
                    let block_msg = "BLOCKED: file_write is not allowed after claude_code_execute failure. \
                                     The agent must report the error instead of falling back to direct code writing.";
                    let block_payload = serde_json::json!({
                        "tool_name": tc.name,
                        "output": block_msg,
                    });
                    state.event_log.append(&session_id, "tool_result", "coordinator", block_payload.clone()).await;
                    let _ = tx
                        .send(Ok(Event::default().event("tool_result").data(
                            block_payload.to_string(),
                        )))
                        .await;
                    messages.push(LlmMessage::tool_result(tc.id.clone(), block_msg.to_string()));
                    continue;
                }

                // Stream tool_call event.
                let tool_call_payload = serde_json::json!({
                    "tool_name": tc.name,
                    "tool_input": tc.arguments,
                });
                state.event_log.append(&session_id, "tool_call", "coordinator", tool_call_payload.clone()).await;
                let _ = tx
                    .send(Ok(Event::default().event("tool_call").data(
                        tool_call_payload.to_string(),
                    )))
                    .await;
                trace_steps.push(StoredTraceStep {
                    step_type: "tool_call".into(),
                    iteration: None,
                    tool_name: Some(tc.name.clone()),
                    tool_input: Some(tc.arguments.clone()),
                    output: None,
                    content: None,
                });
                persist_running_snapshot("running");

                tools_used.push(tc.name.clone());

                // Update agent status to working (executing tool)
                if let Some(ref id) = agent_id {
                    state.kernel.update_agent_activity(
                        id,
                        macaca_proto::AgentActivity::Working {
                            context: format!("Executing {}", tc.name),
                        },
                    ).await;
                }

                // Execute tool with timeout.
                // Helper: look up tool in todo_tools first, then base tools.
                let find_tool = |name: &str| -> Option<&dyn macaca_tools::Tool> {
                    if let Some(t) = todo_tools.iter().find(|t| t.name() == name) {
                        return Some(t.as_ref());
                    }
                    state.tools.get_tool(name)
                };
                // For claude_code_execute, use streaming to send real-time cc_trace events.
                let result = if tc.name == "claude_code_execute" {
                    // Use streaming version for real-time trace events.
                    if let Some(tool) = find_tool(&tc.name) {
                        // Create unbounded channel for trace events.
                        let (trace_tx, mut trace_rx) =
                            tokio::sync::mpsc::unbounded_channel::<TraceEvent>();

                        // Spawn task to forward trace events to SSE channel.
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            while let Some(event) = trace_rx.recv().await {
                                if let Ok(json) = serde_json::to_string(&event) {
                                    let _ = tx_clone.send(Ok(
                                        Event::default().event("cc_trace").data(json)
                                    )).await;
                                }
                            }
                        });

                        match tokio::time::timeout(
                            std::time::Duration::from_secs(600),
                            tool.execute_streaming(tc.arguments.clone(), Some(trace_tx)),
                        )
                        .await
                        {
                            Ok(Ok(output)) => {
                                serde_json::to_string(&output).unwrap_or_default()
                            }
                            Ok(Err(e)) => format!("Tool error: {e}"),
                            Err(_) => "Tool execution timed out (600s)".to_string(),
                        }
                    } else {
                        format!("Tool '{}' not found", tc.name)
                    }
                } else if let Some(tool) = find_tool(&tc.name) {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(600),
                        tool.execute(tc.arguments.clone()),
                    )
                    .await
                    {
                        Ok(Ok(output)) => {
                            serde_json::to_string(&output).unwrap_or_default()
                        }
                        Ok(Err(e)) => format!("Tool error: {e}"),
                        Err(_) => "Tool execution timed out (600s)".to_string(),
                    }
                } else {
                    format!("Tool '{}' not found", tc.name)
                };

                // Truncate output for streaming display.
                let display_result = if result.len() > 2000 {
                    format!(
                        "{}...[truncated, {} bytes]",
                        &result[..2000],
                        result.len()
                    )
                } else {
                    result.clone()
                };

                // Stream tool_result event.
                let tool_result_payload = serde_json::json!({
                    "tool_name": tc.name,
                    "output": display_result,
                });
                state.event_log.append(&session_id, "tool_result", "coordinator", tool_result_payload.clone()).await;
                let _ = tx
                    .send(Ok(Event::default().event("tool_result").data(
                        tool_result_payload.to_string(),
                    )))
                    .await;
                trace_steps.push(StoredTraceStep {
                    step_type: "tool_result".into(),
                    iteration: None,
                    tool_name: Some(tc.name.clone()),
                    tool_input: None,
                    output: Some(display_result.clone()),
                    content: None,
                });
                persist_running_snapshot("running");

                // Append tool result to conversation for next LLM call.
                messages.push(LlmMessage::tool_result(tc.id.clone(), result.clone()));

                // ===== DETECT delegate_task FORK AND PAUSE LOOP =====
                // When delegate_task returns a fork_id (format: "fork:uuid"),
                // we need to register the fork_to_session mapping and pause the loop
                // until the delegated task completes and ForkValidated is received.
                if tc.name == "delegate_task" {
                    // The result is a JSON object with task_id field containing fork info
                    // Example: {"agent":"backend",...,"task_id":"fork:fork-uuid"}
                    let fork_id_from_json = serde_json::from_str::<serde_json::Value>(&result)
                        .ok()
                        .and_then(|v| v.get("task_id").cloned())
                        .and_then(|v| v.as_str().map(|s| s.to_string()));

                    if let Some(task_id_str) = fork_id_from_json {
                        if let Some(fork_id_str) = task_id_str.strip_prefix("fork:") {
                            // Parse the fork_id
                            let uuid_str = fork_id_str.strip_prefix("fork-").unwrap_or(fork_id_str);
                            if let Ok(fork_uuid) = uuid::Uuid::parse_str(uuid_str) {
                                let fork_id = ForkId(fork_uuid);

                                // Register fork_to_session mapping for hook consumer
                                let mapping = crate::state::ForkSessionMapping {
                                    session_id: session_id.clone(),
                                    app_id: app_id.clone(),
                                    from_agent: agent_id
                                        .as_ref()
                                        .map(|id| id.0.to_string())
                                        .unwrap_or_else(|| "unknown".to_string()),
                                };
                                state.fork_to_session.write().await.insert(fork_id, mapping);

                                tracing::info!(
                                    fork_id = %fork_id,
                                    session_id = %session_id,
                                    "Registered fork_to_session mapping, pausing loop"
                                );

                                // Send fork_created event to SSE
                                let fork_created_payload = serde_json::json!({
                                    "fork_id": fork_id_str,
                                    "session_id": session_id,
                                });
                                state.event_log.append(&session_id, "fork_created", "coordinator", fork_created_payload.clone()).await;
                                let _ = tx.send(Ok(Event::default()
                                    .event("fork_created")
                                    .data(fork_created_payload.to_string())))
                                    .await;

                                // Set pause signal - loop will pause at next iteration
                                pause_signal.store(true, Ordering::SeqCst);

                                // Audit: record the delegation event.
                                state.audit_logger.log_tool(
                                    app_id.clone(),
                                    "coordinator",
                                    "delegate_task",
                                    true,
                                ).await;
                            }
                        }
                    }
                }
                // ===== END DETECT delegate_task =====

                // ===== DETECT create_goal AND PAUSE LOOP =====
                // When create_goal returns a goal_id, pause the coordinator loop
                // until the goal is fully completed (all tasks done + evaluated).
                if tc.name == "create_goal" {
                    let goal_id_from_json = serde_json::from_str::<serde_json::Value>(&result)
                        .ok()
                        .and_then(|v| v.get("goal_id").cloned())
                        .and_then(|v| v.as_str().map(|s| s.to_string()));

                    if let Some(goal_id) = goal_id_from_json {
                        // Register goal_to_session mapping for GoalCompleted handler
                        state.goal_to_session.write().await.insert(goal_id.clone(), session_id.clone());

                        tracing::info!(
                            goal_id = %goal_id,
                            session_id = %session_id,
                            "Registered goal_to_session mapping, pausing coordinator loop"
                        );

                        // Set pause signal - loop will pause at next iteration
                        pause_signal.store(true, Ordering::SeqCst);
                    }
                }
                // ===== END DETECT create_goal =====

                // Track claude_code_execute failures.
                if tc.name == "claude_code_execute" {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                        if let Some(trace) = parsed.get("trace") {
                            if let Ok(steps) = serde_json::from_value::<Vec<TraceEvent>>(trace.clone()) {
                                cc_trace_steps.extend(steps);
                                persist_running_snapshot("running");
                            }
                        }
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                        if parsed.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false) {
                            claude_code_failed = true;
                        }
                    }
                }
            }

            continue; // Next agentic loop iteration.
        }

        // No tool calls — final assistant response.
        final_content = response.content.clone();
        persist_running_snapshot(&status);
        messages.push(LlmMessage::assistant(response.content));

        // Stream final content event.
        let content_payload = serde_json::json!({"content": final_content});
        state.event_log.append(&session_id, "content", "coordinator", content_payload.clone()).await;
        let _ = tx
            .send(Ok(Event::default()
                .event("content")
                .data(content_payload.to_string())))
            .await;

        break;
    }

    // Handle max iterations reached.
    if iterations >= max_iterations && final_content.is_empty() {
        final_content =
            "Max iterations reached. The agent may not have completed the task.".into();
        let content_max_payload = serde_json::json!({"content": final_content});
        state.event_log.append(&session_id, "content", "coordinator", content_max_payload.clone()).await;
        let _ = tx
            .send(Ok(Event::default()
                .event("content")
                .data(content_max_payload.to_string())))
            .await;
    }

    // NOTE: Do NOT remove session from active_sessions here.
    // PlanLoop and WorkerLoop events arrive after the coordinator loop ends,
    // and they need sse_tx to broadcast plan_decision events to the frontend.
    // The session will be cleaned up when the browser disconnects or a new session starts.
    tracing::info!(session_id = %session_id, "Coordinator loop ended, session kept alive for plan events");

    // Set agent status back to idle
    if let Some(ref id) = agent_id {
        state.kernel.update_agent_activity(
            id,
            macaca_proto::AgentActivity::Idle,
        ).await;
    }

    Ok(AssistantRunResult {
        content: final_content,
        usage: (prompt_tokens, completion_tokens, total_tokens),
        iterations,
        tools_used,
        status,
        trace_steps,
        cc_trace_steps,
        agent_traces: agent_trace_collector.get_all().await,
    })
}

// ---------------------------------------------------------------------------
// Todo Board API
// ---------------------------------------------------------------------------

/// Optional session_id filter for todo/goal queries.
#[derive(Deserialize, Default)]
pub struct SessionQuery {
    #[serde(default)]
    session_id: Option<String>,
}

/// GET /api/apps/{app_id}/todos — list todos (optionally filtered by session_id)
pub async fn list_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let store = Arc::clone(&state.todo_store);
    let mut todos = if let Some(ref sid) = query.session_id {
        store.list_all_todos_for_session(&app_id, sid).await
    } else {
        store.list_all_todos(&app_id).await
    };
    todos.sort_by_key(|t| t.sequence_number);
    Ok(Json(serde_json::json!({ "todos": todos, "count": todos.len() })))
}

/// GET /api/apps/{app_id}/todos/progress — overall progress (optionally filtered by session_id)
pub async fn get_todo_progress(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let store = Arc::clone(&state.todo_store);
    let space = macaca_task::TaskSpace::new(app_id, query.session_id, store);
    let p = space.overall_progress().await;
    Ok(Json(serde_json::json!({
        "total": p.total, "pending": p.pending, "assigned": p.assigned,
        "in_progress": p.in_progress, "pending_review": p.pending_review,
        "needs_optimization": p.needs_optimization, "completed": p.completed,
        "blocked": p.blocked, "failed": p.failed, "cancelled": p.cancelled,
        "all_done": space.all_tasks_done().await,
    })))
}

/// GET /api/apps/{app_id}/todos/{agent_name} — list agent's board
pub async fn list_agent_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, agent_name)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let store = Arc::clone(&state.todo_store);
    let mut todos = store.list_agent_todos(&app_id, &None, &agent_name).await;
    todos.sort_by_key(|t| t.sequence_number);
    Ok(Json(serde_json::json!({ "agent": agent_name, "todos": todos, "count": todos.len() })))
}

/// GET /api/apps/{app_id}/goals — list goals (optionally filtered by session_id)
pub async fn list_goals(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let store = Arc::clone(&state.todo_store);
    let goals = if let Some(ref sid) = query.session_id {
        store.list_goals_for_session(&app_id, sid).await
    } else {
        store.list_goals(&app_id).await
    };
    Ok(Json(serde_json::json!({ "goals": goals, "count": goals.len() })))
}

/// Send an SSE event to all active sessions belonging to a given application.
///
/// PlanLoop/WorkerLoop events are app-scoped (one loop per app), but SSE
/// connections are session-scoped. This broadcasts the same event to every
/// open browser tab that is watching the app.
///
/// `log_payload` is persisted to the EventLog for every matching session before
/// the SSE send, ensuring durability even if the browser connection drops.
async fn broadcast_to_app_sessions(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    event: Event,
    log_payload: serde_json::Value,
) {
    let sessions = state.active_sessions.read().await;
    let total = sessions.len();
    let matching: Vec<_> = sessions.values()
        .filter(|s| &s.app_id == app_id)
        .collect();
    tracing::info!(
        total_sessions = total,
        matching_sessions = matching.len(),
        app_id = %app_id,
        "Broadcasting plan_decision to app sessions"
    );
    // Append to EventLog BEFORE SSE send (durability guarantee).
    for session in &matching {
        state.event_log.append(
            &session.session_id,
            "plan_decision",
            "plan_loop",
            log_payload.clone(),
        ).await;
    }
    for session in &matching {
        let tx = session.sse_tx.read().await;
        match tx.try_send(Ok(event.clone())) {
            Ok(()) => tracing::info!(session_id = %session.session_id, "plan_decision sent"),
            Err(e) => tracing::warn!(session_id = %session.session_id, error = %e, "plan_decision send failed"),
        }
    }
}

/// Ensure PlanLoop and WorkerLoops are running for an application.
/// Idempotent — safe to call multiple times.
pub async fn ensure_plan_and_worker_loops(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) {
    // Determine which agent handles planning (decomposition + review).
    // Convention: if the app has a "planner" agent registered, use it; otherwise fall back to "coordinator".
    let plan_agent_name: String = if let Some(executor) = state.executor_registry.get(app_id).await {
        let agents = executor.list_agents().await;
        if agents.iter().any(|a| a.name == "planner") {
            "planner".to_string()
        } else {
            "coordinator".to_string()
        }
    } else {
        "coordinator".to_string()
    };

    // ── Migrate legacy tasks (one-time, idempotent) ──
    state.todo_store.migrate_sequence_numbers(app_id).await;

    // ── PlanLoop ──
    {
        let already = state.plan_loop_handles.read().await.contains_key(app_id);
        if !already {
            let mut handles = state.plan_loop_handles.write().await;
            if !handles.contains_key(app_id) {
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                handles.insert(app_id.clone(), Arc::clone(&shutdown));

                let store = Arc::clone(&state.todo_store);
                let plan_space = Arc::new(macaca_task::TaskSpace::new(app_id.clone(), None, Arc::clone(&store)));
                let plan_loop = macaca_task::PlanLoop::new(plan_space, macaca_task::PlanLoopConfig::default());
                let plan_waker = plan_loop.waker();
                state.plan_loop_wakers.write().await.insert(app_id.clone(), plan_waker);

                let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<macaca_task::PlanEvent>(64);
                let event_tx_for_consumer = event_tx.clone();

                tokio::spawn(async move {
                    plan_loop.run(shutdown, event_tx).await;
                });

                let state_for_consumer = Arc::clone(state);
                let app_id_for_consumer = app_id.clone();
                let session_store_for_consumer = Arc::clone(&state.session_store);
                let plan_agent_name_for_loop = plan_agent_name.clone();
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        match event {
                            macaca_task::PlanEvent::GoalReady { goal_id, description, session_id } => {
                                if let Some(executor) = state_for_consumer.executor_registry.get(&app_id_for_consumer).await {
                                    // Dynamically get available agent names from executor (no hardcoding)
                                    let agents = executor.list_agents().await;
                                    let agent_names: Vec<String> = agents.iter()
                                        .map(|a| a.name.clone())
                                        .filter(|n| n != "coordinator" && n != &plan_agent_name_for_loop)
                                        .collect();
                                    let agents_list = if agent_names.is_empty() {
                                        "no worker agents registered".to_string()
                                    } else {
                                        agent_names.join(", ")
                                    };
                                    let prompt = format!(
                                        "A new project goal has been submitted. Decompose it into concrete tasks \
                                         and assign each to the appropriate agent using create_todo.\n\n\
                                         Goal: {}\n\n\
                                         Available agents: {}.\n\
                                         Set priorities (0-10) and acceptance_criteria for each task.",
                                        description, agents_list
                                    );
                                    let ctx = session_id.clone().map(|sid| macaca_kernel::TaskContext {
                                        session_id: Some(sid),
                                        artifacts: vec![],
                                        env: std::collections::HashMap::new(),
                                    });
                                    let _ = executor.delegate_task("plan_loop", &plan_agent_name_for_loop, prompt, 9, false, ctx).await;
                                    // Note: don't wake workers here — planner hasn't created tasks yet.
                                    // Workers will be woken when tasks actually appear on their boards.
                                }
                                // Emit SSE decision event
                                let msg = format!("New goal submitted, decomposing into tasks: {}", description);
                                let plan_payload = serde_json::json!({
                                    "decision_type": "goal_ready",
                                    "goal_id": goal_id.to_string(),
                                    "description": description,
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
                                // Persist decision
                                    save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                        decision_type: "goal_ready".into(),
                                        message: msg,
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": description }),
                                    }).await;
                            }
                            macaca_task::PlanEvent::ReviewNeeded { task_id, agent, title, summary, criteria, session_id } => {
                                if let Some(executor) = state_for_consumer.executor_registry.get(&app_id_for_consumer).await {
                                    let prompt = format!(
                                        "Review this completed task using review_todo:\n\
                                         Task ID: {}\n Agent: {}\n Title: {}\n\
                                         Summary: {}\n Criteria: {:?}\n\n\
                                         Verify the work meets the criteria. Use review_todo with passed=true/false and feedback.",
                                        task_id, agent, title, summary, criteria
                                    );
                                    let ctx = session_id.clone().map(|sid| macaca_kernel::TaskContext {
                                        session_id: Some(sid),
                                        artifacts: vec![],
                                        env: std::collections::HashMap::new(),
                                    });
                                    let _ = executor.delegate_task("plan_loop", &plan_agent_name_for_loop, prompt, 8, false, ctx).await;
                                    // Review delegate completed — wake all WorkerLoops so unblocked tasks are claimed immediately
                                    if let Some(wakers) = state_for_consumer.worker_loop_wakers.read().await.get(&app_id_for_consumer) {
                                        for w in wakers { w.wake(); }
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
                                    broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, review_event, review_payload).await;
                                }
                                // Emit SSE decision event
                                let msg = format!("Task '{}' submitted for review by {}", title, agent);
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
                                broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
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
                            }
                            macaca_task::PlanEvent::AnomalyDetected { ref message } => {
                                tracing::warn!(message, "Plan loop anomaly detected");
                                state_for_consumer.alert_manager.fire(
                                    macaca_kernel::alert::Alert::warning("Task Anomaly", message.clone(), "plan_loop")
                                ).await;
                                // Emit SSE decision event
                                let msg = message.clone();
                                let plan_payload = serde_json::json!({
                                    "decision_type": "anomaly",
                                    "message": msg,
                                });
                                let sse_event = Event::default()
                                    .event("plan_decision")
                                    .data(plan_payload.to_string());
                                broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
                                // Persist decision
                                    save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                        decision_type: "anomaly".into(),
                                        message: msg.clone(),
                                        timestamp: chrono::Utc::now(),
                                        data: serde_json::json!({ "message": msg }),
                                    }).await;
                            }
                            macaca_task::PlanEvent::EvaluateGoalCompletion { goal_id, goal_description, completed_count, failed_count, task_summaries, session_id } => {
                                tracing::info!(goal_id = %goal_id, "Evaluating goal completion");
                                let evaluator = macaca_task::GoalEvaluator::new(
                                    Arc::clone(&state_for_consumer.llm),
                                    state_for_consumer.default_model.clone(),
                                );
                                match evaluator.evaluate(&goal_description, &task_summaries, completed_count, failed_count).await {
                                    Ok(macaca_task::GoalEvaluation::Satisfied { summary }) => {
                                        tracing::info!(goal_id = %goal_id, summary = %summary, "Goal satisfied");
                                        let space = macaca_task::TaskSpace::new(app_id_for_consumer.clone(), None, Arc::clone(&state_for_consumer.todo_store));
                                        space.complete_goal(&goal_id).await;
                                        let _ = event_tx_for_consumer.send(macaca_task::PlanEvent::GoalCompleted { goal_id: goal_id.clone(), description: goal_description.clone() }).await;
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
                                        broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
                                        // Persist decision
                                            save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                                decision_type: "goal_satisfied".into(),
                                                message: msg,
                                                timestamp: chrono::Utc::now(),
                                                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "summary": summary }),
                                            }).await;
                                    }
                                    Ok(macaca_task::GoalEvaluation::NeedsMoreWork { reason, suggestions }) => {
                                        tracing::info!(goal_id = %goal_id, reason = %reason, "Goal needs more work");
                                        if let Some(executor) = state_for_consumer.executor_registry.get(&app_id_for_consumer).await {
                                            let prompt = format!(
                                                "The goal '{}' needs additional work. Reason: {}\nSuggestions:\n{}\n\nCreate follow-up tasks using create_todo.",
                                                goal_description, reason,
                                                suggestions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
                                            );
                                            let ctx = session_id.clone().map(|sid| macaca_kernel::TaskContext {
                                                session_id: Some(sid),
                                                artifacts: vec![],
                                                env: std::collections::HashMap::new(),
                                            });
                                            let _ = executor.delegate_task("plan_loop", &plan_agent_name_for_loop, prompt, 8, false, ctx).await;
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
                                        broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
                                        // Persist decision
                                            save_plan_decision(&session_store_for_consumer, &app_id_for_consumer, PlanDecisionEvent {
                                                decision_type: "goal_needs_work".into(),
                                                message: msg,
                                                timestamp: chrono::Utc::now(),
                                                data: serde_json::json!({ "goal_id": goal_id.to_string(), "description": goal_description, "reason": reason, "suggestions": suggestions }),
                                            }).await;
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "GoalEvaluator failed, marking complete by default");
                                        let space = macaca_task::TaskSpace::new(app_id_for_consumer.clone(), None, Arc::clone(&state_for_consumer.todo_store));
                                        space.complete_goal(&goal_id).await;
                                    }
                                }
                            }
                            macaca_task::PlanEvent::GoalCompleted { goal_id, description } => {
                                tracing::info!(goal_id = %goal_id, "Goal completed: {}", description);

                                // ===== RESUME COORDINATOR IF PAUSED =====
                                let goal_id_str = goal_id.to_string();
                                let waiting_session = state_for_consumer.goal_to_session.write().await.remove(&goal_id_str);
                                if let Some(sid) = waiting_session {
                                    let sessions = state_for_consumer.active_sessions.read().await;
                                    if let Some(session) = sessions.get(&sid) {
                                        let resume_reason = macaca_runtime::agentic_loop::ResumeReason::DelegateCompleted {
                                            task_id: goal_id_str.clone(),
                                            success: true,
                                            output: format!("Goal completed: {}", description),
                                        };
                                        session.pause_signal.store(false, std::sync::atomic::Ordering::SeqCst);
                                        let _ = session.resume_tx.send(resume_reason).await;
                                        tracing::info!(goal_id = %goal_id, session_id = %sid, "Resumed coordinator after goal completion");
                                    }
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
                                broadcast_to_app_sessions(&state_for_consumer, &app_id_for_consumer, sse_event, plan_payload).await;
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
        let already = state.worker_loop_handles.read().await.contains_key(app_id);
        if !already {
            if let Some(executor) = state.executor_registry.get(app_id).await {
                let agents = executor.list_agents().await;
                let mut shutdowns: Vec<Arc<std::sync::atomic::AtomicBool>> = Vec::new();
                let mut worker_wakers: Vec<macaca_task::WorkerLoopWaker> = Vec::new();

                for agent_info in &agents {
                    let agent_name = agent_info.name.clone();
                    // Skip agents that handle planning — they don't pull from the TaskBoard.
                    // "coordinator" is always excluded (handles user interaction).
                    // plan_agent_name is excluded too (handles decomposition + review).
                    if agent_name == "coordinator" || agent_name == plan_agent_name {
                        continue;
                    }

                    let board = Arc::new(macaca_task::TaskBoard::new(
                        app_id.clone(), agent_name.clone(), None, Arc::clone(&state.todo_store),
                    ));
                    let worker_loop = macaca_task::WorkerLoop::new(Arc::clone(&board), macaca_task::WorkerLoopConfig::default());
                    worker_wakers.push(worker_loop.waker());
                    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    shutdowns.push(Arc::clone(&shutdown));

                    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<macaca_task::WorkerEvent>(32);
                    let shutdown_clone = Arc::clone(&shutdown);
                    tokio::spawn(async move { worker_loop.run(shutdown_clone, event_tx).await; });

                    let executor_clone = Arc::clone(&executor);
                    let agent_name_clone = agent_name.clone();
                    let board_clone = Arc::clone(&board);
                    let state_for_worker = Arc::clone(state);
                    let app_id_for_worker = app_id.clone();
                    let session_store_for_worker = Arc::clone(&state.session_store);
                    tokio::spawn(async move {
                        while let Some(event) = event_rx.recv().await {
                            match event {
                                macaca_task::WorkerEvent::TaskClaimed { task_id, title, description, acceptance_criteria, context, optimization_suggestions, .. } => {
                                    let mut prompt = format!("Execute this task:\n\nTitle: {}\nDescription: {}", title, description);
                                    if !acceptance_criteria.is_empty() {
                                        prompt.push_str(&format!("\n\nAcceptance Criteria:\n{}", acceptance_criteria.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n")));
                                    }
                                    if let Some(ctx) = context { prompt.push_str(&format!("\n\nContext: {}", ctx)); }
                                    if let Some(sug) = optimization_suggestions { prompt.push_str(&format!("\n\nOptimization: {}", sug)); }
                                    tracing::info!(agent = %agent_name_clone, title = %title, "WorkerLoop claimed task");
                                    // Emit SSE decision event for task claimed
                                    let msg = format!("Agent '{}' claimed task: {}", agent_name_clone, title);
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
                                    broadcast_to_app_sessions(&state_for_worker, &app_id_for_worker, sse_event, plan_payload).await;
                                    // Persist decision
                                        save_plan_decision(&session_store_for_worker, &app_id_for_worker, PlanDecisionEvent {
                                            decision_type: "task_claimed".into(),
                                            message: msg,
                                            timestamp: chrono::Utc::now(),
                                            data: serde_json::json!({ "task_id": task_id.to_string(), "agent": agent_name_clone, "title": title }),
                                        }).await;
                                    // delegate_task returns a task_id. We must wait for execution
                                    // to complete, then update the TaskBoard based on the result.
                                    match executor_clone.delegate_task("worker_loop", &agent_name_clone, prompt, 7, false, None).await {
                                        Ok(exec_task_id) => {
                                            // Poll for result (executor stores it when agent finishes)
                                            loop {
                                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                                if let Some(result) = executor_clone.get_task_result(exec_task_id).await {
                                                    if result.success {
                                                        let summary = if result.output.is_empty() { format!("Task '{}' completed", title) } else { result.output.clone() };
                                                        board_clone.submit_for_review(&task_id, summary).await;
                                                        // Wake PlanLoop immediately so review is detected without waiting for heartbeat
                                                        if let Some(waker) = state_for_worker.plan_loop_wakers.read().await.get(&app_id_for_worker) {
                                                            waker.wake();
                                                        }
                                                        tracing::info!(agent = %agent_name_clone, "Task completed, submitted for review");
                                                    } else {
                                                        let error = result.error.unwrap_or_else(|| "Unknown error".into());
                                                        board_clone.mark_failed(&task_id, error).await;
                                                        tracing::error!(agent = %agent_name_clone, "Task execution failed");
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(agent = %agent_name_clone, error = %e, "Failed to delegate task");
                                            board_clone.mark_failed(&task_id, format!("Delegation failed: {}", e)).await;
                                        }
                                    }
                                }
                                macaca_task::WorkerEvent::RetryTask { task_id, title, description, optimization_suggestions, .. } => {
                                    let prompt = format!("Retry task:\n\nTitle: {}\nDescription: {}\n\nFeedback: {}", title, description, optimization_suggestions);
                                    match executor_clone.delegate_task("worker_loop", &agent_name_clone, prompt, 7, false, None).await {
                                        Ok(exec_task_id) => {
                                            loop {
                                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                                if let Some(result) = executor_clone.get_task_result(exec_task_id).await {
                                                    if result.success {
                                                        let summary = if result.output.is_empty() { format!("Task '{}' completed on retry", title) } else { result.output.clone() };
                                                        board_clone.submit_for_review(&task_id, summary).await;
                                                        if let Some(waker) = state_for_worker.plan_loop_wakers.read().await.get(&app_id_for_worker) {
                                                            waker.wake();
                                                        }
                                                    } else {
                                                        let error = result.error.unwrap_or_else(|| "Unknown error".into());
                                                        board_clone.mark_failed(&task_id, error).await;
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            board_clone.mark_failed(&task_id, format!("Retry failed: {}", e)).await;
                                        }
                                    }
                                }
                                macaca_task::WorkerEvent::Idle => {}
                            }
                        }
                    });
                }
                state.worker_loop_handles.write().await.insert(app_id.clone(), shutdowns);
                state.worker_loop_wakers.write().await.insert(app_id.clone(), worker_wakers);
                tracing::info!(app_id = %app_id, "WorkerLoops started for app");
            }
        }
    }
}

/// POST /api/apps/{app_id}/goals — create a new goal
pub async fn create_goal(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let description = body["description"].as_str()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing 'description' field".into()))?;
    let store = Arc::clone(&state.todo_store);
    let session_id = body["session_id"].as_str().map(|s| s.to_string());
    let space = macaca_task::TaskSpace::new(app_id.clone(), session_id, Arc::clone(&store));
    let goal = space.push_goal(description).await;

    // Start PlanLoop + WorkerLoops if not already running
    ensure_plan_and_worker_loops(&state, &app_id).await;

    Ok(Json(serde_json::json!({ "goal_id": goal.id.to_string(), "status": "pending" })))
}

// ---------------------------------------------------------------------------
// Schedule routes
// GET    /api/apps/{app_id}/schedules           → list_schedules
// POST   /api/apps/{app_id}/schedules           → create_schedule
// GET    /api/apps/{app_id}/schedules/{id}      → get_schedule
// DELETE /api/apps/{app_id}/schedules/{id}      → delete_schedule
// PUT    /api/apps/{app_id}/schedules/{id}/toggle → toggle_schedule
// ---------------------------------------------------------------------------

/// GET /api/apps/{app_id}/schedules — list all schedules for an application
pub async fn list_schedules(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let entries = scheduler.list(&app_id).await;
    Ok(Json(serde_json::json!({ "schedules": entries, "count": entries.len() })))
}

/// POST /api/apps/{app_id}/schedules — create a new schedule
///
/// Body (JSON):
/// ```json
/// {
///   "name": "daily-report",
///   "cron_expr": "0 9 * * *",          // optional; use this OR interval_secs
///   "interval_secs": 3600,              // optional; use this OR cron_expr
///   "action": {
///     "kind": "create_goal",
///     "description": "Generate daily report"
///   }
/// }
/// ```
pub async fn create_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);

    let name = body["name"].as_str()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing 'name' field".into()))?
        .to_owned();

    let action: macaca_task::ScheduleAction = serde_json::from_value(body["action"].clone())
        .map_err(|e| err(StatusCode::BAD_REQUEST, format!("Invalid 'action': {e}")))?;

    let entry = if let Some(expr) = body["cron_expr"].as_str() {
        macaca_task::ScheduleEntry::new_cron(app_id.clone(), name, expr, action)
    } else if let Some(secs) = body["interval_secs"].as_u64() {
        macaca_task::ScheduleEntry::new_interval(app_id.clone(), name, secs, action)
    } else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "One of 'cron_expr' or 'interval_secs' is required".into(),
        ));
    };

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let created = scheduler.create(entry).await;

    // Lazily start Scheduler loop for this app if not already running
    {
        let handles = state.scheduler_handles.read().await;
        if !handles.contains_key(&app_id) {
            drop(handles);
            let mut handles = state.scheduler_handles.write().await;
            if !handles.contains_key(&app_id) {
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                handles.insert(app_id.clone(), Arc::clone(&shutdown));

                let sched_runner = macaca_task::TaskScheduler::new(
                    Arc::clone(&state.session_store),
                    macaca_task::SchedulerConfig::default(),
                );
                let (sched_event_tx, mut sched_event_rx) =
                    tokio::sync::mpsc::channel::<macaca_task::ScheduleEvent>(64);
                let app_id_sched = app_id.clone();

                tokio::spawn(async move {
                    sched_runner.run(app_id_sched, shutdown, sched_event_tx).await;
                });

                let state_for_sched = Arc::clone(&state);
                let app_id_for_sched = app_id.clone();
                tokio::spawn(async move {
                    while let Some(event) = sched_event_rx.recv().await {
                        match event {
                            macaca_task::ScheduleEvent::Triggered { action, .. } => {
                                match action {
                                    macaca_task::ScheduleAction::CreateGoal { description } => {
                                        let space = macaca_task::TaskSpace::new(
                                            app_id_for_sched.clone(),
                                            None,
                                            Arc::clone(&state_for_sched.todo_store),
                                        );
                                        space.push_goal(description).await;
                                    }
                                    macaca_task::ScheduleAction::CreateTask { agent, title, description, priority } => {
                                        let space = macaca_task::TaskSpace::new(
                                            app_id_for_sched.clone(),
                                            None,
                                            Arc::clone(&state_for_sched.todo_store),
                                        );
                                        space.create_and_assign(
                                            &agent, "scheduler", &title, &description,
                                            vec![], priority, vec![], None,
                                        ).await;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::info!(app_id = %app_id, "Scheduler started for app");
            }
        }
    }

    Ok(Json(serde_json::json!({
        "schedule_id": created.id.to_string(),
        "name": created.name,
        "next_run_at": created.next_run_at,
        "enabled": created.enabled,
    })))
}

/// GET /api/apps/{app_id}/schedules/{id} — get a single schedule
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let id = macaca_proto::TaskId(uuid::Uuid::parse_str(&schedule_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?);

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let entry = scheduler.get(&app_id, &id).await
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Schedule not found".into()))?;
    Ok(Json(serde_json::to_value(&entry)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?))
}

/// DELETE /api/apps/{app_id}/schedules/{id} — delete a schedule
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let id = macaca_proto::TaskId(uuid::Uuid::parse_str(&schedule_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?);

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    scheduler.delete(&app_id, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/apps/{app_id}/schedules/{id}/toggle — enable or disable a schedule
///
/// Body: `{ "enabled": true }` or `{ "enabled": false }`
pub async fn toggle_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(uuid::Uuid::parse_str(&app_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?);
    let id = macaca_proto::TaskId(uuid::Uuid::parse_str(&schedule_id)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?);
    let enabled = body["enabled"].as_bool()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing boolean 'enabled' field".into()))?;

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let ok = scheduler.set_enabled(&app_id, &id, enabled).await;
    if ok {
        Ok(Json(serde_json::json!({ "schedule_id": schedule_id, "enabled": enabled })))
    } else {
        Err(err(StatusCode::NOT_FOUND, "Schedule not found".into()))
    }
}

// ---------------------------------------------------------------------------
// Event Log API
// ---------------------------------------------------------------------------

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub since: Option<u64>,
    pub limit: Option<usize>,
}

/// Response payload for the events endpoint.
#[derive(Serialize)]
pub struct EventsResponse {
    pub events: Vec<macaca_proto::EventEntry>,
    pub latest_seq: u64,
}

/// GET /api/sessions/:id/events?since={seq}&limit={n}
///
/// Returns persisted events for a session. Clients can poll by passing the
/// last received `latest_seq` as `since` to get only new events.
pub async fn get_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit = params.limit.unwrap_or(500);
    let events = state.event_log.query(&session_id, since, limit).await;
    let latest_seq = state.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_turn(role: &str, content: &str, status: Option<&str>) -> StoredTurn {
        StoredTurn {
            role: role.into(),
            content: content.into(),
            status: status.map(String::from),
            trace_steps: Vec::new(),
            cc_trace_steps: Vec::new(),
            meta: None,
            agent_traces: std::collections::HashMap::new(),
        }
    }

    fn make_turn_with_traces(role: &str, content: &str, status: Option<&str>, agents: Vec<&str>) -> StoredTurn {
        let mut agent_traces = std::collections::HashMap::new();
        for agent in agents {
            agent_traces.insert(agent.to_string(), vec![AgentTrace {
                task_id: format!("task-{agent}"),
                agent: agent.to_string(),
                status: "completed".to_string(),
                steps: vec![],
                output: Some("done".to_string()),
                error: None,
            }]);
        }
        StoredTurn {
            role: role.into(),
            content: content.into(),
            status: status.map(String::from),
            trace_steps: Vec::new(),
            cc_trace_steps: Vec::new(),
            meta: None,
            agent_traces,
        }
    }

    #[test]
    fn test_dedup_removes_snapshot_running_turn() {
        // Simulate: snapshot saved [user, assistant(running)], then final save appends new pair
        let prompt = "hello".to_string();
        let mut turns = vec![
            make_turn("user", "hello", None),
            make_turn_with_traces("assistant", "partial...", Some("running"), vec!["backend"]),
        ];

        // Apply the dedup logic (same as in the success path)
        if let Some(pos) = turns.iter().rposition(|t| {
            t.role == "assistant"
                && matches!(t.status.as_deref(), Some("running") | Some("pending"))
        }) {
            turns.remove(pos);
            if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
                turns.remove(pos - 1);
            }
        }

        // After dedup, should be empty (both snapshot turns removed)
        assert!(turns.is_empty(), "snapshot running turn and its user turn should be removed");

        // Now push final turns
        turns.push(make_turn("user", "hello", None));
        turns.push(make_turn_with_traces("assistant", "final answer", Some("completed"), vec!["backend"]));

        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].role, "user");
        assert_eq!(turns[1].role, "assistant");
        assert_eq!(turns[1].status.as_deref(), Some("completed"));
        assert!(!turns[1].agent_traces.is_empty());
    }

    #[test]
    fn test_dedup_preserves_prior_conversation_turns() {
        // Simulate: prior completed turns + snapshot running turn
        let prompt = "second question".to_string();
        let mut turns = vec![
            make_turn("user", "first question", None),
            make_turn_with_traces("assistant", "first answer", Some("completed"), vec!["backend"]),
            make_turn("user", "second question", None),
            make_turn_with_traces("assistant", "partial...", Some("running"), vec!["tester"]),
        ];

        if let Some(pos) = turns.iter().rposition(|t| {
            t.role == "assistant"
                && matches!(t.status.as_deref(), Some("running") | Some("pending"))
        }) {
            turns.remove(pos);
            if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
                turns.remove(pos - 1);
            }
        }

        // Should keep the first completed pair, remove the snapshot pair
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "first question");
        assert_eq!(turns[1].content, "first answer");
        assert_eq!(turns[1].status.as_deref(), Some("completed"));
    }

    #[test]
    fn test_dedup_noop_when_no_running_turn() {
        let prompt = "hello".to_string();
        let mut turns = vec![
            make_turn("user", "hello", None),
            make_turn_with_traces("assistant", "done", Some("completed"), vec!["backend"]),
        ];

        if let Some(pos) = turns.iter().rposition(|t| {
            t.role == "assistant"
                && matches!(t.status.as_deref(), Some("running") | Some("pending"))
        }) {
            turns.remove(pos);
            if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
                turns.remove(pos - 1);
            }
        }

        // No running turn, so nothing removed
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[1].status.as_deref(), Some("completed"));
    }

    #[test]
    fn test_dedup_handles_pending_status() {
        let prompt = "test".to_string();
        let mut turns = vec![
            make_turn("user", "test", None),
            make_turn("assistant", "thinking...", Some("pending")),
        ];

        if let Some(pos) = turns.iter().rposition(|t| {
            t.role == "assistant"
                && matches!(t.status.as_deref(), Some("running") | Some("pending"))
        }) {
            turns.remove(pos);
            if pos > 0 && turns[pos - 1].role == "user" && turns[pos - 1].content == prompt {
                turns.remove(pos - 1);
            }
        }

        assert!(turns.is_empty(), "pending turn should also be removed");
    }

    #[test]
    fn test_agent_trace_serialization_roundtrip() {
        let turn = make_turn_with_traces("assistant", "answer", Some("completed"), vec!["backend", "tester"]);
        let json = serde_json::to_string(&turn).unwrap();
        let deserialized: StoredTurn = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_traces.len(), 2);
        assert!(deserialized.agent_traces.contains_key("backend"));
        assert!(deserialized.agent_traces.contains_key("tester"));
        assert_eq!(deserialized.agent_traces["backend"][0].task_id, "task-backend");
        assert_eq!(deserialized.agent_traces["tester"][0].status, "completed");
    }

    #[test]
    fn test_agent_trace_empty_skipped_in_json() {
        let turn = make_turn("assistant", "answer", Some("completed"));
        let json = serde_json::to_string(&turn).unwrap();

        // agent_traces with skip_serializing_if should not appear in JSON when empty
        assert!(!json.contains("agent_traces"), "empty agent_traces should be skipped in JSON");
    }

    #[test]
    fn test_ensure_running_assistant_turn_creates_new() {
        let mut turns = vec![
            make_turn("user", "hi", None),
        ];
        let turn = ensure_running_assistant_turn(&mut turns);
        assert_eq!(turn.role, "assistant");
        assert_eq!(turn.status.as_deref(), Some("running"));
        assert_eq!(turns.len(), 2);
    }

    #[test]
    fn test_ensure_running_assistant_turn_reuses_existing() {
        let mut turns = vec![
            make_turn("user", "hi", None),
            make_turn("assistant", "partial", Some("running")),
        ];
        let turn = ensure_running_assistant_turn(&mut turns);
        assert_eq!(turn.content, "partial");
        assert_eq!(turns.len(), 2); // no new turn added
    }
}
