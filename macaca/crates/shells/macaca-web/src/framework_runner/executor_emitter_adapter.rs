//! Executor Adapter: bridges lifecycle and tools to executor broadcast events.

use super::tool_trace::{tool_call_event, tool_result_event, tool_trace_output};
use async_trait::async_trait;
use macaca_host_composition::framework::agent::Hook;
use macaca_host_composition::framework::message::Msg;
use macaca_host_composition::framework::tool::{ToolError, ToolMiddleware, ToolResponse};
use std::sync::atomic::Ordering;
use std::sync::Arc;
pub struct ExecutorEmitterHook {
    pub(crate) executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
    pub(crate) task_id: macaca_proto::TaskId,
    pub(crate) agent_name: String,
    pub(crate) iteration: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl Hook for ExecutorEmitterHook {
    async fn pre_reply(
        &self,
        msg: Msg,
    ) -> macaca_host_composition::framework::agent::AgentResult<Msg> {
        let iter = self.iteration.fetch_add(1, Ordering::Relaxed);
        self.executor.broadcast_event(
            macaca_host_composition::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: macaca_proto::AgentExecutionEvent::Thinking {
                    iteration: iter,
                    content: None,
                },
            },
        );
        Ok(msg)
    }

    async fn post_reply(
        &self,
        msg: Msg,
    ) -> macaca_host_composition::framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if !text.is_empty() {
            self.executor.broadcast_event(
                macaca_host_composition::executor::ExecutorEvent::AgentEvent {
                    task_id: self.task_id,
                    agent: self.agent_name.clone(),
                    event: macaca_proto::AgentExecutionEvent::Assistant { content: text },
                },
            );
        }
        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// ExecutorToolMiddleware — bridges tool calls/results to executor broadcast
// ---------------------------------------------------------------------------

/// Middleware that emits executor events for every tool invocation.
/// Used by worker agents to push tool_call/tool_result events to SSE + EventLog.
pub struct ExecutorToolMiddleware {
    pub(crate) executor: Arc<macaca_host_composition::executor::ApplicationExecutor>,
    pub(crate) task_id: macaca_proto::TaskId,
    pub(crate) agent_name: String,
}

#[async_trait]
impl ToolMiddleware for ExecutorToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        self.executor.broadcast_event(
            macaca_host_composition::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_call_event(name, args),
            },
        );
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        self.executor.broadcast_event(
            macaca_host_composition::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_result_event(name, tool_trace_output(response)),
            },
        );
        Ok(())
    }
}
