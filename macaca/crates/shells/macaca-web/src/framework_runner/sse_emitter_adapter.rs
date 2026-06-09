//! SSE Adapter: bridges ReActAgent lifecycle and tool events to HTTP SSE + EventLog.

use std::convert::Infallible;
use std::sync::Arc;
use async_trait::async_trait;
use axum::response::sse::Event;
use macaca_sdk::framework::agent::Hook;
use macaca_sdk::framework::message::Msg;
use macaca_sdk::framework::tool::{ToolError, ToolMiddleware, ToolResponse};
use macaca_sdk::runtime_host::persist::{AppendEventCommand, EventLog};
use tokio::sync::mpsc;
use super::tool_trace::tool_trace_output;
pub struct SseEmitterHook {
    pub(crate) tx: mpsc::Sender<Result<Event, Infallible>>,
    pub(crate) agent_name: String,
    pub(crate) event_log: Option<Arc<EventLog>>,
    pub(crate) session_id: Option<String>,
}

#[async_trait]
impl Hook for SseEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_sdk::framework::agent::AgentResult<Msg> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            // Event log agent attribution follows the manifest-resolved entry agent
            // carried by the hook adapter (Strategy: inject runtime agent identity).
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "thinking",
                    &self.agent_name,
                    serde_json::json!({
                        "iteration": 0,
                    }),
                ))
                .await;
        }
        let event = Event::default().event("thinking").data(
            serde_json::json!({
                "iteration": 0,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_sdk::framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "content",
                    &self.agent_name,
                    serde_json::json!({
                        "content": text,
                    }),
                ))
                .await;
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "done",
                    &self.agent_name,
                    serde_json::json!({
                        "model": "",
                        "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                        "iterations": 0,
                        "tools_used": [],
                    }),
                ))
                .await;
        }
        let content_event = Event::default().event("content").data(
            serde_json::json!({
                "content": text,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(content_event)).await;

        let done_event = Event::default().event("done").data(
            serde_json::json!({
                "model": "",
                "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                "iterations": 0,
                "tools_used": [],
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(done_event)).await;

        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// SseToolMiddleware — bridges tool calls/results to SSE
// ---------------------------------------------------------------------------

/// Middleware that emits SSE events for every tool invocation.
pub struct SseToolMiddleware {
    pub(crate) tx: mpsc::Sender<Result<Event, Infallible>>,
    pub(crate) agent_name: String,
    pub(crate) event_log: Option<Arc<EventLog>>,
    pub(crate) session_id: Option<String>,
}

#[async_trait]
impl ToolMiddleware for SseToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "tool_call",
                    &self.agent_name,
                    serde_json::json!({
                        "tool_name": name,
                        "tool_input": args.clone(),
                    }),
                ))
                .await;
            if name == "file_read" {
                if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
                    if path.ends_with("SKILL.md") && path.contains("/skills/") {
                        event_log
                            .append_command(AppendEventCommand::new(
                                session_id,
                                "skill_file_read",
                                &self.agent_name,
                                serde_json::json!({
                                    "agent": self.agent_name,
                                    "path": path,
                                }),
                            ))
                            .await;
                    }
                }
            }
        }
        let event = Event::default().event("tool_call").data(
            serde_json::json!({
                "tool_name": name,
                "tool_input": args.clone(),
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        let display_result = tool_trace_output(response);

        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append_command(AppendEventCommand::new(
                    session_id,
                    "tool_result",
                    &self.agent_name,
                    serde_json::json!({
                        "tool_name": name,
                        "output": display_result,
                    }),
                ))
                .await;
        }

        let event = Event::default().event("tool_result").data(
            serde_json::json!({
                "tool_name": name,
                "output": display_result,
            })
            .to_string(),
        );
        let _ = self.tx.send(Ok(event)).await;
        Ok(())
    }
}
