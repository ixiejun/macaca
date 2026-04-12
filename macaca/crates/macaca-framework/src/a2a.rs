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
                    // Strip internal reasoning — must not cross agent boundaries.
                    ContentBlock::Thinking(_) => None,
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
mod tests {
    use super::*;
    use crate::message::{ContentBlock, MsgContent, TextBlock, ThinkingBlock, ToolUseBlock};

    // Helper: build a minimal AgentCard.
    fn sample_card() -> AgentCard {
        AgentCard {
            name: "test-agent".into(),
            url: "http://localhost:9000".into(),
            version: "1.0.0".into(),
            description: Some("A test agent".into()),
            capabilities: AgentCapabilities::default(),
            default_input_modes: vec!["text/plain".into()],
            default_output_modes: vec!["text/plain".into()],
            skills: vec![AgentSkillInfo {
                name: "echo".into(),
                description: Some("Echoes back the input".into()),
            }],
        }
    }

    // 1. AgentCard JSON round-trip
    #[test]
    fn test_agent_card_serialization() {
        let card = sample_card();
        let json = serde_json::to_string(&card).unwrap();
        let back: AgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, card.name);
        assert_eq!(back.url, card.url);
        assert_eq!(back.version, card.version);
        assert_eq!(back.description, card.description);
        assert_eq!(back.capabilities.streaming, true);
        assert_eq!(back.capabilities.push_notifications, false);
        assert_eq!(back.default_input_modes, card.default_input_modes);
        assert_eq!(back.skills.len(), 1);
        assert_eq!(back.skills[0].name, "echo");
    }

    // 2. A2AMessage JSON round-trip
    #[test]
    fn test_a2a_message_serialization() {
        let msg = A2AMessage {
            message_id: "msg-001".into(),
            role: A2ARole::User,
            parts: vec![
                A2APart::Text {
                    text: "Hello!".into(),
                },
                A2APart::Data {
                    data: serde_json::json!({"key": "value"}),
                },
            ],
            context_id: Some("ctx-1".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: A2AMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.message_id, "msg-001");
        assert_eq!(back.role, A2ARole::User);
        assert_eq!(back.parts.len(), 2);
        assert_eq!(back.context_id, Some("ctx-1".into()));
    }

    // 3. Task states serialization
    #[test]
    fn test_a2a_task_states() {
        let states = [
            A2ATaskState::Submitted,
            A2ATaskState::Working,
            A2ATaskState::Completed,
            A2ATaskState::Failed,
            A2ATaskState::Canceled,
        ];
        let expected_strs = ["submitted", "working", "completed", "failed", "canceled"];
        for (state, expected) in states.iter().zip(expected_strs.iter()) {
            let json = serde_json::to_string(state).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
            let back: A2ATaskState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *state);
        }

        // Full task round-trip
        let task = A2ATask {
            id: "task-1".into(),
            context_id: None,
            status: A2ATaskStatus {
                state: A2ATaskState::Working,
                message: None,
            },
            artifacts: vec![],
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: A2ATask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "task-1");
        assert_eq!(back.status.state, A2ATaskState::Working);
    }

    // 4. Formatter: text Msg → A2AMessage
    #[test]
    fn test_formatter_text_to_a2a() {
        let msg = Msg::user("alice", "Hello, agent!");
        let a2a = A2AFormatter::to_a2a(&msg);
        assert_eq!(a2a.message_id, msg.id);
        assert_eq!(a2a.role, A2ARole::User);
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "Hello, agent!"),
            other => panic!("Expected text part, got {:?}", other),
        }
    }

    // 5. Formatter: tool-use Msg → A2AMessage (Data part)
    #[test]
    fn test_formatter_tool_use_to_a2a() {
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Calling tool.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "call_42".into(),
                name: "search".into(),
                input: serde_json::json!({"query": "rust a2a"}),
                raw_input: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let a2a = A2AFormatter::to_a2a(&msg);
        assert_eq!(a2a.role, A2ARole::Agent);
        assert_eq!(a2a.parts.len(), 2);
        match &a2a.parts[1] {
            A2APart::Data { data } => {
                assert_eq!(data["kind"], "tool_use");
                assert_eq!(data["id"], "call_42");
                assert_eq!(data["name"], "search");
            }
            other => panic!("Expected Data part, got {:?}", other),
        }
    }

    // 6. Formatter: A2AMessage → Msg (reverse)
    #[test]
    fn test_formatter_a2a_to_msg() {
        let a2a = A2AMessage {
            message_id: "m-1".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Text {
                text: "I found it.".into(),
            }],
            context_id: None,
        };
        let msg = A2AFormatter::from_a2a("remote-agent", &a2a).unwrap();
        assert_eq!(msg.name, "remote-agent");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.get_text(), "I found it.");
    }

    // 7. Formatter: ThinkingBlock is stripped
    #[test]
    fn test_formatter_strips_thinking() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "internal thought".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "Visible answer.".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let a2a = A2AFormatter::to_a2a(&msg);
        // Only the text part survives; thinking is stripped.
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "Visible answer."),
            other => panic!("Expected text part, got {:?}", other),
        }
    }

    // 8. FileCardResolver reads a JSON file
    #[tokio::test]
    async fn test_file_card_resolver() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let card_json = serde_json::to_string(&sample_card()).unwrap();
        tmp.write_all(card_json.as_bytes()).unwrap();

        let resolver = FileCardResolver::new(tmp.path());
        let card = resolver.resolve().await.unwrap();
        assert_eq!(card.name, "test-agent");
        assert_eq!(card.url, "http://localhost:9000");
        assert_eq!(card.skills.len(), 1);
    }

    // 9. SendMessageRequest serialization
    #[test]
    fn test_send_message_request() {
        let req = SendMessageRequest {
            id: "req-001".into(),
            message: A2AMessage {
                message_id: "msg-002".into(),
                role: A2ARole::User,
                parts: vec![A2APart::Text {
                    text: "Do something".into(),
                }],
                context_id: None,
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: SendMessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "req-001");
        assert_eq!(back.message.message_id, "msg-002");
        assert_eq!(back.message.role, A2ARole::User);
        match &back.message.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "Do something"),
            other => panic!("Expected text, got {:?}", other),
        }
    }

    // 10. to_a2a → from_a2a roundtrip for plain text
    #[test]
    fn test_to_a2a_from_a2a_roundtrip_text() {
        let msg = Msg::user("alice", "Hello, round trip!");
        let a2a = A2AFormatter::to_a2a(&msg);
        let back = A2AFormatter::from_a2a("alice", &a2a).unwrap();

        assert_eq!(back.get_text(), "Hello, round trip!");
        assert_eq!(back.role, Role::User);
    }

    // 11. ThinkingBlock is stripped in to_a2a (explicit roundtrip check)
    #[test]
    fn test_to_a2a_strips_thinking() {
        let blocks = vec![
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "secret reasoning".into(),
            }),
            ContentBlock::Text(TextBlock {
                text: "public answer".into(),
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let a2a = A2AFormatter::to_a2a(&msg);

        // Only one part (text), thinking stripped.
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "public answer"),
            other => panic!("Expected text, got {:?}", other),
        }
    }

    // 12. ToolUse roundtrip through A2A
    #[test]
    fn test_to_a2a_from_a2a_tool_use() {
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Invoking tool.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "call_99".into(),
                name: "code_search".into(),
                input: serde_json::json!({"pattern": "fn main"}),
                raw_input: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
        let a2a = A2AFormatter::to_a2a(&msg);
        let back = A2AFormatter::from_a2a("bot", &a2a).unwrap();

        assert_eq!(back.role, Role::Assistant);
        let tool_calls = back.get_tool_calls();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_99");
        assert_eq!(tool_calls[0].name, "code_search");
        assert_eq!(
            tool_calls[0].input,
            serde_json::json!({"pattern": "fn main"})
        );
    }

    // 13. AgentCard full serde roundtrip with all fields
    #[test]
    fn test_agent_card_serde() {
        let card = AgentCard {
            name: "multi-skill".into(),
            url: "https://example.com/a2a".into(),
            version: "2.1.0".into(),
            description: Some("An agent with many skills".into()),
            capabilities: AgentCapabilities {
                streaming: true,
                push_notifications: true,
                state_transition_history: true,
            },
            default_input_modes: vec!["text/plain".into(), "application/json".into()],
            default_output_modes: vec!["text/plain".into()],
            skills: vec![
                AgentSkillInfo {
                    name: "search".into(),
                    description: Some("Search the web".into()),
                },
                AgentSkillInfo {
                    name: "compute".into(),
                    description: None,
                },
            ],
        };
        let json = serde_json::to_string(&card).unwrap();
        let back: AgentCard = serde_json::from_str(&json).unwrap();

        assert_eq!(back.name, "multi-skill");
        assert_eq!(back.version, "2.1.0");
        assert_eq!(back.description, Some("An agent with many skills".into()));
        assert!(back.capabilities.push_notifications);
        assert!(back.capabilities.state_transition_history);
        assert_eq!(back.default_input_modes.len(), 2);
        assert_eq!(back.skills.len(), 2);
        assert_eq!(back.skills[1].name, "compute");
        assert!(back.skills[1].description.is_none());
    }

    // 14. All A2ATaskState variants serde roundtrip
    #[test]
    fn test_a2a_task_state_variants() {
        let all_states = vec![
            A2ATaskState::Submitted,
            A2ATaskState::Working,
            A2ATaskState::Completed,
            A2ATaskState::Failed,
            A2ATaskState::Canceled,
        ];
        for state in &all_states {
            let json = serde_json::to_value(state).unwrap();
            let back: A2ATaskState = serde_json::from_value(json).unwrap();
            assert_eq!(&back, state);
        }
    }

    // 15. Empty parts A2AMessage doesn't panic
    #[test]
    fn test_empty_parts_a2a_message() {
        let msg = A2AMessage {
            message_id: "empty-msg".into(),
            role: A2ARole::Agent,
            parts: vec![],
            context_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: A2AMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.parts.len(), 0);

        // from_a2a with empty parts should not panic.
        let internal = A2AFormatter::from_a2a("agent", &msg).unwrap();
        // Blocks variant with empty vec.
        match &internal.content {
            MsgContent::Blocks(b) => assert!(b.is_empty()),
            _ => panic!("Expected empty Blocks content"),
        }
    }

    // 16. Image roundtrip through A2A
    #[test]
    fn test_a2a_roundtrip_with_image() {
        let blocks = vec![ContentBlock::Image(ImageBlock {
            data: Some("iVBORw0KGgo=".into()),
            url: Some("https://example.com/img.png".into()),
            mime_type: Some("image/png".into()),
        })];
        let msg = Msg::user("alice", MsgContent::Blocks(blocks));
        let a2a = A2AFormatter::to_a2a(&msg);

        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::File { file } => {
                assert_eq!(file.uri.as_deref(), Some("https://example.com/img.png"));
                assert_eq!(file.bytes.as_deref(), Some("iVBORw0KGgo="));
                assert_eq!(file.mime_type.as_deref(), Some("image/png"));
            }
            other => panic!("Expected File part, got {:?}", other),
        }

        // from_a2a should reconstruct the ImageBlock.
        let back = A2AFormatter::from_a2a("alice", &a2a).unwrap();
        match &back.content {
            MsgContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1);
                match &blocks[0] {
                    ContentBlock::Image(img) => {
                        assert_eq!(img.url.as_deref(), Some("https://example.com/img.png"));
                        assert_eq!(img.data.as_deref(), Some("iVBORw0KGgo="));
                        assert_eq!(img.mime_type.as_deref(), Some("image/png"));
                    }
                    other => panic!("Expected ImageBlock, got {:?}", other),
                }
            }
            other => panic!("Expected Blocks, got {:?}", other),
        }
    }

    // 17. from_a2a rejects unknown data type
    #[test]
    fn test_from_a2a_invalid_data_type() {
        let a2a = A2AMessage {
            message_id: "m-bad-type".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Data {
                data: serde_json::json!({ "kind": "unknown", "foo": "bar" }),
            }],
            context_id: None,
        };
        let result = A2AFormatter::from_a2a("agent", &a2a);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, A2AError::InvalidDataType(_)));
        assert!(err.to_string().contains("unknown"));
    }

    // 18. from_a2a rejects tool_use missing name
    #[test]
    fn test_from_a2a_missing_tool_use_fields() {
        let a2a = A2AMessage {
            message_id: "m-no-name".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Data {
                data: serde_json::json!({ "kind": "tool_use", "id": "call_1", "input": {} }),
                // "name" is missing
            }],
            context_id: None,
        };
        let result = A2AFormatter::from_a2a("agent", &a2a);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
    }

    // 19. from_a2a rejects tool_result missing tool_use_id
    #[test]
    fn test_from_a2a_missing_tool_result_fields() {
        let a2a = A2AMessage {
            message_id: "m-no-tuid".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Data {
                data: serde_json::json!({ "kind": "tool_result", "output": "ok" }),
                // "tool_use_id" is missing
            }],
            context_id: None,
        };
        let result = A2AFormatter::from_a2a("agent", &a2a);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
    }
}
