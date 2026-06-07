//! Session DTOs and persistence schema types.
//!
//! These serde models are application-agnostic: agent names and trace payloads
//! are keyed by runtime identifiers, never hardcoded OS business names.

use chrono::{DateTime, Utc};
use macaca_proto::LlmMessage;
use serde::{Deserialize, Serialize};

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
#[derive(Serialize)]
pub(crate) struct SessionResponse {
    pub app_id: String,
    pub messages: Vec<SessionMessage>,
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
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) offset: Option<usize>,
}
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
    /// URL to fetch events from the EventLog for this session.
    pub events_url: String,
    /// Total number of events persisted in EventLog for this session.
    pub events_count: usize,
}
