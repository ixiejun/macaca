use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Identity Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a forked (child) agent instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForkId(pub Uuid);

impl ForkId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ForkId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ForkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fork-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationId(pub Uuid);

impl ApplicationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ApplicationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ApplicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DriverId(pub Uuid);

impl DriverId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DriverId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DriverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Agent Types ──

/// Lifecycle state of an agent (static).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Created,
    Running,
    Suspended,
    Terminated,
}

/// Runtime activity status of an agent (dynamic).
/// Simplified to 4 core states: IDLE, WORKING, ERROR, THINKING.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentActivity {
    /// Agent is idle, waiting for tasks.
    Idle,
    /// Agent is actively working (executing tools, processing).
    Working {
        /// Brief description of what's being worked on.
        context: String,
    },
    /// Agent encountered an error.
    Error {
        /// Error description.
        message: String,
    },
    /// Agent is thinking/processing (LLM call in progress).
    Thinking {
        /// Brief description of what's being processed.
        context: String,
    },
}

impl Default for AgentActivity {
    fn default() -> Self {
        Self::Idle
    }
}

/// Lifecycle state of a forked (child) agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkState {
    /// Fork created, waiting to start execution.
    Pending,
    /// Fork is actively executing.
    Running,
    /// Fork delegated a task and is waiting for hook callback.
    WaitingForHook,
    /// Fork's delegated task completed successfully.
    Completed,
    /// Fork's delegated task failed.
    Failed { error: String },
    /// Fork completed and merged back to parent.
    Merged,
    /// Fork was cancelled.
    Cancelled,
}

/// Criteria for accepting a fork's result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    /// Human-readable description of success criteria.
    pub description: String,
    /// Required artifacts (file paths, URLs, etc.).
    pub required_artifacts: Vec<String>,
    /// Whether to auto-accept on success without validation.
    pub auto_accept: bool,
}

impl Default for AcceptanceCriteria {
    fn default() -> Self {
        Self {
            description: "Task completed successfully".into(),
            required_artifacts: vec![],
            auto_accept: false,
        }
    }
}

/// Result of validating a fork's output against acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationResult {
    /// Validation passed.
    Accepted,
    /// Validation failed.
    Rejected { reason: String },
    /// Unable to validate (e.g., timeout, missing artifacts).
    Unavailable { reason: String },
}

/// Runtime status of an agent, combining lifecycle state and current activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeStatus {
    /// Agent's unique ID.
    pub agent_id: AgentId,
    /// Agent's name.
    pub name: String,
    /// Lifecycle state (Created, Running, etc.).
    pub state: AgentState,
    /// Current activity (what the agent is doing right now).
    pub activity: AgentActivity,
    /// Timestamp of last status update.
    pub updated_at: DateTime<Utc>,
    /// Current task description (if any).
    pub current_task: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// System agent — full access (kernel mode)
    System,
    /// User agent — restricted access (user mode)
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub level: PermissionLevel,
    pub allowed_tools: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub network_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub permission: Permission,
    pub state: AgentState,
    pub created_at: DateTime<Utc>,
    /// Preferred LLM model for this agent (e.g., "", "gpt-4o").
    /// If empty, uses the application's default model.
    #[serde(default)]
    pub model: String,
}

// ── Task Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub description: String,
    pub priority: TaskPriority,
    pub requester: String,
}

impl TaskRequest {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            priority: TaskPriority::Normal,
            requester: String::from("user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assigned_agent: Option<AgentId>,
    pub subtasks: Vec<TaskId>,
    pub parent: Option<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

// ── Memory Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLayer {
    Session,
    File,
    Vector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub layer: MemoryLayer,
    pub content: String,
    pub metadata: serde_json::Value,
    pub agent_id: Option<AgentId>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResult {
    pub entries_before: usize,
    pub entries_after: usize,
    pub bytes_saved: usize,
}

// ── Message Types (IPC) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: MessageId,
    pub from: AgentId,
    pub to: Option<AgentId>,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

// ── Gateway Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayEvent {
    TaskRequest {
        user_id: String,
        channel_id: String,
        content: String,
    },
    StatusQuery {
        user_id: String,
        channel_id: String,
        task_id: Option<TaskId>,
    },
    UserReply {
        user_id: String,
        channel_id: String,
        content: String,
        context_id: String,
    },
    Command {
        user_id: String,
        channel_id: String,
        command: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMessage {
    pub content: String,
    pub format: MessageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageFormat {
    PlainText,
    Markdown,
    CodeBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachment {
    pub filename: String,
    pub data: Vec<u8>,
    pub mime_type: String,
}

// ── LLM Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool call requested by the LLM (function calling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// A tool definition sent to the LLM so it knows what tools are available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
    /// Tool calls requested by the assistant (present when role=Assistant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The tool_call id this message is responding to (present when role=Tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl LlmMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message (plain text, no tool calls).
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message that requests tool calls.
    pub fn assistant_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOptions {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
    /// Tool definitions to send to the LLM for function calling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

impl Default for LlmOptions {
    fn default() -> Self {
        Self {
            model: String::from("gpt-4"),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stop_sequences: Vec::new(),
            tools: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: String,
    /// Tool calls requested by the model (if finish_reason indicates tool use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Agent Output ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub result: String,
    pub artifacts: Vec<String>,
    pub tokens_used: TokenUsage,
}

// ── Task Context (for memory retrieval) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_id: TaskId,
    pub description: String,
    pub agent_id: AgentId,
    pub history: Vec<String>,
}

// ── Agent Execution Events ──

/// Events emitted during agent execution for progress tracking.
/// These events are used to report detailed execution progress to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentExecutionEvent {
    /// Agent is thinking (internal reasoning)
    Thinking {
        iteration: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },
    /// Agent is making a tool call
    ToolCall {
        tool_name: String,
        tool_input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
    },
    /// Tool execution result
    ToolResult {
        tool_name: String,
        output: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Agent produced assistant content
    Assistant {
        content: String,
    },
    /// Claude Code execution trace (for claude_code_execute tool)
    CcTrace {
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_input: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_result: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Execution completed
    Completed {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

impl AgentExecutionEvent {
    /// Create a thinking event
    pub fn thinking(iteration: usize) -> Self {
        Self::Thinking {
            iteration,
            content: None,
        }
    }

    /// Create a thinking event with content
    pub fn thinking_with_content(iteration: usize, content: String) -> Self {
        Self::Thinking {
            iteration,
            content: Some(content),
        }
    }

    /// Create a tool call event
    pub fn tool_call(tool_name: String, tool_input: serde_json::Value) -> Self {
        Self::ToolCall {
            tool_name,
            tool_input,
            call_id: None,
        }
    }

    /// Create a tool call event with call ID
    pub fn tool_call_with_id(tool_name: String, tool_input: serde_json::Value, call_id: String) -> Self {
        Self::ToolCall {
            tool_name,
            tool_input,
            call_id: Some(call_id),
        }
    }

    /// Create a tool result event
    pub fn tool_result(tool_name: String, output: String) -> Self {
        Self::ToolResult {
            tool_name,
            output,
            is_error: None,
        }
    }

    /// Create a tool result event with error flag
    pub fn tool_result_with_error(tool_name: String, output: String, is_error: bool) -> Self {
        Self::ToolResult {
            tool_name,
            output,
            is_error: Some(is_error),
        }
    }

    /// Create an assistant event
    pub fn assistant(content: String) -> Self {
        Self::Assistant { content }
    }

    /// Create a completed event
    pub fn completed(success: bool, error: Option<String>) -> Self {
        Self::Completed { success, error }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_is_unique() {
        let a = AgentId::new();
        let b = AgentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn application_id_is_unique() {
        let a = ApplicationId::new();
        let b = ApplicationId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn driver_id_is_unique() {
        let a = DriverId::new();
        let b = DriverId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn application_id_display() {
        let id = ApplicationId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
    }

    #[test]
    fn task_request_new() {
        let req = TaskRequest::new("build a web app");
        assert_eq!(req.description, "build a web app");
        assert_eq!(req.priority, TaskPriority::Normal);
    }

    #[test]
    fn types_serialize_roundtrip() {
        let entry = MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Vector,
            content: "test memory".into(),
            metadata: serde_json::json!({"key": "value"}),
            agent_id: Some(AgentId::new()),
            created_at: Utc::now(),
            expires_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, "test memory");
        assert_eq!(parsed.layer, MemoryLayer::Vector);
    }

    #[test]
    fn gateway_event_serialize() {
        let event = GatewayEvent::TaskRequest {
            user_id: "u123".into(),
            channel_id: "c456".into(),
            content: "build me an app".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TaskRequest"));
    }

    #[test]
    fn task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    // ── LLM Message constructors ──

    #[test]
    fn llm_message_user_constructor() {
        let msg = LlmMessage::user("hello");
        assert_eq!(msg.role, LlmRole::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn llm_message_system_constructor() {
        let msg = LlmMessage::system("you are helpful");
        assert_eq!(msg.role, LlmRole::System);
        assert_eq!(msg.content, "you are helpful");
    }

    #[test]
    fn llm_message_tool_result_constructor() {
        let msg = LlmMessage::tool_result("call_123", r#"{"result": "ok"}"#);
        assert_eq!(msg.role, LlmRole::Tool);
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn llm_message_assistant_with_tool_calls() {
        let calls = vec![ToolCall {
            id: "call_1".into(),
            name: "file_read".into(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        }];
        let msg = LlmMessage::assistant_with_tool_calls("", calls);
        assert_eq!(msg.role, LlmRole::Assistant);
        assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(msg.tool_calls.as_ref().unwrap()[0].name, "file_read");
    }

    // ── Tool calling types serialization ──

    #[test]
    fn tool_call_serialize_roundtrip() {
        let tc = ToolCall {
            id: "call_abc".into(),
            name: "shell".into(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "call_abc");
        assert_eq!(parsed.name, "shell");
    }

    #[test]
    fn tool_definition_serialize_roundtrip() {
        let td = ToolDefinition {
            name: "file_read".into(),
            description: "Read a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };
        let json = serde_json::to_string(&td).unwrap();
        let parsed: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "file_read");
    }

    #[test]
    fn llm_message_backward_compatible_deserialization() {
        // Old format without tool_calls/tool_call_id should still parse
        let json = r#"{"role":"User","content":"hello"}"#;
        let msg: LlmMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, LlmRole::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn llm_response_with_tool_calls() {
        let resp = LlmResponse {
            content: String::new(),
            model: "gpt-4".into(),
            usage: TokenUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
            finish_reason: "tool_calls".into(),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".into(),
                name: "shell".into(),
                arguments: serde_json::json!({"command": "echo hi"}),
            }]),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: LlmResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.finish_reason, "tool_calls");
        assert_eq!(parsed.tool_calls.unwrap().len(), 1);
    }

    #[test]
    fn llm_response_backward_compatible_deserialization() {
        // Old format without tool_calls should still parse
        let json = r#"{"content":"hi","model":"gpt-4","usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2},"finish_reason":"stop"}"#;
        let resp: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content, "hi");
        assert!(resp.tool_calls.is_none());
    }

    #[test]
    fn llm_options_with_tools() {
        let opts = LlmOptions {
            tools: Some(vec![ToolDefinition {
                name: "test".into(),
                description: "test tool".into(),
                parameters: serde_json::json!({}),
            }]),
            ..Default::default()
        };
        assert_eq!(opts.tools.as_ref().unwrap().len(), 1);
    }
}
