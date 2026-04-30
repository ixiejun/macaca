//! Session management: types, CRUD handlers, and persistence.

use std::convert::Infallible;
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
use macaca_proto::{AgentExecutionEventVisitor, ApplicationId, LlmMessage};

use crate::routes::{err, ErrorResponse};
use crate::sse::{convert_executor_event_to_sse, load_plan_decisions, PlanDecisionEvent};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /api/sessions/:app_id — Get session history
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ExecutionTokens {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AssistantExecutionMeta {
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
pub(crate) struct StoredTraceStep {
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
pub(crate) struct AgentTraceStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
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
    pub tool_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Trace for a single delegated agent execution.
/// task_id uniquely identifies this specific task execution.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AgentTrace {
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

fn driver_trace_step(driver_name: Option<String>, trace: &serde_json::Value) -> AgentTraceStep {
    AgentTraceStep {
        step_type: "driver_trace".to_string(),
        event_type: trace
            .get("type")
            .or_else(|| trace.get("event_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        content: trace
            .get("content")
            .or_else(|| trace.get("thinking"))
            .or_else(|| trace.get("text"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_name: trace
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_input: trace.get("tool_input").cloned(),
        tool_output: trace
            .get("tool_output")
            .or_else(|| trace.get("tool_result"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        output: trace
            .get("output")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        is_error: trace.get("is_error").and_then(|v| v.as_bool()),
        driver_id: trace
            .get("driver_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        driver_name: driver_name.or_else(|| {
            trace
                .get("driver_name")
                .or_else(|| trace.get("driver_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }),
        title: trace
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        timestamp: trace.get("timestamp").and_then(|v| v.as_i64()),
        correlation_id: trace
            .get("correlation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        metadata: trace.get("metadata").cloned(),
        ..Default::default()
    }
}

fn delegated_driver_trace_step(payload: &serde_json::Value) -> AgentTraceStep {
    let event = payload
        .get("event")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if event.get("trace").is_some() {
        let driver_name = payload
            .get("driver_name")
            .or_else(|| event.get("driver_name"))
            .or_else(|| event.get("driver_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return driver_trace_step(
            driver_name,
            event.get("trace").unwrap_or(&serde_json::Value::Null),
        );
    }
    let driver_name = payload
        .get("driver_name")
        .or_else(|| event.get("driver_name"))
        .or_else(|| event.get("driver_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    driver_trace_step(driver_name, &event)
}

struct AgentTraceStepVisitor;

impl AgentExecutionEventVisitor<AgentTraceStep> for AgentTraceStepVisitor {
    fn thinking(&mut self, iteration: usize, content: Option<&str>) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "thinking".to_string(),
            iteration: Some(iteration),
            content: content.map(ToString::to_string),
            ..Default::default()
        }
    }

    fn tool_call(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        call_id: Option<&str>,
    ) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "tool_call".to_string(),
            tool_name: Some(tool_name.to_string()),
            tool_input: Some(tool_input.clone()),
            call_id: call_id.map(ToString::to_string),
            ..Default::default()
        }
    }

    fn tool_result(
        &mut self,
        tool_name: &str,
        output: &str,
        is_error: Option<bool>,
    ) -> AgentTraceStep {
        crate::metrics::record_tool_execution(tool_name, !is_error.unwrap_or(false));
        AgentTraceStep {
            step_type: "tool_result".to_string(),
            tool_name: Some(tool_name.to_string()),
            output: Some(output.to_string()),
            is_error,
            ..Default::default()
        }
    }

    fn assistant(&mut self, content: &str) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "assistant".to_string(),
            content: Some(content.to_string()),
            ..Default::default()
        }
    }

    fn driver_trace(&mut self, driver_name: &str, trace: &serde_json::Value) -> AgentTraceStep {
        driver_trace_step(Some(driver_name.to_string()), trace)
    }

    fn completed(&mut self, success: bool, error: Option<&str>) -> AgentTraceStep {
        AgentTraceStep {
            step_type: "completed".to_string(),
            success: Some(success),
            error: error.map(ToString::to_string),
            ..Default::default()
        }
    }
}

fn trace_step_from_agent_event(event: &macaca_proto::AgentExecutionEvent) -> AgentTraceStep {
    let mut visitor = AgentTraceStepVisitor;
    event.accept(&mut visitor)
}

// ---------------------------------------------------------------------------
// Agent Trace Collector for SSE Stream
// ---------------------------------------------------------------------------

/// Collects agent traces during SSE stream processing.
/// Shared between SSE stream and session saving.
pub(crate) struct AgentTraceCollector {
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
    pub async fn on_agent_event(
        &self,
        task_id: &str,
        agent: &str,
        event: &macaca_proto::AgentExecutionEvent,
    ) {
        tracing::debug!(task_id = %task_id, agent = %agent, event_type = ?std::mem::discriminant(event), "AgentTraceCollector: on_agent_event called");
        let mut traces = self.traces.write().await;
        if let Some(agent_traces) = traces.get_mut(agent) {
            if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                tracing::debug!(task_id = %task_id, agent = %agent, "AgentTraceCollector: found trace, adding step");
                let step = trace_step_from_agent_event(event);
                trace.steps.push(step);
            }
        }
    }

    /// Called when executor emits TaskCompleted/TaskFailed - update trace status
    /// Note: TaskCompleted/TaskFailed don't have agent field, so we look it up from task_to_agent mapping
    pub async fn on_task_completed(
        &self,
        task_id: &str,
        success: bool,
        output: Option<String>,
        error: Option<String>,
    ) {
        let agent = {
            let task_to_agent = self.task_to_agent.read().await;
            task_to_agent.get(task_id).cloned()
        };

        if let Some(agent) = agent {
            let mut traces = self.traces.write().await;
            if let Some(agent_traces) = traces.get_mut(&agent) {
                if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                    trace.status = if success {
                        "completed".to_string()
                    } else {
                        "error".to_string()
                    };
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
pub(crate) struct StoredTurn {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace_steps: Vec<StoredTraceStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<AssistantExecutionMeta>,
    /// Delegated agent traces keyed by agent name.
    /// This is a generic structure that works for any application with any number of agents.
    /// Key: agent name (e.g., "backend", "frontend", "tester" - dynamic, not hardcoded)
    /// Value: array of traces for that agent (supports multiple task executions per agent)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub agent_traces: std::collections::HashMap<String, Vec<AgentTrace>>,
}

pub(crate) fn ensure_running_assistant_turn(turns: &mut Vec<StoredTurn>) -> &mut StoredTurn {
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
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            });
            turns.len() - 1
        }
    };

    &mut turns[idx]
}

pub(crate) fn session_status_from_executor_event(event: &ExecutorEvent) -> Option<&'static str> {
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

pub(crate) async fn persist_session_snapshot(
    store: &Arc<RedbStore>,
    session_id: &str,
    app_id: &ApplicationId,
    status: Option<&str>,
    content: Option<String>,
    trace_steps: Option<Vec<StoredTraceStep>>,
    agent_traces: Option<std::collections::HashMap<String, Vec<AgentTrace>>>,
    meta: Option<AssistantExecutionMeta>,
) {
    // Acquire per-session write lock to prevent concurrent read-modify-write
    // from overwriting each other's data (e.g., periodic saver vs snapshot closure).
    // We use a simple approach: lock the session_key_db string as a file-level mutex.
    static SESSION_LOCKS: std::sync::OnceLock<
        tokio::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    > = std::sync::OnceLock::new();
    let locks =
        SESSION_LOCKS.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let lock = {
        let mut map = locks.lock().await;
        map.entry(session_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
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
pub(crate) struct SessionResponse {
    pub app_id: String,
    pub messages: Vec<SessionMessage>,
}

pub(crate) async fn get_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Json<SessionResponse> {
    let sessions = state.sessions.conversations.read().await;
    let messages = sessions
        .get(&app_id)
        .map(|hist| {
            hist.iter()
                .filter_map(|msg| {
                    // Only include user and assistant messages (not system/tool)
                    match msg.role {
                        macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => {
                            Some(SessionMessage {
                                role: format!("{:?}", msg.role).to_lowercase(),
                                content: msg.content.clone(),
                            })
                        }
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Json(SessionResponse { app_id, messages })
}

pub(crate) fn build_turns_from_messages(messages: &[LlmMessage]) -> Vec<StoredTurn> {
    messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User => Some(StoredTurn {
                role: "user".into(),
                content: msg.content.clone(),
                status: None,
                trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            macaca_proto::LlmRole::Assistant => Some(StoredTurn {
                role: "assistant".into(),
                content: msg.content.clone(),
                status: Some("completed".into()),
                trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn stored_turns_or_messages(stored: &StoredSession) -> Vec<StoredTurn> {
    if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    }
}

// ---------------------------------------------------------------------------
// Persistent Session Storage Types
// ---------------------------------------------------------------------------

/// Session metadata for listing and indexing.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SessionMeta {
    pub session_id: String,
    pub app_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub title: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
}

pub(crate) fn default_status() -> String {
    "draft".to_string()
}

/// Full session with messages for storage.
#[derive(Serialize, Deserialize)]
pub(crate) struct StoredSession {
    pub meta: SessionMeta,
    pub messages: Vec<LlmMessage>,
    #[serde(default)]
    pub turns: Vec<StoredTurn>,
}

// Key prefixes for redb storage
pub(crate) const SESSION_PREFIX: &str = "session/";
pub(crate) const APP_SESSIONS_PREFIX: &str = "app_sessions/";
/// Separate key prefix for agent traces — stored independently from session
/// to avoid read-modify-write races with session updates.
pub(crate) const AGENT_TRACES_PREFIX: &str = "agent_traces/";

/// Save agent traces to a dedicated key (simple overwrite, no read-modify-write).
pub(crate) async fn save_agent_traces(
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
pub(crate) async fn load_agent_traces(
    store: &Arc<RedbStore>,
    session_id: &str,
) -> std::collections::HashMap<String, Vec<AgentTrace>> {
    let key = format!("{}{}", AGENT_TRACES_PREFIX, session_id);
    match store.get(&key).await {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => std::collections::HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Real-time Session Update Helper
// ---------------------------------------------------------------------------

/// Update session status in real-time based on ExecutorEvent.
/// This is called during SSE streaming to keep session status up-to-date.
pub(crate) async fn update_session_realtime(
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
        None, // agent_traces: None — let periodic saver handle it
        None,
    )
    .await;
}

// ---------------------------------------------------------------------------
// GET /api/sessions — List all persistent sessions
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct SessionListItem {
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
pub(crate) struct SessionListQuery {
    #[serde(default)]
    status: Option<String>,
}

pub(crate) async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    let keys = state
        .persist
        .session_store
        .list_keys(SESSION_PREFIX)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list sessions: {e}"),
            )
        })?;

    let mut sessions = Vec::new();
    for key in keys {
        if let Some(data) = state.persist.session_store.get(&key).await.map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })? {
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

pub(crate) async fn list_app_sessions(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, Json<ErrorResponse>)> {
    // Collect session IDs from per-session index keys
    // Format: app_sessions/{app_id}/{session_id}
    let prefix = format!("{}{}/", APP_SESSIONS_PREFIX, app_id);
    let keys = state
        .persist
        .session_store
        .list_keys(&prefix)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list app sessions: {e}"),
            )
        })?;

    let mut session_ids: Vec<String> = keys
        .iter()
        .filter_map(|key| key.strip_prefix(&prefix).map(|s| s.to_string()))
        .collect();

    // Also check legacy aggregate key: app_sessions/{app_id} → Vec<String>
    // Older versions of post_chat_v2 wrote a JSON array instead of per-session keys.
    let legacy_key = format!("{}{}", APP_SESSIONS_PREFIX, app_id);
    if let Ok(Some(data)) = state.persist.session_store.get(&legacy_key).await {
        if let Ok(ids) = serde_json::from_slice::<Vec<String>>(&data) {
            for id in ids {
                if !session_ids.contains(&id) {
                    session_ids.push(id);
                }
            }
            // Migrate: write per-session index keys and delete the legacy aggregate key
            for id in &session_ids {
                let per_session_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id, id);
                let _ = state
                    .persist
                    .session_store
                    .set(&per_session_key, id.as_bytes())
                    .await;
            }
            let _ = state.persist.session_store.delete(&legacy_key).await;
        }
    }

    let mut sessions = Vec::new();
    for session_id in session_ids {
        let session_key = format!("{}{}", SESSION_PREFIX, session_id);
        if let Some(data) = state
            .persist
            .session_store
            .get(&session_key)
            .await
            .map_err(|e| {
                err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get session: {e}"),
                )
            })?
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
// GET /api/sessions/:session_id — Get a specific persistent session
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct SessionDetail {
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

pub(crate) async fn get_session_by_id(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<SessionDetail>, (StatusCode, Json<ErrorResponse>)> {
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state
        .persist
        .session_store
        .get(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data).map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse session: {e}"),
        )
    })?;

    // Also populate in-memory cache so subsequent messages continue the conversation
    {
        let mut sessions = state.sessions.conversations.write().await;
        sessions.insert(session_id.clone(), stored.messages.clone());
    }

    let messages = stored
        .messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User | macaca_proto::LlmRole::Assistant => {
                Some(SessionMessage {
                    role: format!("{:?}", msg.role).to_lowercase(),
                    content: msg.content.clone(),
                })
            }
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
        let events = state.persist.event_log.query(&session_id, 0, 10000).await;
        let mut agent_traces: std::collections::HashMap<String, Vec<AgentTrace>> =
            std::collections::HashMap::new();
        let mut task_to_agent: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for event in &events {
            let payload = &event.payload;
            let agent = payload
                .get("agent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let task_id = payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            match event.event_type.as_str() {
                "delegated_task_start" => {
                    if agent.is_empty() || task_id.is_empty() {
                        continue;
                    }
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
                "delegated_thinking"
                | "delegated_tool_call"
                | "delegated_tool_result"
                | "delegated_assistant"
                | "delegated_driver_trace"
                | "delegated_done" => {
                    let resolved_agent = if !agent.is_empty() {
                        agent.clone()
                    } else {
                        task_to_agent.get(&task_id).cloned().unwrap_or_default()
                    };
                    if resolved_agent.is_empty() || task_id.is_empty() {
                        continue;
                    }
                    if let Some(traces) = agent_traces.get_mut(&resolved_agent) {
                        if let Some(trace) = traces.iter_mut().rfind(|t| t.task_id == task_id) {
                            let step_type = event
                                .event_type
                                .strip_prefix("delegated_")
                                .unwrap_or(&event.event_type);
                            let evt = payload
                                .get("event")
                                .cloned()
                                .unwrap_or(serde_json::json!({}));
                            let step = if event.event_type == "delegated_driver_trace" {
                                delegated_driver_trace_step(payload)
                            } else {
                                AgentTraceStep {
                                    step_type: step_type.to_string(),
                                    iteration: evt
                                        .get("iteration")
                                        .and_then(|v| v.as_u64())
                                        .map(|v| v as usize),
                                    content: evt
                                        .get("content")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    tool_name: evt
                                        .get("tool_name")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    tool_input: evt.get("tool_input").cloned(),
                                    output: evt
                                        .get("output")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    is_error: evt.get("is_error").and_then(|v| v.as_bool()),
                                    call_id: evt
                                        .get("call_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    success: evt.get("success").and_then(|v| v.as_bool()),
                                    error: evt
                                        .get("error")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    ..Default::default()
                                }
                            };
                            trace.steps.push(step);
                        }
                    }
                }
                "delegated_task_complete" => {
                    let resolved_agent = if !agent.is_empty() {
                        agent.clone()
                    } else {
                        task_to_agent.get(&task_id).cloned().unwrap_or_default()
                    };
                    if resolved_agent.is_empty() {
                        continue;
                    }
                    if let Some(traces) = agent_traces.get_mut(&resolved_agent) {
                        if let Some(trace) = traces.iter_mut().rfind(|t| t.task_id == task_id) {
                            trace.status = "completed".to_string();
                            trace.output = payload
                                .get("output")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
                "delegated_task_error" => {
                    let resolved_agent = if !agent.is_empty() {
                        agent.clone()
                    } else {
                        task_to_agent.get(&task_id).cloned().unwrap_or_default()
                    };
                    if resolved_agent.is_empty() {
                        continue;
                    }
                    if let Some(traces) = agent_traces.get_mut(&resolved_agent) {
                        if let Some(trace) = traces.iter_mut().rfind(|t| t.task_id == task_id) {
                            trace.status = "error".to_string();
                            trace.error = payload
                                .get("error")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        // Rebuild coordinator trace_steps from EventLog (source of truth).
        // Coordinator events: thinking, tool_call, tool_result, content
        let mut coordinator_traces: Vec<StoredTraceStep> = Vec::new();
        let mut latest_coordinator_content: Option<String> = None;
        let mut coordinator_done = false;
        for event in &events {
            if event.source != "coordinator" {
                continue;
            }
            let payload = &event.payload;
            match event.event_type.as_str() {
                "thinking" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "thinking".into(),
                        iteration: payload
                            .get("iteration")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize),
                        content: payload
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        tool_name: None,
                        tool_input: None,
                        output: None,
                    });
                }
                "tool_call" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "tool_call".into(),
                        iteration: None,
                        tool_name: payload
                            .get("tool_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        tool_input: payload.get("tool_input").cloned(),
                        content: None,
                        output: None,
                    });
                }
                "tool_result" => {
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "tool_result".into(),
                        iteration: None,
                        tool_name: payload
                            .get("tool_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        output: payload
                            .get("output")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        content: None,
                        tool_input: None,
                    });
                }
                "content" => {
                    if let Some(content) = payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                    {
                        latest_coordinator_content = Some(content.clone());
                        coordinator_traces.push(StoredTraceStep {
                            step_type: "assistant".into(),
                            iteration: None,
                            tool_name: None,
                            tool_input: None,
                            output: None,
                            content: Some(content),
                        });
                    }
                }
                "done" => {
                    coordinator_done = true;
                    coordinator_traces.push(StoredTraceStep {
                        step_type: "done".into(),
                        iteration: None,
                        tool_name: None,
                        tool_input: None,
                        output: Some(payload.to_string()),
                        content: None,
                    });
                }
                _ => {}
            }
        }

        // Running sessions can have persisted EventLog traces before the final
        // assistant turn is saved. Create a placeholder assistant turn so a
        // browser refresh can restore both coordinator and delegated history.
        if (!agent_traces.is_empty()
            || !coordinator_traces.is_empty()
            || latest_coordinator_content.is_some())
            && !turns.iter().any(|t| t.role == "assistant")
        {
            let turn = ensure_running_assistant_turn(&mut turns);
            turn.status = Some(stored.meta.status.clone());
        }

        if let Some(assistant_turn) = turns.iter_mut().rev().find(|t| t.role == "assistant") {
            if !agent_traces.is_empty() {
                assistant_turn.agent_traces = agent_traces;
            }
            if !coordinator_traces.is_empty() {
                assistant_turn.trace_steps = coordinator_traces;
            }
            if let Some(content) = latest_coordinator_content {
                assistant_turn.content = content;
            }
            if coordinator_done {
                assistant_turn.status = Some("completed".to_string());
            }
        }
    }

    // Load plan decision events from independent storage.
    let plan_decisions = if let Ok(app_uuid) = uuid::Uuid::parse_str(&stored.meta.app_id) {
        load_plan_decisions(&state.persist.session_store, &ApplicationId(app_uuid)).await
    } else {
        Vec::new()
    };

    // EventLog metadata for frontend migration.
    let events_count = state.persist.event_log.count(&session_id).await;
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

pub(crate) async fn delete_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // First get the session to find the app_id
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state
        .persist
        .session_store
        .get(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data).map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse session: {e}"),
        )
    })?;

    // Delete the session data
    state
        .persist
        .session_store
        .delete(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete session: {e}"),
            )
        })?;

    // Delete the app session index entry
    let app_index_key = format!(
        "{}{}/{}",
        APP_SESSIONS_PREFIX, stored.meta.app_id, session_id
    );
    state
        .persist
        .session_store
        .delete(&app_index_key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete session index: {e}"),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// GET /api/sessions/:session_id/events — Stream session events via SSE
// ---------------------------------------------------------------------------

pub(crate) async fn stream_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    let key = format!("{}{}", SESSION_PREFIX, session_id);
    let data = state
        .persist
        .session_store
        .get(&key)
        .await
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {e}"),
            )
        })?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Session not found".into()))?;

    let stored: StoredSession = serde_json::from_slice(&data).map_err(|e| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse session: {e}"),
        )
    })?;

    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&stored.meta.app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let maybe_executor = state.executor_registry.get(&app_id).await;

    // Check if coordinator is actively running (has an active_session entry)
    let active = state
        .sessions
        .active_sessions
        .read()
        .await
        .contains_key(&session_id);

    // If coordinator is active, hot-swap its sse_tx so events flow to this new connection
    let mut coordinator_rx = if active {
        let (new_tx, new_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
        // Replace sse_tx → bridge now forwards coordinator events to new_rx
        let sessions = state.sessions.active_sessions.read().await;
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

        // Subscribe to EventLog broadcast so we can notify the frontend
        // when new events are appended (e.g. driver_trace from delegated agents).
        let mut event_log_rx = state.persist.event_log.subscribe();
        let stream_session_id = session_id.clone();

        let Some(executor) = maybe_executor else {
            yield Ok(Event::default().event("session_end").data("{}"));
            return;
        };

        if let Some(ref mut coord_rx) = coordinator_rx {
            // Active session mode:
            // The chat_v2 forwarders already send both coordinator and delegated
            // executor events into this hot-swapped coordinator channel.
            // Reading executor_rx here would duplicate every delegated event.
            loop {
                tokio::select! {
                    msg = coord_rx.recv() => {
                        match msg {
                            Some(event) => yield event,
                            None => break,
                        }
                    }
                    result = event_log_rx.recv() => {
                        match result {
                            Ok((notified_sid, latest_seq)) => {
                                if notified_sid == stream_session_id {
                                    yield Ok(Event::default()
                                        .event("update")
                                        .data(serde_json::json!({
                                            "seq": latest_seq
                                        }).to_string()));
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
        } else {
            // No active coordinator — just stream executor events
            let mut executor_rx = executor.subscribe_to_events();
            loop {
                tokio::select! {
                    result = executor_rx.recv() => {
                        match result {
                            Ok(event) => yield convert_executor_event_to_sse(event),
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                    result = event_log_rx.recv() => {
                        match result {
                            Ok((notified_sid, latest_seq)) => {
                                if notified_sid == stream_session_id {
                                    yield Ok(Event::default()
                                        .event("update")
                                        .data(serde_json::json!({
                                            "seq": latest_seq
                                        }).to_string()));
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
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
            meta: None,
            agent_traces: std::collections::HashMap::new(),
        }
    }

    fn make_turn_with_traces(
        role: &str,
        content: &str,
        status: Option<&str>,
        agents: Vec<&str>,
    ) -> StoredTurn {
        let mut agent_traces = std::collections::HashMap::new();
        for agent in agents {
            agent_traces.insert(
                agent.to_string(),
                vec![AgentTrace {
                    task_id: format!("task-{agent}"),
                    agent: agent.to_string(),
                    status: "completed".to_string(),
                    steps: vec![],
                    output: Some("done".to_string()),
                    error: None,
                }],
            );
        }
        StoredTurn {
            role: role.into(),
            content: content.into(),
            status: status.map(String::from),
            trace_steps: Vec::new(),
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
        assert!(
            turns.is_empty(),
            "snapshot running turn and its user turn should be removed"
        );

        // Now push final turns
        turns.push(make_turn("user", "hello", None));
        turns.push(make_turn_with_traces(
            "assistant",
            "final answer",
            Some("completed"),
            vec!["backend"],
        ));

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
            make_turn_with_traces(
                "assistant",
                "first answer",
                Some("completed"),
                vec!["backend"],
            ),
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
        let turn = make_turn_with_traces(
            "assistant",
            "answer",
            Some("completed"),
            vec!["backend", "tester"],
        );
        let json = serde_json::to_string(&turn).unwrap();
        let deserialized: StoredTurn = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_traces.len(), 2);
        assert!(deserialized.agent_traces.contains_key("backend"));
        assert!(deserialized.agent_traces.contains_key("tester"));
        assert_eq!(
            deserialized.agent_traces["backend"][0].task_id,
            "task-backend"
        );
        assert_eq!(deserialized.agent_traces["tester"][0].status, "completed");
    }

    #[test]
    fn test_agent_trace_empty_skipped_in_json() {
        let turn = make_turn("assistant", "answer", Some("completed"));
        let json = serde_json::to_string(&turn).unwrap();

        // agent_traces with skip_serializing_if should not appear in JSON when empty
        assert!(
            !json.contains("agent_traces"),
            "empty agent_traces should be skipped in JSON"
        );
    }

    #[test]
    fn test_delegated_driver_trace_step_handles_direct_trace_payload() {
        let payload = serde_json::json!({
            "driver_name": "opencode",
            "event": {
                "type": "bash",
                "driver_id": "opencode",
                "tool_name": "bash",
                "tool_input": { "cmd": "ls -la" },
                "tool_output": "ok",
                "title": "Bash"
            }
        });

        let step = delegated_driver_trace_step(&payload);

        assert_eq!(step.step_type, "driver_trace");
        assert_eq!(step.event_type.as_deref(), Some("bash"));
        assert_eq!(step.driver_name.as_deref(), Some("opencode"));
        assert_eq!(step.driver_id.as_deref(), Some("opencode"));
        assert_eq!(step.tool_name.as_deref(), Some("bash"));
        assert_eq!(step.tool_output.as_deref(), Some("ok"));
        assert_eq!(step.title.as_deref(), Some("Bash"));
    }

    #[test]
    fn test_delegated_driver_trace_step_unwraps_nested_driver_trace_payload() {
        let payload = serde_json::json!({
            "event": {
                "type": "driver_trace",
                "driver_name": "claude-code",
                "trace": {
                    "type": "thinking",
                    "driver_id": "claude-code",
                    "content": "planning next action"
                }
            }
        });

        let step = delegated_driver_trace_step(&payload);

        assert_eq!(step.step_type, "driver_trace");
        assert_eq!(step.event_type.as_deref(), Some("thinking"));
        assert_eq!(step.driver_name.as_deref(), Some("claude-code"));
        assert_eq!(step.driver_id.as_deref(), Some("claude-code"));
        assert_eq!(step.content.as_deref(), Some("planning next action"));
    }

    #[test]
    fn test_ensure_running_assistant_turn_creates_new() {
        let mut turns = vec![make_turn("user", "hi", None)];
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
