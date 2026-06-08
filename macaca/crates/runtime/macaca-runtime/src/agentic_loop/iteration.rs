//! Single-iteration template: LLM call, tool execution, context compaction.
//!
//! [`AgenticLoop::run_iteration`] is the **Template Method** step invoked by
//! [`super::execute`] and [`super::pausable::PausableAgenticLoop`].

use macaca_context::{
    assemble_context_providers, ContextAssembleInput, ContextEngineSelection, ContextFacade,
    ContextFacadeAssemblyPolicy, ProviderAssemblyEnvironment, ProviderFactoryInput,
};
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentExecutionEvent, AgentId, LlmMessage, LlmOptions, MacacaResult, Permission, TokenUsage,
    ToolCall,
};
use macaca_tools::ToolCatalog;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::context_window::ContextWindowManager;
use crate::events::RuntimeEventSink;
use crate::loop_detector::{LoopDetector, LoopDetectorAction};
use crate::permission::PermissionChecker;
use crate::template::RuntimeIterationOutcome;

use super::helpers::accumulate_usage;
use super::types::AgenticLoop;

impl AgenticLoop {
    /// Core single-iteration logic shared by all loop variants.
    ///
    /// Handles LLM call, tool execution, event emission, and loop detection.
    /// Returns `RuntimeIterationOutcome` so callers can decide whether to continue.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_iteration(
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

        // The runtime now has a two-stage context pipeline:
        // 1. `ContextWindowManager` performs a coarse trim on the in-memory
        //    transcript to avoid obviously oversized requests.
        // 2. `ContextFacade` runs the composer (provider candidates) then the selected
        //    context engine (`legacy`, `windowed`, `pruning`, `summary`, …) for
        //    provider-neutral prompt assembly and a structured report.
        //
        // The original `messages` vector is intentionally left intact so the
        // runtime keeps its full internal history even if the outgoing prompt is
        // trimmed or rewritten for this specific model call.
        let trimmed = ctx_manager.trim_if_needed(messages.clone());
        let env = ProviderAssemblyEnvironment::kernel_minimal();
        let factory_input = ProviderFactoryInput {
            agent_name: agent_id.to_string(),
            session_id: None,
            params: serde_json::json!({}),
        };
        let (providers, _) =
            assemble_context_providers(&self.config.context, &env, &factory_input, None).await?;
        let policy = ContextFacadeAssemblyPolicy::from_context_config_parts(
            self.config.context.governance.clone(),
            self.config.context.trust_governance.clone(),
            self.config.context.knowledge_digest.clone(),
        );
        let assembled = ContextFacade::builtins(ContextEngineSelection {
            engine_id: self.config.context_engine.clone(),
            fallback_engine_id: self.config.context_fallback_engine.clone(),
        })
        .assemble_model_context(
            ContextAssembleInput {
                app_id: None,
                session_id: None,
                agent_name: agent_id.to_string(),
                model: options_with_tools.model.clone(),
                base_messages: trimmed,
                options: options_with_tools.clone(),
                budget: self.config.context_budget,
            },
            &providers,
            policy,
        )
        .await?;
        // Emit a compact driver trace summary rather than the full prompt body.
        // This keeps runtime tracing cheap while still exposing enough metadata
        // for debugging engine selection, pruning, and token pressure.
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
        // Only the assembled prompt slice is sent to the LLM. Any trimming,
        // pruning, or fallback has already been applied by the context engine.
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
