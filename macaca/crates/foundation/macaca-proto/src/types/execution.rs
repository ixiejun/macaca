//! Execution context, event journal entries, run traces, and visitor-dispatched events.
//!
//! The Visitor pattern (`AgentExecutionEventVisitor`) enables SDK bridges and audit
//! sinks without coupling domain events to a single serialization format.

use serde::{Deserialize, Serialize};

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
