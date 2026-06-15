//! Top-level loop drivers: execute, run, and event-sink variants.
//!
//! These methods implement the **driver** around [`AgenticLoop::run_iteration`].

use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, MacacaResult, Permission, TokenUsage,
};
use macaca_tools::ToolCatalog;
use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use crate::context_window::{ContextWindowConfig, ContextWindowManager};
use crate::loop_detector::{LoopDetector, LoopDetectorConfig};
use crate::permission::PermissionChecker;
use crate::template::RuntimeIterationOutcome;

use super::helpers::tool_definitions;
use super::types::{AgenticLoop, LoopResult, RuntimeConfig};

impl AgenticLoop {
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
    ///
    /// Runtime model:
    /// - each iteration assembles context through the configured context engine
    /// - the LLM response is inspected for tool calls
    /// - tool results are appended back into transcript history
    /// - the loop repeats until a final assistant answer is produced
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
}

impl Default for AgenticLoop {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}
