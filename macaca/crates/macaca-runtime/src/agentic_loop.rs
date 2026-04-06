//! Core agentic loop — the "heartbeat" of Agent OS agents.
//!
//! The loop sends messages to an LLM, checks for tool calls in the response,
//! executes the requested tools, feeds results back to the LLM, and repeats
//! until the LLM produces a final (non-tool-call) response or a safety limit is hit.

use std::sync::Arc;
use std::time::Duration;

use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, MacacaError, MacacaResult, LlmMessage, LlmOptions, Permission,
    TokenUsage, ToolCall,
};
use macaca_tools::{ToolSet, TraceEvent};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

use crate::context_window::{ContextWindowConfig, ContextWindowManager};
use crate::loop_detector::{LoopDetector, LoopDetectorAction, LoopDetectorConfig};
use crate::permission::PermissionChecker;

/// Configuration for the agentic runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of LLM round-trips before forcing a stop.
    pub max_iterations: usize,
    /// Timeout for a single tool execution.
    pub tool_timeout: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            tool_timeout: Duration::from_secs(60),
        }
    }
}

/// The result of an agentic loop execution.
#[derive(Debug, Clone)]
pub struct LoopResult {
    /// The final text response from the LLM.
    pub content: String,
    /// Total token usage across all iterations.
    pub total_usage: TokenUsage,
    /// Number of LLM round-trips performed.
    pub iterations: usize,
    /// The complete conversation history.
    pub messages: Vec<LlmMessage>,
}

/// The agentic execution loop.
///
/// Drives the LLM → tool → LLM cycle until the model produces a final
/// response or the iteration limit is reached.
pub struct AgenticLoop {
    config: RuntimeConfig,
}

impl AgenticLoop {
    pub fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }

    /// Run the agentic loop.
    ///
    /// # Arguments
    /// - `agent_id` — identity of the running agent (for permission checks)
    /// - `llm` — the LLM provider to call
    /// - `tools` — the tool set available to the agent
    /// - `initial_messages` — the starting conversation (system + user messages)
    /// - `options` — LLM options (model, temperature, etc.)
    /// - `permission` — the agent's permission policy
    /// - `permission_checker` — optional checker; if `None`, all tools are allowed
    #[instrument(name = "agentic_loop", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn run(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolSet,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> MacacaResult<LoopResult> {
        let mut messages = initial_messages;
        let mut total_usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        let mut iterations = 0;
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());

        // Build LlmOptions with tool definitions injected.
        let options_with_tools = {
            let mut opts = options.clone();
            let defs = tools.to_definitions();
            if !defs.is_empty() {
                opts.tools = Some(defs);
            }
            opts
        };

        loop {
            iterations += 1;

            if iterations > self.config.max_iterations {
                warn!(
                    iterations,
                    max = self.config.max_iterations,
                    "Agentic loop hit max iterations — forcing stop"
                );
                // Return accumulated content from the last assistant message.
                let last_content = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                    .map(|m| m.content.clone())
                    .unwrap_or_default();

                return Ok(LoopResult {
                    content: last_content,
                    total_usage,
                    iterations: iterations - 1,
                    messages,
                });
            }

            debug!(iteration = iterations, "Sending request to LLM");

            // 1. Call LLM (trim context window if needed; internal history stays intact)
            let trimmed = ctx_manager.trim_if_needed(messages.clone());
            let response = llm.chat(trimmed, &options_with_tools).await?;
            accumulate_usage(&mut total_usage, &response.usage);

            // 2. Check for tool calls
            let tool_calls = response.tool_calls.clone().unwrap_or_default();

            // Append the assistant message (with or without tool_calls)
            if tool_calls.is_empty() {
                messages.push(LlmMessage::assistant(response.content.clone()));
            } else {
                messages.push(LlmMessage::assistant_with_tool_calls(
                    response.content.clone(),
                    tool_calls.clone(),
                ));
            }

            if tool_calls.is_empty() {
                // No tool calls — the model produced a final response.
                debug!(iteration = iterations, "LLM returned final response");
                return Ok(LoopResult {
                    content: response.content,
                    total_usage,
                    iterations,
                    messages,
                });
            }

            // 3. Execute each tool call
            debug!(
                iteration = iterations,
                count = tool_calls.len(),
                "Executing tool calls"
            );

            for tc in &tool_calls {
                let args_str = serde_json::to_string(&tc.arguments)
                    .unwrap_or_else(|_| tc.arguments.to_string());

                match loop_detector.record_tool_call(&tc.name, &args_str) {
                    LoopDetectorAction::Continue => {}
                    LoopDetectorAction::Warn(msg) => {
                        warn!(msg = %msg, "Loop detector warning");
                        messages.push(LlmMessage::system(&msg));
                    }
                    LoopDetectorAction::Terminate(msg) => {
                        error!(msg = %msg, "Loop detector terminated loop");
                        let last_content = messages
                            .iter()
                            .rev()
                            .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                            .map(|m| m.content.clone())
                            .unwrap_or_default();
                        return Ok(LoopResult {
                            content: last_content,
                            total_usage,
                            iterations,
                            messages,
                        });
                    }
                }

                let tool_result = self
                    .execute_tool_call(agent_id, tools, tc, permission, permission_checker)
                    .await;

                let result_value = match tool_result {
                    Ok(value) => serde_json::to_string(&value)
                        .unwrap_or_else(|_| value.to_string()),
                    Err(e) => format!("Error: {e}"),
                };

                // Append tool result message
                messages.push(LlmMessage::tool_result(&tc.id, result_value));
            }
        }
    }

    /// Run the agentic loop with event callbacks for progress tracking.
    ///
    /// This is similar to `run` but sends events to the provided channel during execution.
    #[instrument(name = "agentic_loop_with_events", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn run_with_events(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolSet,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        let mut messages = initial_messages;
        let mut iterations = 0;
        let mut total_usage = TokenUsage::default();
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());
        let options_with_tools = LlmOptions {
            tools: Some(tools.to_definitions()),
            ..options.clone()
        };

        info!(max_iterations = self.config.max_iterations, "Starting agentic loop with events");

        loop {
            iterations += 1;
            if iterations > self.config.max_iterations {
                warn!(iterations, "Iteration limit reached");
                break;
            }

            // Send thinking event
            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentExecutionEvent::thinking(iterations)).await;
            }

            debug!(iteration = iterations, "Sending request to LLM");

            // Call LLM (trim context window if needed; internal history stays intact)
            let trimmed = ctx_manager.trim_if_needed(messages.clone());
            let response = llm.chat(trimmed, &options_with_tools).await?;
            accumulate_usage(&mut total_usage, &response.usage);

            // Send assistant event if there's content
            if !response.content.is_empty() {
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::assistant(response.content.clone())).await;
                }
            }

            // Check for tool calls
            let tool_calls = response.tool_calls.clone().unwrap_or_default();

            // Append the assistant message
            if tool_calls.is_empty() {
                messages.push(LlmMessage::assistant(response.content.clone()));
            } else {
                messages.push(LlmMessage::assistant_with_tool_calls(
                    response.content.clone(),
                    tool_calls.clone(),
                ));
            }

            if tool_calls.is_empty() {
                // No tool calls — final response
                debug!(iteration = iterations, "LLM returned final response");

                // Send completed event
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
                }

                return Ok(LoopResult {
                    content: response.content,
                    total_usage,
                    iterations,
                    messages,
                });
            }

            // Execute each tool call
            debug!(iteration = iterations, count = tool_calls.len(), "Executing tool calls");

            for tc in &tool_calls {
                let args_str = serde_json::to_string(&tc.arguments)
                    .unwrap_or_else(|_| tc.arguments.to_string());

                match loop_detector.record_tool_call(&tc.name, &args_str) {
                    LoopDetectorAction::Continue => {}
                    LoopDetectorAction::Warn(msg) => {
                        warn!(msg = %msg, "Loop detector warning");
                        messages.push(LlmMessage::system(&msg));
                    }
                    LoopDetectorAction::Terminate(msg) => {
                        error!(msg = %msg, "Loop detector terminated loop");
                        let last_content = messages
                            .iter()
                            .rev()
                            .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                            .map(|m| m.content.clone())
                            .unwrap_or_default();
                        if let Some(ref tx) = event_tx {
                            let _ = tx.send(AgentExecutionEvent::completed(false, Some(msg))).await;
                        }
                        return Ok(LoopResult {
                            content: last_content,
                            total_usage,
                            iterations,
                            messages,
                        });
                    }
                }

                // Send tool call event
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::tool_call_with_id(
                        tc.name.clone(),
                        tc.arguments.clone(),
                        tc.id.clone(),
                    )).await;
                }

                let tool_result = self
                    .execute_tool_call_with_events(agent_id, tools, tc, permission, permission_checker, event_tx.as_ref())
                    .await;

                let (result_value, is_error) = match tool_result {
                    Ok(value) => {
                        let s = serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
                        (s, false)
                    }
                    Err(e) => (format!("Error: {e}"), true),
                };

                // Send tool result event
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::tool_result_with_error(
                        tc.name.clone(),
                        result_value.clone(),
                        is_error,
                    )).await;
                }

                // Append tool result message
                messages.push(LlmMessage::tool_result(&tc.id, result_value));
            }
        }

        // Iteration limit reached - extract last assistant content
        let last_content = messages
            .iter()
            .rev()
            .find(|m| m.role == macaca_proto::LlmRole::Assistant)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        // Send completed event
        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
        }

        Ok(LoopResult {
            content: last_content,
            total_usage,
            iterations: iterations - 1,
            messages,
        })
    }

    /// Execute a single tool call with streaming events support.
    /// This method creates a channel to receive TraceEvents from tools like claude_code_execute
    /// and forwards them as AgentExecutionEvent::CcTrace to the event_tx.
    async fn execute_tool_call_with_events(
        &self,
        agent_id: &AgentId,
        tools: &dyn ToolSet,
        tool_call: &ToolCall,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<&mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<serde_json::Value> {
        let tool_name = &tool_call.name;
        let args_preview = serde_json::to_string(&tool_call.arguments)
            .map(|s| s.chars().take(200).collect::<String>())
            .unwrap_or_else(|_| "<serialize error>".to_string());

        info!(
            agent_id = %agent_id,
            tool_name = %tool_name,
            args_preview = %args_preview,
            "[TOOL] Executing tool"
        );

        // Permission check (includes path and network access enforcement)
        if let Some(checker) = permission_checker {
            checker.check_tool_with_args(agent_id, permission, &tool_call.name, &tool_call.arguments)?;
        }

        // Find tool
        let tool = tools.get_tool(&tool_call.name).ok_or_else(|| {
            error!(
                agent_id = %agent_id,
                tool_name = %tool_name,
                "[TOOL] Tool not found"
            );
            MacacaError::NotFound(format!("Tool '{}' not found", tool_call.name))
        })?;

        // Create channel for trace events if we have an event sender
        let (trace_tx, mut trace_rx) = if event_tx.is_some() {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<TraceEvent>();
            (Some(tx), rx)
        } else {
            let (_, rx) = tokio::sync::mpsc::unbounded_channel::<TraceEvent>();
            (None, rx)
        };

        // Execute with timeout, streaming events if available
        let tool_name = tool_call.name.clone();
        let timeout_duration = self.config.tool_timeout;

        // Spawn a task to forward trace events in real-time while tool executes
        let forward_task = if let Some(tx) = event_tx.cloned() {
            Some(tokio::spawn(async move {
                while let Some(trace) = trace_rx.recv().await {
                    let cc_trace = AgentExecutionEvent::CcTrace {
                        thinking: trace.thinking,
                        text: trace.text,
                        tool_name: trace.tool_name,
                        tool_input: trace.tool_input,
                        tool_result: trace.tool_result,
                        is_error: trace.is_error,
                    };
                    if tx.send(cc_trace).await.is_err() {
                        break;
                    }
                }
            }))
        } else {
            None
        };

        let result = tokio::time::timeout(
            timeout_duration,
            tool.execute_streaming(tool_call.arguments.clone(), trace_tx),
        )
        .await
        .map_err(|_| {
            error!(
                agent_id = %agent_id,
                tool_name = %tool_name,
                timeout_secs = timeout_duration.as_secs(),
                "[TOOL] Tool execution timed out"
            );
            MacacaError::Timeout(format!(
                "Tool '{}' timed out after {}s",
                tool_name,
                timeout_duration.as_secs()
            ))
        })??;

        // Wait for the forward task to finish processing all events
        // The task will exit when trace_tx is dropped (which happens when execute_streaming completes)
        // and trace_rx.recv() returns None
        if let Some(task) = forward_task {
            let _ = task.await;
        }

        let result_preview = serde_json::to_string(&result)
            .map(|s| s.chars().take(200).collect::<String>())
            .unwrap_or_else(|_| "<serialize error>".to_string());

        info!(
            agent_id = %agent_id,
            tool_name = %tool_name,
            result_preview = %result_preview,
            "[TOOL] Tool execution completed"
        );

        Ok(result)
    }

    /// Execute a single tool call with permission checking and timeout.
    async fn execute_tool_call(
        &self,
        agent_id: &AgentId,
        tools: &dyn ToolSet,
        tool_call: &ToolCall,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> MacacaResult<serde_json::Value> {
        // Permission check (includes path and network access enforcement)
        if let Some(checker) = permission_checker {
            checker.check_tool_with_args(agent_id, permission, &tool_call.name, &tool_call.arguments)?;
        }

        // Find tool
        let tool = tools.get_tool(&tool_call.name).ok_or_else(|| {
            MacacaError::NotFound(format!("Tool '{}' not found", tool_call.name))
        })?;

        // Execute with timeout
        let result = tokio::time::timeout(
            self.config.tool_timeout,
            tool.execute(tool_call.arguments.clone()),
        )
        .await
        .map_err(|_| {
            MacacaError::Timeout(format!(
                "Tool '{}' timed out after {}s",
                tool_call.name,
                self.config.tool_timeout.as_secs()
            ))
        })??;

        Ok(result)
    }
}

impl Default for AgenticLoop {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

/// Accumulate token usage across iterations.
fn accumulate_usage(total: &mut TokenUsage, delta: &TokenUsage) {
    total.prompt_tokens += delta.prompt_tokens;
    total.completion_tokens += delta.completion_tokens;
    total.total_tokens += delta.total_tokens;
}

// ── Pausable Agentic Loop ────────────────────────────────────────────────────────

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc as StdArc;

/// Pausable wrapper around AgenticLoop that supports suspend/resume.
///
/// This enables the Fork-Join workflow where a child agent can suspend
/// while waiting for a delegated task to complete, and resume when
/// the task is done via a hook callback.
pub struct PausableAgenticLoop {
    /// The underlying agentic loop.
    inner: AgenticLoop,
    /// Signal to pause execution.
    pause_signal: StdArc<AtomicBool>,
    /// Resume reason received from hook callback.
    resume_reason: StdArc<tokio::sync::RwLock<Option<ResumeReason>>>,
}

/// Reason for resuming a paused loop.
#[derive(Debug, Clone)]
pub enum ResumeReason {
    /// Normal resume request.
    Manual,
    /// Resume due to delegate task completion.
    DelegateCompleted {
        task_id: String,
        success: bool,
        output: String,
    },
    /// Resume due to delegate task failure.
    DelegateFailed {
        task_id: String,
        error: String,
    },
    /// Resume due to timeout.
    Timeout,
}

impl PausableAgenticLoop {
    /// Create a new pausable agentic loop.
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            inner: AgenticLoop::new(config),
            pause_signal: StdArc::new(AtomicBool::new(false)),
            resume_reason: StdArc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Get a clone of the pause signal for external control.
    pub fn pause_signal(&self) -> StdArc<AtomicBool> {
        StdArc::clone(&self.pause_signal)
    }

    /// Request pause of the loop.
    pub fn request_pause(&self) {
        self.pause_signal.store(true, Ordering::SeqCst);
    }

    /// Resume the loop with a reason.
    pub async fn resume(&self, reason: ResumeReason) {
        self.pause_signal.store(false, Ordering::SeqCst);
        let mut r = self.resume_reason.write().await;
        *r = Some(reason);
    }

    /// Check if paused and consume the resume reason if any.
    pub async fn check_and_consume_resume(&self) -> Option<ResumeReason> {
        if !self.pause_signal.load(Ordering::SeqCst) {
            let mut r = self.resume_reason.write().await;
            return r.take();
        }
        None
    }

    /// Run with pause support.
    ///
    /// The loop will check for pause signals at the start of each iteration.
    /// When paused, it will wait until resumed via `resume()`.
    pub async fn run_with_pause(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolSet,
        mut messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        let options_with_tools = LlmOptions {
            tools: Some(tools.to_definitions()),
            ..options.clone()
        };

        let mut total_usage = TokenUsage::default();
        let mut iterations = 0;
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());

        loop {
            // Check if paused and wait for resume
            if self.pause_signal.load(Ordering::SeqCst) {
                info!(
                    agent_id = %agent_id,
                    iteration = iterations,
                    "[PAUSE] Loop paused, waiting for resume signal"
                );
            }
            let mut pause_logged = false;
            while self.pause_signal.load(Ordering::SeqCst) {
                if !pause_logged {
                    pause_logged = true;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            if pause_logged {
                info!(
                    agent_id = %agent_id,
                    iteration = iterations,
                    "[PAUSE] Loop resumed"
                );
            }

            // Check for resume reason and inject it
            if let Some(reason) = self.check_and_consume_resume().await {
                let resume_msg = match reason {
                    ResumeReason::DelegateCompleted { task_id, success, output } => {
                        info!(
                            agent_id = %agent_id,
                            task_id = %task_id,
                            success = success,
                            output_len = output.len(),
                            "[RESUME] Delegate task completed"
                        );
                        format!("[Delegate Task {} Completed]\nSuccess: {}\nOutput: {}", task_id, success, output)
                    }
                    ResumeReason::DelegateFailed { task_id, error } => {
                        warn!(
                            agent_id = %agent_id,
                            task_id = %task_id,
                            error = %error,
                            "[RESUME] Delegate task failed"
                        );
                        format!("[Delegate Task {} Failed]\nError: {}", task_id, error)
                    }
                    ResumeReason::Timeout => {
                        warn!(
                            agent_id = %agent_id,
                            "[RESUME] Delegate task timed out"
                        );
                        "[Delegate Task Timed Out]".to_string()
                    }
                    ResumeReason::Manual => {
                        info!(
                            agent_id = %agent_id,
                            "[RESUME] Manual resume requested"
                        );
                        "[Resume Requested]".to_string()
                    }
                };
                messages.push(LlmMessage::user(resume_msg));
            }

            iterations += 1;

            if iterations > self.inner.config.max_iterations {
                warn!(iterations, "Iteration limit reached");
                break;
            }

            // Send thinking event
            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentExecutionEvent::thinking(iterations)).await;
            }

            // Call LLM (trim context window if needed; internal history stays intact)
            let trimmed = ctx_manager.trim_if_needed(messages.clone());
            let response = llm.chat(trimmed, &options_with_tools).await?;
            accumulate_usage(&mut total_usage, &response.usage);

            // Send assistant event
            if !response.content.is_empty() {
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::assistant(response.content.clone())).await;
                }
            }

            // Check for tool calls
            let tool_calls = response.tool_calls.clone().unwrap_or_default();

            // Append the assistant message
            if tool_calls.is_empty() {
                messages.push(LlmMessage::assistant(response.content.clone()));
            } else {
                messages.push(LlmMessage::assistant_with_tool_calls(
                    response.content.clone(),
                    tool_calls.clone(),
                ));
            }

            if tool_calls.is_empty() {
                // No tool calls - final response
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
                }

                return Ok(LoopResult {
                    content: response.content,
                    total_usage,
                    iterations,
                    messages,
                });
            }

            // Execute tool calls
            for tc in &tool_calls {
                let args_str = serde_json::to_string(&tc.arguments)
                    .unwrap_or_else(|_| tc.arguments.to_string());

                match loop_detector.record_tool_call(&tc.name, &args_str) {
                    LoopDetectorAction::Continue => {}
                    LoopDetectorAction::Warn(msg) => {
                        warn!(msg = %msg, "Loop detector warning");
                        messages.push(LlmMessage::system(&msg));
                    }
                    LoopDetectorAction::Terminate(msg) => {
                        error!(msg = %msg, "Loop detector terminated loop");
                        let last_content = messages
                            .iter()
                            .rev()
                            .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                            .map(|m| m.content.clone())
                            .unwrap_or_default();
                        if let Some(ref tx) = event_tx {
                            let _ = tx.send(AgentExecutionEvent::completed(false, Some(msg))).await;
                        }
                        return Ok(LoopResult {
                            content: last_content,
                            total_usage,
                            iterations,
                            messages,
                        });
                    }
                }

                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentExecutionEvent::tool_call_with_id(
                        tc.name.clone(),
                        tc.arguments.clone(),
                        tc.id.clone(),
                    )).await;
                }

                let tool_result = self.inner.execute_tool_call(
                    agent_id, tools, tc, permission, permission_checker,
                ).await;

                let result_value = match tool_result {
                    Ok(value) => serde_json::to_string(&value)
                        .unwrap_or_else(|_| value.to_string()),
                    Err(e) => format!("Error: {e}"),
                };

                if let Some(ref tx) = event_tx {
                    let is_error = result_value.contains("error") || result_value.contains("Error") || result_value.contains("failed");
                    let _ = tx.send(AgentExecutionEvent::tool_result_with_error(
                        tc.name.clone(),
                        result_value.clone(),
                        is_error,
                    )).await;
                }

                messages.push(LlmMessage::tool_result(&tc.id, result_value));
            }
        }

        // Return last assistant content
        let last_content = messages
            .iter()
            .rev()
            .find(|m| m.role == macaca_proto::LlmRole::Assistant)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
        }

        Ok(LoopResult {
            content: last_content,
            total_usage,
            iterations: iterations - 1,
            messages,
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::LlmResponse;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // ── Mock LLM: returns tool calls on first call, then a final response ──

    struct MockToolLlm {
        call_count: Arc<AtomicUsize>,
    }

    impl MockToolLlm {
        fn new() -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockToolLlm {
        fn name(&self) -> &str {
            "mock-tool-llm"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst);

            if n == 0 {
                // First call: request a tool call
                Ok(LlmResponse {
                    content: String::new(),
                    model: "mock".into(),
                    usage: TokenUsage {
                        prompt_tokens: 10,
                        completion_tokens: 5,
                        total_tokens: 15,
                    },
                    finish_reason: "tool_calls".into(),
                    tool_calls: Some(vec![ToolCall {
                        id: "call_1".into(),
                        name: "shell".into(),
                        arguments: serde_json::json!({ "command": "echo hello" }),
                    }]),
                })
            } else {
                // Second call: final response
                Ok(LlmResponse {
                    content: "Done! The command output was: hello".into(),
                    model: "mock".into(),
                    usage: TokenUsage {
                        prompt_tokens: 20,
                        completion_tokens: 10,
                        total_tokens: 30,
                    },
                    finish_reason: "stop".into(),
                    tool_calls: None,
                })
            }
        }
    }

    // ── Mock LLM: always requests tool calls (for max iteration test) ──

    struct InfiniteToolLlm;

    #[async_trait]
    impl LlmProvider for InfiniteToolLlm {
        fn name(&self) -> &str {
            "infinite-tool-llm"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "thinking...".into(),
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 5,
                    completion_tokens: 5,
                    total_tokens: 10,
                },
                finish_reason: "tool_calls".into(),
                tool_calls: Some(vec![ToolCall {
                    id: "call_loop".into(),
                    name: "shell".into(),
                    arguments: serde_json::json!({ "command": "echo loop" }),
                }]),
            })
        }
    }

    // ── Mock LLM: no tool calls at all ──

    struct DirectResponseLlm;

    #[async_trait]
    impl LlmProvider for DirectResponseLlm {
        fn name(&self) -> &str {
            "direct-response-llm"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "Direct answer: 42".into(),
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    fn default_permission() -> Permission {
        Permission {
            level: macaca_proto::PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: true,
        }
    }

    #[tokio::test]
    async fn direct_response_no_tool_calls() {
        let runner = AgenticLoop::default();
        let llm = DirectResponseLlm;
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![
            LlmMessage::system("You are helpful."),
            LlmMessage::user("What is 6*7?"),
        ];

        let result = runner
            .run(
                &agent_id,
                &llm,
                &tools,
                messages,
                &LlmOptions::default(),
                &default_permission(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.content, "Direct answer: 42");
        assert_eq!(result.iterations, 1);
        assert_eq!(result.total_usage.total_tokens, 15);
    }

    #[tokio::test]
    async fn tool_call_then_final_response() {
        let runner = AgenticLoop::new(RuntimeConfig {
            max_iterations: 10,
            tool_timeout: Duration::from_secs(5),
        });
        let llm = MockToolLlm::new();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![
            LlmMessage::system("You are helpful."),
            LlmMessage::user("Run echo hello"),
        ];

        let result = runner
            .run(
                &agent_id,
                &llm,
                &tools,
                messages,
                &LlmOptions::default(),
                &default_permission(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.content, "Done! The command output was: hello");
        assert_eq!(result.iterations, 2);
        assert_eq!(result.total_usage.total_tokens, 45); // 15 + 30
        // Messages should contain: system, user, assistant(tool_call), tool_result, assistant(final)
        assert_eq!(result.messages.len(), 5);
    }

    #[tokio::test]
    async fn max_iterations_stops_loop() {
        let runner = AgenticLoop::new(RuntimeConfig {
            max_iterations: 3,
            tool_timeout: Duration::from_secs(5),
        });
        let llm = InfiniteToolLlm;
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![LlmMessage::user("Loop forever")];

        let result = runner
            .run(
                &agent_id,
                &llm,
                &tools,
                messages,
                &LlmOptions::default(),
                &default_permission(),
                None,
            )
            .await
            .unwrap();

        // Should stop at max_iterations
        assert_eq!(result.iterations, 3);
        assert_eq!(result.content, "thinking...");
    }

    #[tokio::test]
    async fn permission_denied_returns_error_in_tool_result() {
        use crate::permission::DefaultPermissionChecker;

        let runner = AgenticLoop::new(RuntimeConfig {
            max_iterations: 5,
            tool_timeout: Duration::from_secs(5),
        });
        let llm = MockToolLlm::new();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![LlmMessage::user("Run shell")];

        // Only allow file_read — shell should be denied
        let permission = Permission {
            level: macaca_proto::PermissionLevel::User,
            allowed_tools: vec!["file_read".into()],
            allowed_paths: vec![],
            network_access: false,
        };
        let checker = DefaultPermissionChecker;

        let result = runner
            .run(
                &agent_id,
                &llm,
                &tools,
                messages,
                &LlmOptions::default(),
                &permission,
                Some(&checker),
            )
            .await
            .unwrap();

        // The loop should have continued: permission error fed back as tool result,
        // then the LLM gave a final response on iteration 2.
        assert_eq!(result.iterations, 2);
        // Check that the tool result message contains the permission error
        let tool_msg = result
            .messages
            .iter()
            .find(|m| m.role == macaca_proto::LlmRole::Tool)
            .expect("should have a Tool role message");
        assert!(tool_msg.content.contains("not allowed"));
    }

    #[tokio::test]
    async fn tool_not_found_returns_error_in_tool_result() {
        // LLM requests a tool that doesn't exist
        struct UnknownToolLlm;

        #[async_trait]
        impl LlmProvider for UnknownToolLlm {
            fn name(&self) -> &str {
                "unknown-tool-llm"
            }

            async fn chat(
                &self,
                messages: Vec<LlmMessage>,
                _options: &LlmOptions,
            ) -> MacacaResult<LlmResponse> {
                // Only request unknown tool on first call
                let has_tool_result = messages
                    .iter()
                    .any(|m| m.role == macaca_proto::LlmRole::Tool);

                if !has_tool_result {
                    Ok(LlmResponse {
                        content: String::new(),
                        model: "mock".into(),
                        usage: TokenUsage::default(),
                        finish_reason: "tool_calls".into(),
                        tool_calls: Some(vec![ToolCall {
                            id: "call_x".into(),
                            name: "nonexistent_tool".into(),
                            arguments: serde_json::json!({}),
                        }]),
                    })
                } else {
                    Ok(LlmResponse {
                        content: "Tool not found, giving up.".into(),
                        model: "mock".into(),
                        usage: TokenUsage::default(),
                        finish_reason: "stop".into(),
                        tool_calls: None,
                    })
                }
            }
        }

        let runner = AgenticLoop::default();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();

        let result = runner
            .run(
                &agent_id,
                &UnknownToolLlm,
                &tools,
                vec![LlmMessage::user("Use nonexistent tool")],
                &LlmOptions::default(),
                &default_permission(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.iterations, 2);
        let tool_msg = result
            .messages
            .iter()
            .find(|m| m.role == macaca_proto::LlmRole::Tool)
            .unwrap();
        assert!(tool_msg.content.contains("not found"));
    }

    #[test]
    fn runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_iterations, 25);
        assert_eq!(config.tool_timeout, Duration::from_secs(60));
    }

    #[test]
    fn accumulate_usage_test() {
        let mut total = TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        };
        let delta = TokenUsage {
            prompt_tokens: 20,
            completion_tokens: 10,
            total_tokens: 30,
        };
        accumulate_usage(&mut total, &delta);
        assert_eq!(total.prompt_tokens, 30);
        assert_eq!(total.completion_tokens, 15);
        assert_eq!(total.total_tokens, 45);
    }
}
