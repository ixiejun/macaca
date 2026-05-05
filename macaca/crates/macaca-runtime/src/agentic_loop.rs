//! Core agentic loop — the "heartbeat" of Agent OS agents.
//!
//! The loop sends messages to an LLM, checks for tool calls in the response,
//! executes the requested tools, feeds results back to the LLM, and repeats
//! until the LLM produces a final (non-tool-call) response or a safety limit is hit.

use std::time::Duration;

use macaca_context::{
    ContextAssembleInput, ContextBudget, ContextEngineSelection, ContextRuntimeFacade,
};
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, MacacaResult, Permission, TokenUsage,
    ToolCall,
};
use macaca_tools::ToolCatalog;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

use crate::context_window::{ContextWindowConfig, ContextWindowManager};
use crate::events::RuntimeEventSink;
use crate::loop_detector::{LoopDetector, LoopDetectorAction, LoopDetectorConfig};
use crate::permission::PermissionChecker;
use crate::template::RuntimeIterationOutcome;

/// Configuration for the agentic runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of LLM round-trips before forcing a stop.
    pub max_iterations: usize,
    /// Timeout for a single tool execution.
    pub tool_timeout: Duration,
    /// Selected runtime context engine.
    pub context_engine: String,
    /// Fallback context engine if the selected engine fails.
    pub context_fallback_engine: String,
    /// Provider-neutral context budget.
    pub context_budget: ContextBudget,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            tool_timeout: Duration::from_secs(60),
            context_engine: "legacy".into(),
            context_fallback_engine: "legacy".into(),
            context_budget: ContextBudget::default(),
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

    /// Core single-iteration logic shared by all loop variants.
    ///
    /// Handles LLM call, tool execution, event emission, and loop detection.
    /// Returns `RuntimeIterationOutcome` so callers can decide whether to continue.
    #[allow(clippy::too_many_arguments)]
    async fn run_iteration(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        messages: &mut Vec<LlmMessage>,
        options_with_tools: &LlmOptions,
        total_usage: &mut TokenUsage,
        iteration: usize,
        loop_detector: &mut LoopDetector,
        ctx_manager: &ContextWindowManager,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<&mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<RuntimeIterationOutcome> {
        let event_sink = RuntimeEventSink::new(event_tx);
        event_sink.emit_thinking(iteration).await;

        debug!(iteration, "Sending request to LLM");

        // Call LLM (trim context window if needed; internal history stays intact)
        let trimmed = ctx_manager.trim_if_needed(messages.clone());
        let assembled = ContextRuntimeFacade::builtins(ContextEngineSelection {
            engine_id: self.config.context_engine.clone(),
            fallback_engine_id: self.config.context_fallback_engine.clone(),
        })
        .assemble(ContextAssembleInput {
            app_id: None,
            session_id: None,
            agent_name: agent_id.to_string(),
            model: options_with_tools.model.clone(),
            base_messages: trimmed,
            options: options_with_tools.clone(),
            budget: self.config.context_budget,
        })
        .await?;
        event_sink
            .emit(AgentExecutionEvent::DriverTrace {
                driver_name: "macaca-context".into(),
                trace: serde_json::json!({
                    "event_type": "context_report",
                    "engine_id": assembled.report.engine_id,
                    "request_id": assembled.report.request_id,
                    "estimated_total_tokens": assembled.report.estimated_total_tokens,
                    "token_budget": assembled.report.token_budget,
                    "pruned_tokens": assembled.report.pruned_tokens,
                    "source_count": assembled.report.sources.len(),
                    "decision_count": assembled.report.decisions.len(),
                }),
            })
            .await;
        tracing::debug!(
            engine = %assembled.report.engine_id,
            request_id = %assembled.report.request_id,
            estimated_tokens = assembled.report.estimated_total_tokens,
            "runtime_context_report"
        );
        let response = llm.chat(assembled.messages, &assembled.options).await?;
        accumulate_usage(total_usage, &response.usage);

        event_sink.emit_assistant(response.content.clone()).await;

        // Check for tool calls
        let tool_calls = response.tool_calls.clone().unwrap_or_default();

        // Append assistant message (with or without tool_calls)
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
            debug!(iteration, "LLM returned final response");
            return Ok(RuntimeIterationOutcome::FinalResponse {
                content: response.content,
            });
        }

        // Execute each tool call
        debug!(iteration, count = tool_calls.len(), "Executing tool calls");

        for tc in &tool_calls {
            let args_str =
                serde_json::to_string(&tc.arguments).unwrap_or_else(|_| tc.arguments.to_string());

            match loop_detector.record_tool_call(&tc.name, &args_str) {
                LoopDetectorAction::Continue => {}
                LoopDetectorAction::Warn(msg) => {
                    warn!(msg = %msg, "Loop detector warning");
                    messages.push(LlmMessage::system(&msg));
                }
                LoopDetectorAction::Terminate(msg) => {
                    error!(msg = %msg, "Loop detector terminated loop");
                    event_sink.emit_completed(false, Some(msg)).await;
                    let last_content = messages
                        .iter()
                        .rev()
                        .find(|m| m.role == macaca_proto::LlmRole::Assistant)
                        .map(|m| m.content.clone())
                        .unwrap_or_default();
                    // Signal callers to stop by returning a FinalResponse with the last content
                    return Ok(RuntimeIterationOutcome::FinalResponse {
                        content: last_content,
                    });
                }
            }

            // Emit tool_call event
            event_sink
                .emit(AgentExecutionEvent::tool_call_with_id(
                    tc.name.clone(),
                    tc.arguments.clone(),
                    tc.id.clone(),
                ))
                .await;

            let tool_result = self
                .execute_tool_command(
                    agent_id,
                    tools,
                    tc,
                    permission,
                    permission_checker,
                    &event_sink,
                )
                .await;

            let (result_value, is_error) = match tool_result {
                Ok(value) => {
                    let s = serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
                    (s, false)
                }
                Err(e) => (format!("Error: {e}"), true),
            };

            // Emit tool_result event
            event_sink
                .emit(AgentExecutionEvent::tool_result_with_error(
                    tc.name.clone(),
                    result_value.clone(),
                    is_error,
                ))
                .await;

            messages.push(LlmMessage::tool_result(&tc.id, result_value));
        }

        Ok(RuntimeIterationOutcome::ToolsExecuted)
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
    pub async fn execute(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
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
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());

        // Build LlmOptions with tool definitions injected.
        let options_with_tools = {
            let mut opts = options.clone();
            opts.tools = tool_definitions(tools);
            opts
        };

        let mut iterations = 0;
        loop {
            iterations += 1;

            if iterations > self.config.max_iterations {
                warn!(
                    iterations,
                    max = self.config.max_iterations,
                    "Agentic loop hit max iterations — forcing stop"
                );
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

            match self
                .run_iteration(
                    agent_id,
                    llm,
                    tools,
                    &mut messages,
                    &options_with_tools,
                    &mut total_usage,
                    iterations,
                    &mut loop_detector,
                    &ctx_manager,
                    permission,
                    permission_checker,
                    None,
                )
                .await?
            {
                RuntimeIterationOutcome::FinalResponse { content } => {
                    return Ok(LoopResult {
                        content,
                        total_usage,
                        iterations,
                        messages,
                    });
                }
                RuntimeIterationOutcome::ToolsExecuted => continue,
            }
        }
    }

    /// Run the agentic loop.
    ///
    /// Deprecated compatibility wrapper. Use [`AgenticLoop::execute`] for new code.
    #[deprecated(note = "use AgenticLoop::execute")]
    #[instrument(name = "agentic_loop_legacy", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn run(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
    ) -> MacacaResult<LoopResult> {
        self.execute(
            agent_id,
            llm,
            tools,
            initial_messages,
            options,
            permission,
            permission_checker,
        )
        .await
    }

    /// Run the agentic loop with event callbacks for progress tracking.
    ///
    /// This is similar to `run` but sends events to the provided channel during execution.
    #[instrument(name = "agentic_loop_with_events", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn execute_with_events(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        let mut messages = initial_messages;
        let mut total_usage = TokenUsage::default();
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());
        let options_with_tools = LlmOptions {
            tools: tool_definitions(tools),
            ..options.clone()
        };

        info!(
            max_iterations = self.config.max_iterations,
            "Starting agentic loop with events"
        );

        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > self.config.max_iterations {
                warn!(iterations, "Iteration limit reached");
                break;
            }

            match self
                .run_iteration(
                    agent_id,
                    llm,
                    tools,
                    &mut messages,
                    &options_with_tools,
                    &mut total_usage,
                    iterations,
                    &mut loop_detector,
                    &ctx_manager,
                    permission,
                    permission_checker,
                    event_tx.as_ref(),
                )
                .await?
            {
                RuntimeIterationOutcome::FinalResponse { content } => {
                    // Emit completed(true) on normal finish
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
                    }
                    return Ok(LoopResult {
                        content,
                        total_usage,
                        iterations,
                        messages,
                    });
                }
                RuntimeIterationOutcome::ToolsExecuted => continue,
            }
        }

        // Iteration limit reached — extract last assistant content
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

    /// Run the agentic loop with event callbacks for progress tracking.
    ///
    /// Deprecated compatibility wrapper. Use [`AgenticLoop::execute_with_events`] for new code.
    #[deprecated(note = "use AgenticLoop::execute_with_events")]
    #[instrument(name = "agentic_loop_with_events_legacy", skip_all, fields(agent_id = %agent_id, max_iter = self.config.max_iterations))]
    pub async fn run_with_events(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        initial_messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        self.execute_with_events(
            agent_id,
            llm,
            tools,
            initial_messages,
            options,
            permission,
            permission_checker,
            event_tx,
        )
        .await
    }

    /// Execute a single tool call through the runtime command boundary.
    async fn execute_tool_command(
        &self,
        agent_id: &AgentId,
        tools: &dyn ToolCatalog,
        tool_call: &ToolCall,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_sink: &RuntimeEventSink<'_>,
    ) -> MacacaResult<serde_json::Value> {
        crate::execution::execute_tool_call(
            agent_id,
            tools,
            tool_call,
            permission,
            permission_checker,
            self.config.tool_timeout,
            event_sink.sender(),
        )
        .await
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

fn tool_definitions(
    tools: &dyn macaca_tools::ToolCatalog,
) -> Option<Vec<macaca_proto::ToolDefinition>> {
    let defs = macaca_tools::ToolCatalog::definitions(tools);
    if defs.is_empty() {
        None
    } else {
        Some(defs)
    }
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
    /// Notifier to wake the loop when resumed (replaces 100ms polling).
    resume_notify: StdArc<tokio::sync::Notify>,
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
    DelegateFailed { task_id: String, error: String },
    /// Resume due to timeout.
    Timeout,
}

impl PausableAgenticLoop {
    /// Create a new pausable agentic loop.
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            inner: AgenticLoop::new(config),
            pause_signal: StdArc::new(AtomicBool::new(false)),
            resume_notify: StdArc::new(tokio::sync::Notify::new()),
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
        // Wake the waiting loop instead of relying on 100ms polling.
        self.resume_notify.notify_one();
    }

    /// Check if paused and consume the resume reason if any.
    pub async fn check_and_consume_resume(&self) -> Option<ResumeReason> {
        if !self.pause_signal.load(Ordering::SeqCst) {
            let mut r = self.resume_reason.write().await;
            return r.take();
        }
        None
    }

    /// Execute with pause support.
    ///
    /// The loop will check for pause signals at the start of each iteration.
    /// When paused, it will wait until resumed via `resume()`.
    pub async fn execute_with_pause(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        mut messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        let options_with_tools = LlmOptions {
            tools: tool_definitions(tools),
            ..options.clone()
        };

        let mut total_usage = TokenUsage::default();
        let mut iterations = 0;
        let mut loop_detector = LoopDetector::new(LoopDetectorConfig::default());
        let ctx_manager = ContextWindowManager::new(ContextWindowConfig::default());

        loop {
            // Check for pause and wait for resume using Notify (no polling).
            if self.pause_signal.load(Ordering::SeqCst) {
                info!(
                    agent_id = %agent_id,
                    iteration = iterations,
                    "[PAUSE] Loop paused, waiting for resume signal"
                );
                self.resume_notify.notified().await;
                info!(
                    agent_id = %agent_id,
                    iteration = iterations,
                    "[PAUSE] Loop resumed"
                );
            }

            // Inject resume reason as a user message if present.
            if let Some(reason) = self.check_and_consume_resume().await {
                let resume_msg = match reason {
                    ResumeReason::DelegateCompleted {
                        task_id,
                        success,
                        output,
                    } => {
                        info!(
                            agent_id = %agent_id,
                            task_id = %task_id,
                            success = success,
                            output_len = output.len(),
                            "[RESUME] Delegate task completed"
                        );
                        format!(
                            "[Delegate Task {} Completed]\nSuccess: {}\nOutput: {}",
                            task_id, success, output
                        )
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

            match self
                .inner
                .run_iteration(
                    agent_id,
                    llm,
                    tools,
                    &mut messages,
                    &options_with_tools,
                    &mut total_usage,
                    iterations,
                    &mut loop_detector,
                    &ctx_manager,
                    permission,
                    permission_checker,
                    event_tx.as_ref(),
                )
                .await?
            {
                RuntimeIterationOutcome::FinalResponse { content } => {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentExecutionEvent::completed(true, None)).await;
                    }
                    return Ok(LoopResult {
                        content,
                        total_usage,
                        iterations,
                        messages,
                    });
                }
                RuntimeIterationOutcome::ToolsExecuted => continue,
            }
        }

        // Iteration limit reached — return last assistant content.
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

    /// Run with pause support.
    ///
    /// Deprecated compatibility wrapper. Use [`PausableAgenticLoop::execute_with_pause`] for new code.
    #[deprecated(note = "use PausableAgenticLoop::execute_with_pause")]
    pub async fn run_with_pause(
        &self,
        agent_id: &AgentId,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
        permission: &Permission,
        permission_checker: Option<&dyn PermissionChecker>,
        event_tx: Option<mpsc::Sender<AgentExecutionEvent>>,
    ) -> MacacaResult<LoopResult> {
        self.execute_with_pause(
            agent_id,
            llm,
            tools,
            messages,
            options,
            permission,
            permission_checker,
            event_tx,
        )
        .await
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::LlmResponse;
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
                    reasoning_content: None,
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
                    reasoning_content: None,
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
                reasoning_content: None,
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
                reasoning_content: None,
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
            .execute(
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
            ..RuntimeConfig::default()
        });
        let llm = MockToolLlm::new();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![
            LlmMessage::system("You are helpful."),
            LlmMessage::user("Run echo hello"),
        ];

        let result = runner
            .execute(
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
            ..RuntimeConfig::default()
        });
        let llm = InfiniteToolLlm;
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let messages = vec![LlmMessage::user("Loop forever")];

        let result = runner
            .execute(
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
            ..RuntimeConfig::default()
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
            .execute(
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
                        reasoning_content: None,
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
                        reasoning_content: None,
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
            .execute(
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

    #[tokio::test]
    async fn execute_with_events_preserves_event_order() {
        let runner = AgenticLoop::new(RuntimeConfig {
            max_iterations: 5,
            tool_timeout: Duration::from_secs(5),
            ..RuntimeConfig::default()
        });
        let llm = MockToolLlm::new();
        let tools = macaca_tools::DefaultToolSet::new();
        let agent_id = AgentId::new();
        let (tx, mut rx) = mpsc::channel(16);

        let result = runner
            .execute_with_events(
                &agent_id,
                &llm,
                &tools,
                vec![LlmMessage::user("Run echo hello")],
                &LlmOptions::default(),
                &default_permission(),
                None,
                Some(tx),
            )
            .await
            .unwrap();

        assert_eq!(result.iterations, 2);

        let mut kinds = Vec::new();
        while let Some(event) = rx.recv().await {
            let kind = match event {
                AgentExecutionEvent::Thinking { .. } => "thinking",
                AgentExecutionEvent::ToolCall { .. } => "tool_call",
                AgentExecutionEvent::DriverTrace { driver_name, .. }
                    if driver_name == "macaca-context" =>
                {
                    continue;
                }
                AgentExecutionEvent::DriverTrace { .. } => "driver_trace",
                AgentExecutionEvent::ToolResult { .. } => "tool_result",
                AgentExecutionEvent::Assistant { .. } => "assistant",
                AgentExecutionEvent::Completed { .. } => "completed",
            };
            kinds.push(kind);
        }

        assert_eq!(
            kinds,
            vec![
                "thinking",
                "tool_call",
                "driver_trace",
                "driver_trace",
                "tool_result",
                "thinking",
                "assistant",
                "completed"
            ]
        );
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
