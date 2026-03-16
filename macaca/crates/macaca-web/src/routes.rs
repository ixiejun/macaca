//! API route handlers for the Agent OS web server.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use macaca_kernel::executor::ExecutorEvent;
use macaca_persist::PersistStore;
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
struct AgentTraceCollector {
    traces: RwLock<std::collections::HashMap<String, Vec<AgentTrace>>>,
    /// Maps task_id to agent name for looking up agent when TaskCompleted/TaskFailed is received
    task_to_agent: RwLock<std::collections::HashMap<String, String>>,
}

impl AgentTraceCollector {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            traces: RwLock::new(std::collections::HashMap::new()),
            task_to_agent: RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Called when executor emits TaskStarted - creates new trace
    async fn on_task_started(&self, task_id: &str, agent: &str) {
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
    }

    /// Called when executor emits AgentEvent - adds step to trace
    async fn on_agent_event(&self, task_id: &str, agent: &str, event: &macaca_proto::AgentExecutionEvent) {
        let mut traces = self.traces.write().await;
        if let Some(agent_traces) = traces.get_mut(agent) {
            if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
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
    async fn on_task_completed(&self, task_id: &str, success: bool, output: Option<String>, error: Option<String>) {
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
    async fn get_all(&self) -> std::collections::HashMap<String, Vec<AgentTrace>> {
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
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
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

    let turns = if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    };

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
    "qwen3-max".into()
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

// ---------------------------------------------------------------------------
// POST /api/chat/stop — Cancel a running agentic loop
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct StopRequest {
    pub app_id: String,
}

pub async fn post_stop(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StopRequest>,
) -> Json<ErrorResponse> {
    let flags = state.cancel_flags.read().await;
    if let Some(flag) = flags.get(&req.app_id) {
        flag.store(true, Ordering::Relaxed);
        Json(ErrorResponse {
            error: "stopped".into(),
        })
    } else {
        Json(ErrorResponse {
            error: "no active task".into(),
        })
    }
}

/// POST /api/chat — returns an SSE stream of agentic loop events.
///
/// SSE event types:
/// - `thinking`     — LLM is processing (iteration number)
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
    let coordinator = discovered_app
        .as_ref()
        .and_then(|a| a.manifest.workflows.as_ref())
        .and_then(|w| w.values().next())
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
    let full_system = if catalog_prompt.is_empty() {
        system_prompt
    } else {
        format!("{system_prompt}\n\n{catalog_prompt}")
    };

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
    drop(sessions);

    messages.push(LlmMessage::user(req.prompt.clone()));

    // 5. Create SSE channel, cancel flag, and spawn the agentic loop task.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let cancel_flag = Arc::new(AtomicBool::new(false));

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

    tokio::spawn(async move {
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
            session_key.clone(),
        ).await;

        match result {
            Ok(run) => {
                // Send done event.
                let _ = tx
                    .send(Ok(Event::default().event("done").data(
                        serde_json::json!({
                            "model": model,
                            "tokens": {
                                "prompt": run.usage.0,
                                "completion": run.usage.1,
                                "total": run.usage.2,
                            },
                            "iterations": run.iterations,
                            "tools_used": run.tools_used,
                        })
                        .to_string(),
                    )))
                    .await;

                // Save conversation history to in-memory cache.
                let mut sessions = state_clone.sessions.write().await;
                let hist = sessions.entry(session_key.clone()).or_insert_with(Vec::new);
                hist.push(LlmMessage::user(prompt.clone()));
                hist.push(LlmMessage::assistant(run.content.clone()));
                let hist_snapshot = hist.clone();
                drop(sessions);

                // Persist session to redb store.
                let session_id = session_key.clone();
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
                    agent_traces: collector_for_save.get_all().await,
                });
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
                let hist = sessions.entry(session_key.clone()).or_insert_with(Vec::new);
                hist.push(LlmMessage::user(prompt.clone()));
                hist.push(LlmMessage::assistant(format!("Error: {e}")));
                let hist_snapshot = hist.clone();
                drop(sessions);

                let session_id = session_key.clone();
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

        // Clean up cancel flag.
        let mut flags = state_clone.cancel_flags.write().await;
        flags.remove(&app_id_for_cleanup);
    });

    // Convert receiver into a Stream for SSE, merging with executor events if available.
    let collector_for_stream = Arc::clone(&agent_trace_collector);
    let stream = async_stream::stream! {
        // Fork: receive from both channels
        use tokio::sync::broadcast;

        if let Some(mut executor_rx) = executor_events_rx {
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
                            // Collect agent traces (new)
                            match &event {
                                ExecutorEvent::TaskStarted { task_id, agent } => {
                                    collector_for_stream.on_task_started(&task_id.to_string(), agent).await;
                                }
                                ExecutorEvent::AgentEvent { task_id, agent, event: agent_event } => {
                                    collector_for_stream.on_agent_event(&task_id.to_string(), agent, agent_event).await;
                                }
                                ExecutorEvent::TaskCompleted { task_id, result } => {
                                    collector_for_stream.on_task_completed(
                                        &task_id.to_string(),
                                        result.success,
                                        Some(result.output.clone()),
                                        None,
                                    ).await;
                                }
                                ExecutorEvent::TaskFailed { task_id, error } => {
                                    collector_for_stream.on_task_completed(
                                        &task_id.to_string(),
                                        false,
                                        None,
                                        Some(error.clone()),
                                    ).await;
                                }
                                _ => {}
                            }

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
                            // Convert ExecutorEvent to SSE Event
                            let sse_event = convert_executor_event_to_sse(event);
                            yield sse_event;
                        }
                        Err(broadcast::error::TryRecvError::Empty) => break, // No more events
                        Err(broadcast::error::TryRecvError::Closed) => return,
                        Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                    }
                }

                // Check if we should exit after draining executor events
                if rx_closed && pending_delegated_tasks.is_empty() {
                    break;
                }

                // Now wait on main channel with a timeout to periodically check executor events
                tokio::select! {
                    // Regular SSE events from agent execution
                    result = rx.recv(), if !rx_closed => {
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
            // No executor events - just use regular channel
            while let Some(event) = rx.recv().await {
                yield event;
            }
        }
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
) -> Result<AssistantRunResult, String> {
    let agent_id = find_agent_by_name(state, app_id, agent_name).await;
    run_agentic_stream_with_agent_for_step(
        state, app_dir, agent_name, prompt, model, tool_defs,
        agent_id, tx, cancel, session_id, app_id.clone()
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
    let full_system = if catalog_prompt.is_empty() {
        system_prompt
    } else {
        format!("{system_prompt}\n\n{catalog_prompt}")
    };

    let messages = vec![
        LlmMessage::system(full_system),
        LlmMessage::user(prompt),
    ];

    run_agentic_stream(state, agent_id.clone(), messages, model, tool_defs, tx, cancel, session_id, app_id).await
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
) -> Result<AssistantRunResult, String> {
    use macaca_runtime::agentic_loop::ResumeReason;
    use macaca_proto::ForkId;

    let options = LlmOptions {
        model: model.to_string(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        tools: Some(tool_defs),
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
    // Collect delegated agent traces during execution
    // Key: agent name, Value: array of traces for that agent
    let mut agent_traces: std::collections::HashMap<String, Vec<AgentTrace>> = std::collections::HashMap::new();

    // Create pause/resume mechanism for Fork-Join workflow
    let (resume_tx, mut resume_rx) = tokio::sync::mpsc::channel::<ResumeReason>(1);
    let pause_signal = Arc::new(AtomicBool::new(false));

    // Register active session for hook consumer to resume
    {
        let session_handle = crate::state::ActiveSession {
            session_id: session_id.clone(),
            app_id: app_id.clone(),
            pause_signal: pause_signal.clone(),
            resume_tx: resume_tx.clone(),
        };
        state.active_sessions.write().await.insert(session_id.clone(), session_handle);
    }

    for i in 1..=max_iterations {
        iterations = i;

        // ===== PAUSE/RESUME CHECK FOR FORK-JOIN WORKFLOW =====
        while pause_signal.load(Ordering::SeqCst) {
            // Send paused event to SSE
            let _ = tx.send(Ok(Event::default()
                .event("loop_paused")
                .data(serde_json::json!({
                    "session_id": session_id,
                    "iteration": i,
                    "reason": "waiting_for_delegate"
                }).to_string())))
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
                        let _ = tx.send(Ok(Event::default()
                            .event("loop_resumed")
                            .data(serde_json::json!({
                                "session_id": session_id,
                                "task_id": task_id,
                                "success": success
                            }).to_string())))
                            .await;
                    }
                    ResumeReason::DelegateFailed { task_id, error } => {
                        let result_msg = format!(
                            "[Delegate Task {} Failed]\nError: {}",
                            task_id, error
                        );
                        messages.push(LlmMessage::user(result_msg));

                        let _ = tx.send(Ok(Event::default()
                            .event("loop_resumed")
                            .data(serde_json::json!({
                                "session_id": session_id,
                                "task_id": task_id,
                                "success": false,
                                "error": error
                            }).to_string())))
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
        let _ = tx
            .send(Ok(Event::default()
                .event("thinking")
                .data(serde_json::json!({"iteration": i}).to_string())))
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
                    let _ = tx
                        .send(Ok(Event::default().event("tool_result").data(
                            serde_json::json!({
                                "tool_name": tc.name,
                                "output": block_msg,
                            })
                            .to_string(),
                        )))
                        .await;
                    messages.push(LlmMessage::tool_result(tc.id.clone(), block_msg.to_string()));
                    continue;
                }

                // Stream tool_call event.
                let _ = tx
                    .send(Ok(Event::default().event("tool_call").data(
                        serde_json::json!({
                            "tool_name": tc.name,
                            "tool_input": tc.arguments,
                        })
                        .to_string(),
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
                // For claude_code_execute, use streaming to send real-time cc_trace events.
                let result = if tc.name == "claude_code_execute" {
                    // Use streaming version for real-time trace events.
                    if let Some(tool) = state.tools.get_tool(&tc.name) {
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
                } else if let Some(tool) = state.tools.get_tool(&tc.name) {
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
                let _ = tx
                    .send(Ok(Event::default().event("tool_result").data(
                        serde_json::json!({
                            "tool_name": tc.name,
                            "output": display_result,
                        })
                        .to_string(),
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
                                let _ = tx.send(Ok(Event::default()
                                    .event("fork_created")
                                    .data(serde_json::json!({
                                        "fork_id": fork_id_str,
                                        "session_id": session_id,
                                    }).to_string())))
                                    .await;

                                // Set pause signal - loop will pause at next iteration
                                pause_signal.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                }
                // ===== END DETECT delegate_task =====

                // Track claude_code_execute failures.
                if tc.name == "claude_code_execute" {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                        if let Some(trace) = parsed.get("trace") {
                            if let Ok(steps) = serde_json::from_value::<Vec<TraceEvent>>(trace.clone()) {
                                cc_trace_steps.extend(steps);
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
        messages.push(LlmMessage::assistant(response.content));

        // Stream final content event.
        let _ = tx
            .send(Ok(Event::default()
                .event("content")
                .data(serde_json::json!({"content": final_content}).to_string())))
            .await;

        break;
    }

    // Handle max iterations reached.
    if iterations >= max_iterations && final_content.is_empty() {
        final_content =
            "Max iterations reached. The agent may not have completed the task.".into();
        let _ = tx
            .send(Ok(Event::default()
                .event("content")
                .data(serde_json::json!({"content": final_content}).to_string())))
            .await;
    }

    // Clean up active session
    state.active_sessions.write().await.remove(&session_id);
    tracing::info!(session_id = %session_id, "Active session cleaned up");

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
        agent_traces,
    })
}
