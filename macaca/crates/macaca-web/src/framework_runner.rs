//! Framework-based agent runner — builds `ReActAgent` instances from persona
//! configuration and bridges events to SSE.
//!
//! This module replaces the ad-hoc `AgenticLoop` execution with the
//! `macaca-framework` `ReActAgent`, providing:
//! - Unified tool management via `Toolkit` (with middleware chain)
//! - Working memory with tag-based filtering
//! - Hook system for SSE event bridging
//! - Pause/resume via `ToolMiddleware` for `create_goal` coordination

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use axum::response::sse::Event;
use tokio::sync::{mpsc, Mutex};

use macaca_framework::adapter::RoutedLlmAdapter;
use macaca_framework::agent::{Hook, HookRegistry, HookedAgent};
use macaca_framework::formatter::OpenAiFormatter;
use macaca_framework::memory::InMemoryWorkingMemory;
use macaca_framework::message::Msg;
use macaca_framework::react_agent::ReActAgent;
use macaca_framework::tool::{ToolError, ToolMiddleware, ToolResponse};
use macaca_persist::EventLog;
use macaca_proto::ApplicationId;
use macaca_runtime::agentic_loop::ResumeReason;
use macaca_sdk::AgentPersona;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// FrameworkRunner — Agent factory
// ---------------------------------------------------------------------------

/// Builds `ReActAgent` instances from the existing Macaca OS infrastructure.
///
/// This is the bridge between the OS layer (AppState, personas, tool registry)
/// and the framework layer (ReActAgent, Toolkit, WorkingMemory).
pub struct FrameworkRunner;

impl FrameworkRunner {
    /// Deprecated: do not use. All agents must be constructed through traced
    /// builders so execution is visible in EventLog and SSE.
    #[deprecated(
        note = "build_agent is disabled. Use build_traced_agent/build_traced_agent_with_goal/build_worker_agent/build_coordinator instead."
    )]
    pub async fn build_agent(
        _state: &Arc<AppState>,
        _app_id: &ApplicationId,
        _agent_name: &str,
        _session_id: Option<String>,
    ) -> Result<ReActAgent, String> {
        Err("FrameworkRunner::build_agent is disabled. Use a traced builder instead.".into())
    }

    /// Deprecated: do not use. All agents must be constructed through traced
    /// builders so execution is visible in EventLog and SSE.
    #[deprecated(
        note = "build_agent_with_goal is disabled. Use build_traced_agent_with_goal instead."
    )]
    pub async fn build_agent_with_goal(
        _state: &Arc<AppState>,
        _app_id: &ApplicationId,
        _agent_name: &str,
        _session_id: Option<String>,
        _goal_id: Option<macaca_proto::TaskId>,
    ) -> Result<ReActAgent, String> {
        Err(
            "FrameworkRunner::build_agent_with_goal is disabled. Use a traced builder instead."
                .into(),
        )
    }

    /// Build a traced `ReActAgent` without goal context.
    pub async fn build_traced_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_traced_agent_with_goal(
            state, app_id, agent_name, session_id, task_id, executor, None,
        )
        .await
    }

    /// Build a worker `ReActAgent` wrapped with `HookedAgent` that emits execution
    /// events (thinking, tool_call, tool_result, assistant) to the executor broadcast
    /// channel for SSE + EventLog persistence.
    pub async fn build_worker_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        Self::build_traced_agent(state, app_id, agent_name, session_id, task_id, executor).await
    }

    /// Build a traced `ReActAgent` that emits execution events through the
    /// executor broadcast channel. Supports optional goal context so planner
    /// calls to `create_todo` can be linked to the active goal.
    pub async fn build_traced_agent_with_goal(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        task_id: macaca_proto::TaskId,
        executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
        goal_id: Option<macaca_proto::TaskId>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit = crate::framework_toolkit::build_toolkit(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            goal_id,
        )
        .await;

        // Executor tool middleware — emits tool_call / tool_result via broadcast
        toolkit.add_middleware(Box::new(ExecutorToolMiddleware {
            executor: Arc::clone(&executor),
            task_id,
            agent_name: agent_name.to_string(),
        }));

        let model_name = selection.primary.reference();

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(25)
            .with_model_name(model_name);

        // Wrap with HookedAgent + ExecutorEmitterHook
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(ExecutorEmitterHook {
            executor: Arc::clone(&executor),
            task_id,
            agent_name: agent_name.to_string(),
            iteration: std::sync::atomic::AtomicUsize::new(0),
        }));
        let hooked = HookedAgent::new(agent, hooks);

        Ok(hooked)
    }

    /// Build a framework-native runtime agent for executor call sites that
    /// still depend on `AgentRunner`. Optional event channels receive
    /// `AgentExecutionEvent` updates directly from framework hooks.
    pub async fn build_runtime_agent(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        goal_id: Option<macaca_proto::TaskId>,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<HookedAgent<ReActAgent>, String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit =
            crate::framework_toolkit::build_toolkit(state, app_id, agent_name, session_id, goal_id)
                .await;

        if let Some(ref tx) = event_tx {
            toolkit.add_middleware(Box::new(ChannelToolMiddleware { tx: tx.clone() }));
        }

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(25)
            .with_model_name(selection.primary.reference());

        let mut hooks = HookRegistry::new();
        if let Some(tx) = event_tx {
            hooks.register_instance_hook(Box::new(ChannelEmitterHook {
                tx,
                iteration: std::sync::atomic::AtomicUsize::new(0),
            }));
        }

        Ok(HookedAgent::new(agent, hooks))
    }

    /// Build a coordinator `ReActAgent` wrapped with `HookedAgent` for SSE bridging
    /// and `PauseOnGoalMiddleware` for pause/resume on `create_goal`.
    ///
    /// Returns `(HookedAgent<ReActAgent>, CancellationToken)`.
    pub async fn build_coordinator(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        pause_signal: Arc<AtomicBool>,
        resume_rx: mpsc::Receiver<ResumeReason>,
    ) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
        let system_prompt = Self::build_system_prompt(state, app_id, agent_name).await;
        let selection = Self::resolve_model_selection(state, app_id, agent_name).await?;
        let model = Arc::new(RoutedLlmAdapter::new(
            Arc::clone(&state.llm_router),
            selection.clone(),
        ));
        let formatter = Arc::new(OpenAiFormatter);
        let mut toolkit = crate::framework_toolkit::build_toolkit(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            None,
        )
        .await;

        // SSE tool middleware — emits tool_call / tool_result events
        toolkit.add_middleware(Box::new(SseToolMiddleware {
            tx: sse_tx.clone(),
            agent_name: agent_name.to_string(),
            event_log: Some(Arc::clone(&state.persist.event_log)),
            session_id: session_id.clone(),
        }));

        // Pause-on-goal middleware — blocks until goal completes
        toolkit.add_middleware(Box::new(PauseOnGoalMiddleware {
            pause_signal,
            resume_rx: Arc::new(Mutex::new(resume_rx)),
        }));

        let model_name = selection.primary.reference();

        let agent = ReActAgent::new(agent_name, &system_prompt, model, formatter)
            .with_toolkit(toolkit)
            .with_memory(Box::new(InMemoryWorkingMemory::new()))
            .with_max_iters(50)
            .with_model_name(model_name);

        let cancel_token = agent.cancel_token();

        // Wrap with HookedAgent + SseEmitterHook
        let mut hooks = HookRegistry::new();
        hooks.register_instance_hook(Box::new(SseEmitterHook {
            tx: sse_tx,
            agent_name: agent_name.to_string(),
            event_log: Some(Arc::clone(&state.persist.event_log)),
            session_id,
        }));
        let hooked = HookedAgent::new(agent, hooks);

        Ok((hooked, cancel_token))
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Resolve the routed model selection for an agent.
    /// Priority: agent manifest model > app llm_config > system default.
    async fn resolve_model_selection(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> Result<macaca_llm::ModelSelection, String> {
        let agent_model = state
            .kernel
            .get_agent_by_name(agent_name)
            .await
            .and_then(|manifest| (!manifest.model.is_empty()).then_some(manifest.model));

        let app_defaults = {
            let registry = state.registry.read().await;
            registry
                .get_app(app_id)
                .and_then(|app| app.manifest.llm_config.clone())
        };

        state
            .llm_router
            .resolve_selection(&macaca_llm::ModelSelectionRequest {
                agent_model,
                app_model: app_defaults.as_ref().map(|cfg| cfg.model.clone()),
                app_provider: app_defaults.as_ref().map(|cfg| cfg.provider.clone()),
                system_model: (!state.config.default_model.is_empty())
                    .then_some(state.config.default_model.clone()),
                ..Default::default()
            })
            .map_err(|e| e.to_string())
    }

    /// Load the agent's persona and build the system prompt.
    async fn build_system_prompt(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
    ) -> String {
        let app_dir = {
            let dirs = state.config.app_dirs.read().await;
            dirs.iter()
                .find(|(id, _)| **id == *app_id)
                .map(|(_, path)| path.clone())
        };

        let persona = if let Some(ref dir) = app_dir {
            let persona_dir = dir.join("personas").join(agent_name);
            if persona_dir.exists() {
                AgentPersona::load_from_directory(&persona_dir).await.ok()
            } else {
                None
            }
        } else {
            None
        };

        let mut prompt = if let Some(ref p) = persona {
            p.to_system_prompt(None)
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };

        // Inject capabilities
        let manifests = state.kernel.list_agents().await;
        if let Some(info) = manifests.iter().find(|m| m.name == agent_name) {
            let caps: Vec<&str> = info.capabilities.iter().map(|c| c.name.as_str()).collect();
            if !caps.is_empty() {
                prompt.push_str(&format!("\n\nYour capabilities: {}", caps.join(", ")));
            }
        }

        // Inject workspace paths
        {
            let workspaces = state.config.app_workspaces.read().await;
            if let Some(ws) = workspaces.get(app_id) {
                prompt.push_str(&format!(
                    "\n\n## Workspace Paths\n\
                     - Workspace root (default cwd for file/shell tools): {}\n\
                     - Shared workspace: {}\n\
                     - Your private workspace: {}\n\
                     Relative paths are resolved from the workspace root above. \
                     Create project files in the shared workspace. \
                     Use your private workspace for temporary/scratch files only.",
                    ws.root.display(),
                    ws.shared.display(),
                    ws.agent_workspace(agent_name).display(),
                ));
            }
        }

        prompt
    }
}

fn truncate_tool_output(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...[truncated, {} bytes]", &text[..end], text.len())
}

const TOOL_TRACE_OUTPUT_MAX_BYTES: usize = 2000;

fn tool_response_text(response: &ToolResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            macaca_framework::message::ContentBlock::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn tool_trace_output(response: &ToolResponse) -> String {
    truncate_tool_output(&tool_response_text(response), TOOL_TRACE_OUTPUT_MAX_BYTES)
}

fn tool_call_event(name: &str, args: &serde_json::Value) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolCall {
        tool_name: name.to_string(),
        tool_input: args.clone(),
        call_id: None,
    }
}

fn tool_result_event(name: &str, output: String) -> macaca_proto::AgentExecutionEvent {
    macaca_proto::AgentExecutionEvent::ToolResult {
        tool_name: name.to_string(),
        output,
        is_error: None,
    }
}

// ---------------------------------------------------------------------------
// SseEmitterHook — bridges ReActAgent lifecycle to SSE
// ---------------------------------------------------------------------------

/// Hook that emits SSE events at the start and end of a `reply` call.
pub struct SseEmitterHook {
    tx: mpsc::Sender<Result<Event, Infallible>>,
    agent_name: String,
    event_log: Option<Arc<EventLog>>,
    session_id: Option<String>,
}

#[async_trait]
impl Hook for SseEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "thinking",
                    "coordinator",
                    serde_json::json!({
                        "iteration": 0,
                    }),
                )
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

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "content",
                    "coordinator",
                    serde_json::json!({
                        "content": text,
                    }),
                )
                .await;
            event_log
                .append(
                    session_id,
                    "done",
                    "coordinator",
                    serde_json::json!({
                        "model": "",
                        "tokens": { "prompt": 0, "completion": 0, "total": 0 },
                        "iterations": 0,
                        "tools_used": [],
                    }),
                )
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
    tx: mpsc::Sender<Result<Event, Infallible>>,
    agent_name: String,
    event_log: Option<Arc<EventLog>>,
    session_id: Option<String>,
}

#[async_trait]
impl ToolMiddleware for SseToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        if let (Some(event_log), Some(session_id)) = (&self.event_log, &self.session_id) {
            event_log
                .append(
                    session_id,
                    "tool_call",
                    "coordinator",
                    serde_json::json!({
                        "tool_name": name,
                        "tool_input": args.clone(),
                    }),
                )
                .await;
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
                .append(
                    session_id,
                    "tool_result",
                    "coordinator",
                    serde_json::json!({
                        "tool_name": name,
                        "output": display_result,
                    }),
                )
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

// ---------------------------------------------------------------------------
// ChannelEmitterHook — bridges ReActAgent lifecycle to AgentExecutionEvent
// ---------------------------------------------------------------------------

pub struct ChannelEmitterHook {
    tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
    iteration: std::sync::atomic::AtomicUsize,
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
    tx: mpsc::Sender<macaca_proto::AgentExecutionEvent>,
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

// ---------------------------------------------------------------------------
// ExecutorEmitterHook — bridges ReActAgent lifecycle to executor broadcast
// ---------------------------------------------------------------------------

/// Hook that emits executor events at the start and end of a `reply` call.
/// Used by worker agents to push thinking/assistant events to SSE + EventLog.
pub struct ExecutorEmitterHook {
    executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    task_id: macaca_proto::TaskId,
    agent_name: String,
    iteration: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl Hook for ExecutorEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let iter = self.iteration.fetch_add(1, Ordering::Relaxed);
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: macaca_proto::AgentExecutionEvent::Thinking {
                    iteration: iter,
                    content: None,
                },
            });
        Ok(msg)
    }

    async fn post_reply(&self, msg: Msg) -> macaca_framework::agent::AgentResult<Msg> {
        let text = msg.get_text();
        if !text.is_empty() {
            self.executor
                .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                    task_id: self.task_id,
                    agent: self.agent_name.clone(),
                    event: macaca_proto::AgentExecutionEvent::Assistant { content: text },
                });
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
    executor: Arc<macaca_kernel::executor::ApplicationExecutor>,
    task_id: macaca_proto::TaskId,
    agent_name: String,
}

#[async_trait]
impl ToolMiddleware for ExecutorToolMiddleware {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<(), ToolError> {
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_call_event(name, args),
            });
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        self.executor
            .broadcast_event(macaca_kernel::executor::ExecutorEvent::AgentEvent {
                task_id: self.task_id,
                agent: self.agent_name.clone(),
                event: tool_result_event(name, tool_trace_output(response)),
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{tool_response_text, truncate_tool_output};
    use macaca_framework::message::{ContentBlock, TextBlock};
    use macaca_framework::tool::ToolResponse;

    #[test]
    fn truncate_tool_output_respects_utf8_boundaries() {
        let text = "─".repeat(800);

        let truncated = truncate_tool_output(&text, 2000);

        assert!(truncated.ends_with("[truncated, 2400 bytes]"));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn truncate_tool_output_keeps_short_text_unchanged() {
        let text = "北京 weather";

        assert_eq!(truncate_tool_output(text, 2000), text);
    }

    #[test]
    fn tool_response_text_joins_multiple_text_blocks() {
        let response = ToolResponse {
            content: vec![
                ContentBlock::Text(TextBlock {
                    text: "hello".into(),
                }),
                ContentBlock::Text(TextBlock {
                    text: " world".into(),
                }),
            ],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "hello world");
    }

    #[test]
    fn tool_response_text_returns_empty_string_for_empty_response() {
        let response = ToolResponse {
            content: Vec::new(),
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        };

        assert_eq!(tool_response_text(&response), "");
    }
}

// ---------------------------------------------------------------------------
// PauseOnGoalMiddleware — pauses coordinator when create_goal is called
// ---------------------------------------------------------------------------

/// Middleware that blocks after `create_goal` tool execution until the goal
/// completes (via `resume_rx`). This replaces the `PausableAgenticLoop`'s
/// external pause signal mechanism with a tool-level block.
pub struct PauseOnGoalMiddleware {
    pause_signal: Arc<AtomicBool>,
    resume_rx: Arc<Mutex<mpsc::Receiver<ResumeReason>>>,
}

#[async_trait]
impl ToolMiddleware for PauseOnGoalMiddleware {
    async fn before(&self, _name: &str, _args: &mut serde_json::Value) -> Result<(), ToolError> {
        Ok(())
    }

    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        if name != "create_goal" {
            return Ok(());
        }

        tracing::info!("PauseOnGoalMiddleware: create_goal detected, pausing coordinator");
        self.pause_signal.store(true, Ordering::SeqCst);

        // Wait for the goal to complete (GoalCompleted sends resume signal).
        // Autonomous goals can legitimately run longer than a fixed HTTP-era
        // timeout; ending this wait early loses the paused coordinator.
        let mut rx = self.resume_rx.lock().await;
        match rx.recv().await {
            Some(reason) => {
                self.pause_signal.store(false, Ordering::SeqCst);
                let context = match &reason {
                    ResumeReason::DelegateCompleted { output, .. } => output.clone(),
                    _ => "Goal processing completed.".to_string(),
                };
                response
                    .content
                    .push(macaca_framework::message::ContentBlock::Text(
                        macaca_framework::message::TextBlock {
                            text: format!("\n\n[Goal completed: {}]", context),
                        },
                    ));
                tracing::info!("PauseOnGoalMiddleware: resumed after goal completion");
            }
            None => {
                self.pause_signal.store(false, Ordering::SeqCst);
                tracing::warn!("PauseOnGoalMiddleware: resume channel closed");
            }
        }
        Ok(())
    }
}
