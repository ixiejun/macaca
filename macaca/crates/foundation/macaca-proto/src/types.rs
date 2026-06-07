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

    /// Create a deterministic ApplicationId from an app name.
    /// Same name always produces the same ID across restarts.
    pub fn from_name(name: &str) -> Self {
        // UUID v5 with a fixed namespace ensures deterministic IDs
        const MACACA_NS: Uuid = Uuid::from_bytes([
            0x6d, 0x61, 0x63, 0x61, 0x63, 0x61, 0x2d, 0x6f, 0x73, 0x2d, 0x61, 0x70, 0x70, 0x2d,
            0x6e, 0x73,
        ]);
        Self(Uuid::new_v5(&MACACA_NS, name.as_bytes()))
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

/// Structured worker profile used by planner decomposition contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationPlanningAgentProfile {
    pub name: String,
    pub capabilities: Vec<String>,
    pub available: bool,
    pub current_load: usize,
    pub max_load: usize,
    pub permission_level: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
}

impl ApplicationPlanningAgentProfile {
    pub fn legacy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capabilities: vec!["no capability metadata".into()],
            available: true,
            current_load: 0,
            max_load: 0,
            permission_level: "unknown".into(),
            model: "app default".into(),
            allowed_tools: vec![],
        }
    }
}

/// Stable application-level planning contract shared across app/task/web
/// boundaries for planner decomposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationTaskPlanningContract {
    pub workflow_name: String,
    pub entry_agent: String,
    pub worker_agents: Vec<ApplicationPlanningAgentProfile>,
}

impl ApplicationTaskPlanningContract {
    pub fn available_agent_names(&self) -> Vec<String> {
        self.worker_agents
            .iter()
            .map(|agent| agent.name.clone())
            .collect()
    }

    pub fn render_agent_profiles(&self) -> String {
        if self.worker_agents.is_empty() {
            return "(none)".to_string();
        }
        self.worker_agents
            .iter()
            .map(|agent| {
                let capabilities = if agent.capabilities.is_empty() {
                    "    - no capability metadata".to_string()
                } else {
                    agent
                        .capabilities
                        .iter()
                        .map(|capability| format!("    - {capability}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let tools = if agent.allowed_tools.is_empty() {
                    "all registered tools (open policy)".to_string()
                } else {
                    agent.allowed_tools.join(", ")
                };
                format!(
                    "- Agent `{}`\n  available: {}\n  load: {}/{}\n  permission: {}\n  model: {}\n  tools: {}\n  capabilities:\n{}",
                    agent.name,
                    agent.available,
                    agent.current_load,
                    agent.max_load,
                    agent.permission_level,
                    agent.model,
                    tools,
                    capabilities
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
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

// ── Todo System Types ──

/// Status of a todo item in the Task Board system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    /// Waiting to be claimed by an agent
    Pending,
    /// Agent has acknowledged the task
    Assigned,
    /// Agent is actively working on it
    InProgress,
    /// Agent finished, awaiting Plan Agent verification
    PendingReview,
    /// Verification failed, agent should retry with suggestions
    NeedsOptimization,
    /// Plan Agent verified – done
    Completed,
    /// Blocked by dependency tasks
    Blocked,
    /// Cancelled by Plan Agent
    Cancelled,
    /// Exceeded max attempts, needs human intervention
    Failed,
}

/// Service-owned classification for a task graph entry.
///
/// The value intentionally describes the owning Macaca service boundary instead
/// of an application, workflow, model, driver, provider, or business domain.
/// Application-execution projections use this marker to decide which tasks are
/// authoritative terminal facts for a run, while compatibility and diagnostic
/// tasks remain visible for audit without being allowed to fail an unrelated
/// execution.  The default keeps legacy persisted tasks out of the
/// application-execution terminal path unless a service explicitly marks them
/// as authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskGraphOwner {
    /// Task entries that are authoritative for `service.application_execution`
    /// run completion and failure projection.
    ApplicationExecution,
    /// Existing task-service, scheduler, chat, or goal-loop entries whose
    /// lifecycle is visible on the board but is not an application-execution
    /// terminal source unless a higher-level service explicitly binds it.
    #[default]
    TaskServiceNative,
    /// Compatibility fallback entries created while migrating legacy planner
    /// and Web loop behavior behind the Task Service boundary.
    TaskServiceCompatibility,
    /// Diagnostic entries that explain observations or failures but should
    /// never drive terminal execution state.
    DiagnosticOnly,
}

impl TaskGraphOwner {
    /// Return true when this graph owner may drive application-execution
    /// terminal aggregation.
    pub fn is_application_execution_authoritative(self) -> bool {
        matches!(self, Self::ApplicationExecution)
    }

    /// Stable label used in service metadata, trace events, and sanitized logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationExecution => "application_execution",
            Self::TaskServiceNative => "task_service_native",
            Self::TaskServiceCompatibility => "task_service_compatibility",
            Self::DiagnosticOnly => "diagnostic_only",
        }
    }
}

/// A single work item on an agent's task board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: TaskId,
    pub application_id: ApplicationId,
    /// Session this task belongs to (None = global / cross-session)
    #[serde(default)]
    pub session_id: Option<String>,
    /// Which agent this task is assigned to
    pub assigned_agent: String,
    /// Who created it (usually the plan agent / coordinator)
    pub created_by: String,
    /// Service boundary that owns this task entry for terminal aggregation.
    ///
    /// This field is a generic service classification.  It prevents a
    /// compatibility fallback task or diagnostic task from being interpreted as
    /// the authoritative terminal state of an application-execution run.  It is
    /// never allowed to encode application names, workflow names, provider
    /// names, programming languages, or product-domain semantics.
    #[serde(default)]
    pub graph_owner: TaskGraphOwner,

    // ── content ──
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    /// Extra context from parent goal or prior tasks
    pub context: Option<String>,

    // ── lifecycle ──
    pub status: TodoStatus,
    pub priority: u8,
    /// Execution order within this agent+session scope (1-based, ascending).
    /// Lower numbers execute first. 0 means unassigned (legacy data).
    #[serde(default)]
    pub sequence_number: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,

    // ── dependencies ──
    pub depends_on: Vec<TaskId>,
    pub parent_task: Option<TaskId>,

    // ── execution records ──
    /// Progress notes recorded during task execution
    #[serde(default)]
    pub progress_notes: Vec<String>,
    /// Agent's summary when submitting for review
    pub completion_summary: Option<String>,
    /// Plan Agent's feedback after review
    pub review_feedback: Option<String>,
    /// Suggestions when status = NeedsOptimization
    pub optimization_suggestions: Option<String>,
    pub attempt_count: u32,
    pub max_attempts: u32,
}

impl TodoItem {
    pub fn new(
        application_id: ApplicationId,
        session_id: Option<String>,
        assigned_agent: impl Into<String>,
        created_by: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        priority: u8,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            application_id,
            session_id,
            assigned_agent: assigned_agent.into(),
            created_by: created_by.into(),
            graph_owner: TaskGraphOwner::TaskServiceNative,
            title: title.into(),
            description: description.into(),
            acceptance_criteria: Vec::new(),
            context: None,
            status: TodoStatus::Pending,
            priority,
            sequence_number: 0,
            created_at: now,
            updated_at: now,
            deadline: None,
            depends_on: Vec::new(),
            parent_task: None,
            progress_notes: Vec::new(),
            completion_summary: None,
            review_feedback: None,
            optimization_suggestions: None,
            attempt_count: 0,
            max_attempts: 3,
        }
    }
}

/// Result of a Plan Agent reviewing a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoReviewResult {
    pub passed: bool,
    pub feedback: String,
    /// Per-criterion verification: (criterion text, passed?)
    pub verified_criteria: Vec<(String, bool)>,
}

/// High-level goal submitted to an application's task space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoGoal {
    pub id: TaskId,
    pub application_id: ApplicationId,
    /// Session this goal belongs to (None = global / cross-session)
    #[serde(default)]
    pub session_id: Option<String>,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub status: TodoGoalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoGoalStatus {
    Pending,
    /// Planner is decomposing the goal into subtasks.
    Decomposing,
    InProgress,
    /// GoalEvaluator is assessing whether the goal is satisfied.
    Evaluating,
    Completed,
    /// Goal was cancelled (e.g. session cleanup).
    Cancelled,
    /// Goal decomposition or evaluation failed.
    Failed,
}

/// Reference to tasks owned by another agent, used for cross-agent dependency declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTaskRef {
    /// Depend on ALL tasks assigned to the named agent.
    AllTasks { agent: String },
    /// Depend on a specific task (matched by title) assigned to the named agent.
    SpecificTask { agent: String, title: String },
}

impl TodoGoal {
    pub fn new(
        application_id: ApplicationId,
        session_id: Option<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: TaskId::new(),
            application_id,
            session_id,
            description: description.into(),
            created_at: Utc::now(),
            status: TodoGoalStatus::Pending,
        }
    }
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
    /// Provider-specific reasoning content returned by thinking models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
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
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::User,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message (plain text, no tool calls).
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with provider-specific reasoning content.
    pub fn assistant_with_reasoning(
        content: impl Into<String>,
        reasoning_content: impl Into<String>,
    ) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: Some(reasoning_content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message that requests tool calls.
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Tool,
            content: content.into(),
            reasoning_content: None,
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
    /// Provider-specific reasoning content returned by thinking models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
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

// ── Task Context (execution context passed with delegated tasks) ──

/// Additional context for task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Session ID for conversation continuity.
    pub session_id: Option<String>,
    /// Files or artifacts relevant to the task.
    pub artifacts: Vec<String>,
    /// Environment variables or configuration.
    pub env: std::collections::HashMap<String, String>,
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            session_id: None,
            artifacts: vec![],
            env: std::collections::HashMap::new(),
        }
    }
}

// ── Event Log Types ──

/// A persisted event entry in the EventLog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    /// Monotonically increasing sequence number (per-session).
    pub seq: u64,
    /// When the event occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Which session this event belongs to.
    pub session_id: String,
    /// Event type: "thinking", "tool_call", "tool_result", "assistant", "content", "done",
    /// "error", "delegated_task_start", "delegated_thinking", "delegated_tool_call",
    /// "delegated_tool_result", "delegated_assistant", "delegated_driver_trace",
    /// "delegated_completed", "delegated_task_complete", "delegated_task_error",
    /// "plan_decision", "loop_paused", "loop_resumed", "fork_created",
    /// "run_trace" (structured pipeline phases), etc.
    pub event_type: String,
    /// Source of the event: "coordinator", "executor:backend", "plan_loop", "worker_loop", etc.
    pub source: String,
    /// Event payload (varies by event_type).
    pub payload: serde_json::Value,
}

/// Structured payload for `event_type == "run_trace"` — pipeline / health checkpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTracePayload {
    /// Logical phase, e.g. `chat.request`, `delegate.task_start`, `coordinator.llm_error`.
    pub phase: String,
    /// Subsystem emitting the checkpoint: `coordinator`, `executor`, `plan_loop`, ...
    pub component: String,
    /// `ok` | `error` | `waiting` | `info`
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
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
    Assistant { content: String },
    /// Generic driver event trace
    DriverTrace {
        /// Source driver name
        driver_name: String,
        /// Serialized TraceEvent (serde_json::Value to avoid cross-crate type dependency)
        trace: serde_json::Value,
    },
    /// Execution completed
    Completed {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

pub trait AgentExecutionEventVisitor<R> {
    fn thinking(&mut self, iteration: usize, content: Option<&str>) -> R;
    fn tool_call(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        call_id: Option<&str>,
    ) -> R;
    fn tool_result(&mut self, tool_name: &str, output: &str, is_error: Option<bool>) -> R;
    fn assistant(&mut self, content: &str) -> R;
    fn driver_trace(&mut self, driver_name: &str, trace: &serde_json::Value) -> R;
    fn completed(&mut self, success: bool, error: Option<&str>) -> R;
}

impl AgentExecutionEvent {
    pub fn accept<R>(&self, visitor: &mut dyn AgentExecutionEventVisitor<R>) -> R {
        match self {
            AgentExecutionEvent::Thinking { iteration, content } => {
                visitor.thinking(*iteration, content.as_deref())
            }
            AgentExecutionEvent::ToolCall {
                tool_name,
                tool_input,
                call_id,
            } => visitor.tool_call(tool_name, tool_input, call_id.as_deref()),
            AgentExecutionEvent::ToolResult {
                tool_name,
                output,
                is_error,
            } => visitor.tool_result(tool_name, output, *is_error),
            AgentExecutionEvent::Assistant { content } => visitor.assistant(content),
            AgentExecutionEvent::DriverTrace { driver_name, trace } => {
                visitor.driver_trace(driver_name, trace)
            }
            AgentExecutionEvent::Completed { success, error } => {
                visitor.completed(*success, error.as_deref())
            }
        }
    }

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
    pub fn tool_call_with_id(
        tool_name: String,
        tool_input: serde_json::Value,
        call_id: String,
    ) -> Self {
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
    use serde_json::json;

    struct RecordingVisitor;

    impl AgentExecutionEventVisitor<String> for RecordingVisitor {
        fn thinking(&mut self, iteration: usize, content: Option<&str>) -> String {
            format!("thinking:{iteration}:{}", content.unwrap_or(""))
        }

        fn tool_call(
            &mut self,
            tool_name: &str,
            tool_input: &serde_json::Value,
            call_id: Option<&str>,
        ) -> String {
            format!(
                "tool_call:{tool_name}:{tool_input}:{}",
                call_id.unwrap_or("")
            )
        }

        fn tool_result(&mut self, tool_name: &str, output: &str, is_error: Option<bool>) -> String {
            format!(
                "tool_result:{tool_name}:{output}:{}",
                is_error.unwrap_or(false)
            )
        }

        fn assistant(&mut self, content: &str) -> String {
            format!("assistant:{content}")
        }

        fn driver_trace(&mut self, driver_name: &str, trace: &serde_json::Value) -> String {
            format!("driver_trace:{driver_name}:{trace}")
        }

        fn completed(&mut self, success: bool, error: Option<&str>) -> String {
            format!("completed:{success}:{}", error.unwrap_or(""))
        }
    }

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
    fn agent_execution_event_visitor_dispatches_correctly() {
        let event = AgentExecutionEvent::ToolCall {
            tool_name: "file_read".into(),
            tool_input: json!({"path":"/tmp/a.txt"}),
            call_id: Some("call-1".into()),
        };
        let mut visitor = RecordingVisitor;
        let result = event.accept(&mut visitor);
        assert_eq!(
            result,
            r#"tool_call:file_read:{"path":"/tmp/a.txt"}:call-1"#
        );
    }

    #[test]
    fn agent_execution_event_serde_shape_is_unchanged() {
        let event = AgentExecutionEvent::Thinking {
            iteration: 2,
            content: Some("planning".into()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["iteration"], 2);
        assert_eq!(json["content"], "planning");
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
            reasoning_content: None,
            model: "gpt-4".into(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
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
    fn todo_item_sequence_number_default() {
        let item = TodoItem::new(
            ApplicationId::new(),
            None,
            "agent1",
            "plan",
            "Test task",
            "desc",
            5,
        );
        assert_eq!(item.sequence_number, 0);
    }

    #[test]
    fn todo_item_serialize_with_sequence_number() {
        let mut item = TodoItem::new(
            ApplicationId::new(),
            None,
            "agent1",
            "plan",
            "Test",
            "desc",
            5,
        );
        item.sequence_number = 3;
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"sequence_number\":3"));
        let parsed: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sequence_number, 3);
    }

    #[test]
    fn todo_item_deserialize_without_sequence_number() {
        // Simulate legacy data without sequence_number field
        let json = serde_json::json!({
            "id": TaskId::new(),
            "application_id": ApplicationId::new(),
            "assigned_agent": "agent1",
            "created_by": "plan",
            "title": "Legacy task",
            "description": "desc",
            "acceptance_criteria": [],
            "status": "Pending",
            "priority": 5,
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-01T00:00:00Z",
            "depends_on": [],
            "progress_notes": [],
            "attempt_count": 0,
            "max_attempts": 3
        });
        let item: TodoItem = serde_json::from_value(json).unwrap();
        assert_eq!(item.sequence_number, 0); // default
    }

    #[test]
    fn agent_task_ref_serialize_roundtrip() {
        let all = AgentTaskRef::AllTasks {
            agent: "architect".into(),
        };
        let json = serde_json::to_string(&all).unwrap();
        let parsed: AgentTaskRef = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentTaskRef::AllTasks { agent } => assert_eq!(agent, "architect"),
            _ => panic!("Expected AllTasks"),
        }

        let specific = AgentTaskRef::SpecificTask {
            agent: "backend".into(),
            title: "Design API".into(),
        };
        let json = serde_json::to_string(&specific).unwrap();
        let parsed: AgentTaskRef = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentTaskRef::SpecificTask { agent, title } => {
                assert_eq!(agent, "backend");
                assert_eq!(title, "Design API");
            }
            _ => panic!("Expected SpecificTask"),
        }
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
