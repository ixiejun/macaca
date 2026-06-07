//! Channel Adapter: bridges lifecycle and tools to `AgentExecutionEvent` channels.

use std::sync::atomic::Ordering;
use async_trait::async_trait;
use macaca_framework::agent::Hook;
use macaca_framework::message::Msg;
use macaca_framework::tool::{ToolError, ToolMiddleware, ToolResponse};
use tokio::sync::mpsc;
use super::tool_trace::{tool_call_event, tool_result_event, tool_trace_output};
pub struct ChannelEmitterHook {
    pub(crate) tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
    pub(crate) iteration: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl Hook for ChannelEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let iter = self.iteration.fetch_add(1, Ordering::Relaxed);
        let _ = self
            .tx
            .send(macaca_proto::AgentExecutionEvent::Thinking {
                iteration: iter,
                content: None,
            })
            .await;
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if !text.is_empty() {
            let _ = self
                .tx
                .send(macaca_proto::AgentExecutionEvent::Assistant { content: text })
                .await;
        }
        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// ChannelToolMiddleware — bridges tool calls/results to AgentExecutionEvent
// ---------------------------------------------------------------------------

pub struct ChannelToolMiddleware {
    pub(crate) tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
}

#[async_trait]
impl ToolMiddleware for ChannelToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        let _ = self.tx.send(tool_call_event(name, args)).await;
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        let _ = self
            .tx
            .send(tool_result_event(name, tool_trace_output(response)))
            .await;
        Ok(())
    }
}
