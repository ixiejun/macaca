//! Pause/resume decorator around [`super::types::AgenticLoop`].
//!
//! **Decorator pattern**: [`PausableAgenticLoop`] wraps the inner loop and gates
//! each iteration on an external pause signal (human-in-the-loop, approval flows).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc as StdArc;

use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, MacacaResult, Permission, TokenUsage,
};
use macaca_tools::ToolCatalog;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::context_window::{ContextWindowConfig, ContextWindowManager};
use crate::loop_detector::{LoopDetector, LoopDetectorConfig};
use crate::permission::PermissionChecker;
use crate::template::RuntimeIterationOutcome;

use super::helpers::tool_definitions;
use super::types::{AgenticLoop, LoopResult, RuntimeConfig};

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
}

// ── Tests ────────────────────────────────────────────────────────────────────
