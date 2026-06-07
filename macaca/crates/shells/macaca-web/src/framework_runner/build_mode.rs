//! Build-mode enums and driver-trace routing helpers.

use std::convert::Infallible;
use std::sync::Arc;
use axum::response::sse::Event;
use tokio::sync::mpsc;
use macaca_framework::model::ToolChoice;
use macaca_runtime_host::persist::EventLog;
use crate::state::AppState;
use super::runtime_execution_control::RuntimeExecutionControl;
pub(crate) enum FrameworkRunnerBuildMode {
    Executor {
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
    },
    Runtime {
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: Option<RuntimeExecutionControl>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    },
    Coordinator {
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        execution_control: RuntimeExecutionControl,
    },
}
pub(crate) enum StandardAgentMode {
    Executor {
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
    },
    Runtime {
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
        execution_control: Option<RuntimeExecutionControl>,
    },
}

pub(crate) enum DriverTraceRoute {
    Executor {
        state: Arc<AppState>,
        executor: Arc<macaca_runtime_host::executor::ApplicationExecutor>,
        task_id: macaca_proto::TaskId,
        agent_name: String,
        session_id: Option<String>,
    },
    Runtime {
        tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
    },
    Coordinator {
        tx: mpsc::Sender<Result<Event, Infallible>>,
        event_log: Arc<EventLog>,
        agent_name: String,
        session_id: Option<String>,
    },
}

impl DriverTraceRoute {
    /// Return a bounded route label for structured diagnostics.
    ///
    /// The label is intentionally static and provider-neutral. It is used only
    /// when trace routing suppresses a framework wrapper event that is already
    /// represented by a semantic agent-execution tool event.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            DriverTraceRoute::Executor { .. } => "executor",
            DriverTraceRoute::Runtime { .. } => "runtime",
            DriverTraceRoute::Coordinator { .. } => "coordinator",
        }
    }
}

/// Return whether a tool trace is only the framework's generic wrapper event.
///
/// Macaca emits semantic `AgentExecutionEvent::ToolCall` and
/// `AgentExecutionEvent::ToolResult` events at the agent-execution layer. The
/// lower tool-command pipeline also emits `TraceEvent` values for the same
/// call/result lifecycle when a concrete driver does not provide its own trace
/// identity. Those no-driver wrapper traces are useful as a fallback in raw
/// tool pipelines, but forwarding them as `DriverTrace` inside agent execution
/// would persist the same logical operation twice. Concrete driver/provider
/// traces still pass through because they carry a real `driver_id` or a richer
/// diagnostic event type.
pub(crate) fn is_framework_tool_wrapper_trace(trace: &macaca_sdk::tools::TraceEvent) -> bool {
    trace.driver_id.is_none() && matches!(trace.event_type.as_str(), "tool_call" | "tool_result")
}

/// Decide whether a trace event should be forwarded as a driver trace.
///
/// This small Specification keeps Executor, Runtime, and Coordinator routing in
/// sync. It avoids frontend-only hiding and keeps EventLog replay faithful:
/// semantic tool events remain durable, while redundant framework wrappers are
/// suppressed before they become driver-trace events.
pub(crate) fn should_forward_driver_trace(trace: &macaca_sdk::tools::TraceEvent) -> bool {
    !is_framework_tool_wrapper_trace(trace)
}
