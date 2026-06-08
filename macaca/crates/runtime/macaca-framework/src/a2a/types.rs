//! A2A wire protocol value objects (discovery, messages, tasks, RPC envelopes).
//!
//! These types mirror the vendor-neutral Agent-to-Agent JSON schema used for
//! inter-agent discovery and message exchange. They are serde-friendly DTOs with
//! no transport or application-specific logic.

use serde::{Deserialize, Serialize};

/// Describes an agent's capabilities, skills, and endpoint for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Human-readable agent name.
    pub name: String,
    /// Base URL of the agent's A2A endpoint.
    pub url: String,
    /// Semantic version string.
    pub version: String,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Feature flags this agent supports.
    pub capabilities: AgentCapabilities,
    /// Input MIME modes supported by default (e.g., `["text/plain"]`).
    #[serde(default)]
    pub default_input_modes: Vec<String>,
    /// Output MIME modes supported by default.
    #[serde(default)]
    pub default_output_modes: Vec<String>,
    /// Named skills this agent can perform.
    #[serde(default)]
    pub skills: Vec<AgentSkillInfo>,
}

/// Feature flags for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Whether the agent supports streaming responses.
    #[serde(default)]
    pub streaming: bool,
    /// Whether the agent supports push notifications.
    #[serde(default)]
    pub push_notifications: bool,
    /// Whether the agent tracks full state-transition history.
    #[serde(default)]
    pub state_transition_history: bool,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            streaming: true,
            push_notifications: false,
            state_transition_history: false,
        }
    }
}

/// A named skill exposed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillInfo {
    /// Skill identifier.
    pub name: String,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// A2A Message
// ---------------------------------------------------------------------------

/// A message exchanged between agents in the A2A protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Unique message identifier.
    pub message_id: String,
    /// Role of the sender.
    pub role: A2ARole,
    /// Content parts of this message.
    pub parts: Vec<A2APart>,
    /// Optional context / session ID grouping related messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

/// Role of the sender in an A2A exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2ARole {
    User,
    Agent,
}

/// A typed content part inside an `A2AMessage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2APart {
    /// Plain text.
    Text { text: String },
    /// A file attachment (URI or base64 bytes).
    File { file: A2AFile },
    /// Arbitrary structured data (tool calls, results, etc.).
    Data { data: serde_json::Value },
}

/// A file referenced by an `A2APart::File`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AFile {
    /// Remote URI to the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Base64-encoded file bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<String>,
    /// MIME type (e.g., `"image/png"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

// ---------------------------------------------------------------------------
// A2A Task
// ---------------------------------------------------------------------------

/// A long-running task managed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Unique task identifier.
    pub id: String,
    /// Optional context / session ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Current task status.
    pub status: A2ATaskStatus,
    /// Output artifacts produced by the task.
    #[serde(default)]
    pub artifacts: Vec<A2AArtifact>,
}

/// Status of an `A2ATask`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskStatus {
    /// Lifecycle state of the task.
    pub state: A2ATaskState,
    /// Optional message from the agent explaining the current state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<A2AMessage>,
}

/// Lifecycle state of an A2A task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum A2ATaskState {
    /// Task received but not yet started.
    Submitted,
    /// Task is actively being processed.
    Working,
    /// Task finished successfully.
    Completed,
    /// Task ended with an error.
    Failed,
    /// Task was canceled.
    Canceled,
}

/// An output artifact produced by an A2A task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AArtifact {
    /// Unique artifact identifier.
    pub artifact_id: String,
    /// Optional human-readable name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Content parts of this artifact.
    pub parts: Vec<A2APart>,
}

// ---------------------------------------------------------------------------
// Request / Response
// ---------------------------------------------------------------------------

/// Request to send a message to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// Client-generated request ID for deduplication.
    pub id: String,
    /// The message being sent.
    pub message: A2AMessage,
}

/// Response to a `SendMessageRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SendMessageResponse {
    /// The agent replied inline with a message.
    Message { message: A2AMessage },
    /// The agent created an asynchronous task.
    Task { task: A2ATask },
}
