//! Explicit framework contract surface for SDK composition roots.
//!
//! Framework owns agent traits, construction DTOs, message/model contracts, MCP
//! helpers, and ReAct/toolkit runtime abstractions.  Runtime-host forwards the
//! SDK-published subset so the SDK facade does not depend on framework directly.

pub use macaca_framework::agent::{
    Agent, AgentError, AgentResult, Hook, HookRegistry, HookedAgent,
};
pub use macaca_framework::construction::{
    AgentBuildIntent, AgentBuildRequest, AgentBuildRequestBuilder, AgentCapabilitySet,
    AgentExecutionInput, AgentExecutionLauncher, AgentExecutionOutput, AgentIdentity,
    AgentLifecycleConfig, AgentServices, AgentToolConfig, AgentTraceContext, AgentTransitionReason,
    ApplicationPromptParts, ApplicationSemantics, ApplicationToolPolicy, ToolkitContributor,
    TracedAgentFactory,
};
pub use macaca_framework::execution::{ExecutionContext, ExecutionStatus};
pub use macaca_framework::formatter::{
    AnthropicFormatter, DashScopeFormatter, Formatter, FormatterError, OpenAiFormatter,
};
pub use macaca_framework::llm_wire::{
    chat_options_to_llm_options, llm_messages_to_json_values, messages_from_json_values,
};
pub use macaca_framework::mcp::{
    client_from_transport, register_mcp_tools, register_mcp_tools_with_options, McpCallResult,
    McpClient, McpError, McpResourceDef, McpResourceRead, McpResourceTemplateDef, McpSessionMode,
    McpTimeouts, McpToolDef, McpToolHandler, McpToolNameConflictPolicy, McpToolRegistrationOptions,
    McpTransportConfig,
};
pub use macaca_framework::message::{
    AudioBlock, ContentBlock, DataBlock, GenerateReason, HintBlock, ImageBlock, MessageUsage, Msg,
    MsgContent, MsgValidationError, Role, TextBlock, ThinkingBlock, ToolCallState, ToolResultBlock,
    ToolResultState, ToolUseBlock, VideoBlock,
};
pub use macaca_framework::model::{
    ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError, ToolChoice,
};
pub use macaca_framework::plan::{Plan, PlanError, PlanNotebook, PlanState, SubTask, SubTaskState};
pub use macaca_framework::react_agent::{AgentRunCommand, AgentRunResult, ReActAgent};
pub use macaca_framework::runtime_context::{
    AgentSessionError, AgentSessionHealth, AgentSessionHealthState, AgentSessionStore,
    AgentSessionStoreSnapshot, AgentState, CachedRead, InMemoryAgentSessionStore, PendingToolState,
    PermissionState, PlanModeState, RuntimeContext, RuntimeContextError, SessionKey,
    TaskContextState,
};
pub use macaca_framework::tool::{
    RegisteredTool, ToolError, ToolGroup, ToolHandler, ToolMiddleware, ToolResponse,
    ToolTraceEvent, Toolkit, ToolkitResource,
};
