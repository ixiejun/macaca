//! A2A (Agent-to-Agent) protocol support.
//!
//! Implements the A2A protocol types and converters:
//! - `AgentCard` for service discovery
//! - `A2AMessage`, `A2ATask`, `A2AArtifact` for agent communication
//! - `A2AFormatter` for bidirectional conversion between internal `Msg` and A2A types
//! - `AgentCardResolver` trait and `FileCardResolver` implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ToolResultBlock, ToolUseBlock,
};

// ---------------------------------------------------------------------------
// AgentCard — service discovery
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// A2AFormatter — bidirectional conversion
// ---------------------------------------------------------------------------

/// Converts between framework-internal `Msg` and A2A protocol types.
pub struct A2AFormatter;

impl A2AFormatter {
    /// Convert an internal `Msg` to an `A2AMessage`.
    ///
    /// `ThinkingBlock`s are stripped — internal reasoning must not cross
    /// agent boundaries. `Audio` and `Video` blocks are also omitted as A2A
    /// has no standard representation for them.
    pub fn to_a2a(msg: &Msg) -> A2AMessage {
        let role = match msg.role {
            Role::User | Role::System | Role::Tool => A2ARole::User,
            Role::Assistant => A2ARole::Agent,
        };

        let parts = match &msg.content {
            MsgContent::Text(t) => vec![A2APart::Text { text: t.clone() }],
            MsgContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text(t) => Some(A2APart::Text {
                        text: t.text.clone(),
                    }),
                    ContentBlock::Image(img) => Some(A2APart::File {
                        file: A2AFile {
                            uri: img.url.clone(),
                            bytes: img.data.clone(),
                            mime_type: img.mime_type.clone(),
                        },
                    }),
                    ContentBlock::ToolUse(tu) => Some(A2APart::Data {
                        data: serde_json::json!({
                            "kind": "tool_use",
                            "id": tu.id,
                            "name": tu.name,
                            "input": tu.input,
                        }),
                    }),
                    ContentBlock::ToolResult(tr) => Some(A2APart::Data {
                        data: serde_json::json!({
                            "kind": "tool_result",
                            "tool_use_id": tr.tool_use_id,
                            "output": tr.output,
                            "is_error": tr.is_error,
                        }),
                    }),
                    ContentBlock::Data(data) => Some(A2APart::Data {
                        data: serde_json::json!({
                            "kind": "data",
                            "id": data.id,
                            "name": data.name,
                            "data": data.data,
                        }),
                    }),
                    // Strip internal reasoning — must not cross agent boundaries.
                    ContentBlock::Thinking(_) => None,
                    // Hints are middleware guidance and must not leak to peer agents.
                    ContentBlock::Hint(_) => None,
                    // Audio/Video have no A2A representation yet.
                    ContentBlock::Audio(_) | ContentBlock::Video(_) => None,
                })
                .collect(),
        };

        A2AMessage {
            message_id: msg.id.clone(),
            role,
            parts,
            context_id: None,
        }
    }

    /// Convert an `A2AMessage` to an internal `Msg`.
    ///
    /// `name` is the local name to assign to the sender (used in multi-agent
    /// pipelines to identify the remote participant).
    ///
    /// Returns an error when a `Data` part contains an unrecognised `kind`,
    /// or when required fields for `tool_use` / `tool_result` are missing.
    pub fn from_a2a(name: &str, a2a_msg: &A2AMessage) -> Result<Msg, A2AError> {
        let role = match a2a_msg.role {
            A2ARole::User => Role::User,
            A2ARole::Agent => Role::Assistant,
        };

        let mut blocks: Vec<ContentBlock> = Vec::new();

        for p in &a2a_msg.parts {
            match p {
                A2APart::Text { text } => {
                    blocks.push(ContentBlock::Text(TextBlock { text: text.clone() }));
                }
                A2APart::File { file } => {
                    blocks.push(ContentBlock::Image(ImageBlock {
                        url: file.uri.clone(),
                        data: file.bytes.clone(),
                        mime_type: file.mime_type.clone(),
                    }));
                }
                A2APart::Data { data } => {
                    let kind = data
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| A2AError::MissingField("kind".into()))?;

                    match kind {
                        "tool_use" => {
                            // Validate required fields
                            if data.get("name").and_then(|v| v.as_str()).is_none() {
                                return Err(A2AError::MissingField(
                                    "name (required for tool_use)".into(),
                                ));
                            }
                            if data.get("id").and_then(|v| v.as_str()).is_none() {
                                return Err(A2AError::MissingField(
                                    "id (required for tool_use)".into(),
                                ));
                            }
                            blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                                id: data["id"].as_str().unwrap_or("").to_string(),
                                name: data["name"].as_str().unwrap_or("").to_string(),
                                input: data["input"].clone(),
                                raw_input: None,
                            }));
                        }
                        "tool_result" => {
                            if data.get("tool_use_id").and_then(|v| v.as_str()).is_none() {
                                return Err(A2AError::MissingField(
                                    "tool_use_id (required for tool_result)".into(),
                                ));
                            }
                            blocks.push(ContentBlock::ToolResult(ToolResultBlock {
                                tool_use_id: data["tool_use_id"].as_str().unwrap_or("").to_string(),
                                output: data["output"].as_str().unwrap_or("").to_string(),
                                name: None,
                                is_error: data["is_error"].as_bool().unwrap_or(false),
                            }));
                        }
                        other => {
                            return Err(A2AError::InvalidDataType(other.to_string()));
                        }
                    }
                }
            }
        }

        // Prefer a simple Text content when there is exactly one text block.
        let content = match blocks.as_slice() {
            [ContentBlock::Text(t)] => MsgContent::Text(t.text.clone()),
            _ => MsgContent::Blocks(blocks),
        };

        Ok(Msg::new(name, content, role))
    }
}

// ---------------------------------------------------------------------------
// AgentCardResolver trait
// ---------------------------------------------------------------------------

/// Error type for A2A operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum A2AError {
    /// Failed to locate or fetch an agent card.
    #[error("Discovery error: {0}")]
    Discovery(String),
    /// Network or transport failure.
    #[error("Communication error: {0}")]
    Communication(String),
    /// Unexpected protocol violation.
    #[error("Protocol error: {0}")]
    Protocol(String),
    /// Data part has an unrecognised `type` / `kind` value.
    #[error("Invalid data type: {0}")]
    InvalidDataType(String),
    /// A required field is missing from a Data part.
    #[error("Missing required field: {0}")]
    MissingField(String),
    /// JSON deserialization failed.
    #[error("Deserialization error: {0}")]
    Deserialize(String),
}

/// Resolves an `AgentCard` from some source (file, HTTP, registry, …).
#[async_trait]
pub trait AgentCardResolver: Send + Sync {
    /// Fetch the agent card.
    async fn resolve(&self) -> Result<AgentCard, A2AError>;
}

/// Load an `AgentCard` from a local JSON file.
pub struct FileCardResolver {
    path: std::path::PathBuf,
}

impl FileCardResolver {
    /// Create a new resolver that reads from `path`.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl AgentCardResolver for FileCardResolver {
    async fn resolve(&self) -> Result<AgentCard, A2AError> {
        let data = tokio::fs::read_to_string(&self.path).await.map_err(|e| {
            A2AError::Discovery(format!(
                "Failed to read card file '{}': {e}",
                self.path.display()
            ))
        })?;
        serde_json::from_str(&data)
            .map_err(|e| A2AError::Discovery(format!("Failed to parse agent card: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "a2a_tests.rs"]
mod a2a_tests;
