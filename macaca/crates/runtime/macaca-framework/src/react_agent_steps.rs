// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

impl ReActAgent {
    pub(super) async fn reasoning(
        &self,
        runtime: &RuntimeContext,
        state: &AgentState,
    ) -> AgentResult<ChatResponse> {
        let system_context = MiddlewareContext::new(
            MiddlewareStage::SystemPrompt,
            MiddlewarePhase::Before,
            runtime.clone(),
            Msg::system(self.sys_prompt.as_str()),
        )
        .with_agent_state(state.clone());
        let system_context = self.middleware.run(system_context).await?;
        let mut messages = vec![system_context.message];
        if let Some(reminder) = &self.execution_config.structured_output_reminder {
            debug!(
                agent = %self.name,
                trace_id = %runtime.trace_id,
                "react_agent structured output reminder attached"
            );
            messages.push(Msg::system(reminder.as_str()));
        }
        messages.extend(state.conversation.clone());

        let before_context = MiddlewareContext::new(
            MiddlewareStage::Reasoning,
            MiddlewarePhase::Before,
            runtime.clone(),
            messages
                .last()
                .cloned()
                .unwrap_or_else(|| Msg::system(self.sys_prompt.as_str())),
        )
        .with_agent_state(state.clone());
        let _ = self.middleware.run(before_context).await?;

        let formatted = self.formatter.format(&messages);
        let tool_defs = {
            let toolkit = self.toolkit.lock().await;
            toolkit.get_definitions()
        };
        let has_tools = !tool_defs.is_empty();
        let options = ChatOptions {
            model: self.model_name.clone(),
            temperature: self.execution_config.temperature,
            max_tokens: self.execution_config.max_tokens,
            top_p: self.execution_config.top_p,
            tools: has_tools.then_some(tool_defs),
            tool_choice: if has_tools {
                self.tool_choice.clone()
            } else {
                None
            },
            ..Default::default()
        };

        let model_context = MiddlewareContext::new(
            MiddlewareStage::ModelCall,
            MiddlewarePhase::Before,
            runtime.clone(),
            messages
                .last()
                .cloned()
                .unwrap_or_else(|| Msg::system(self.sys_prompt.as_str())),
        )
        .with_agent_state(state.clone());
        let _ = self.middleware.run(model_context).await?;

        self.chat_with_retry(formatted, &options, runtime)
            .await
            .map_err(|error| AgentError::Llm(error.to_string()))
    }

    pub(super) async fn acting(
        &self,
        runtime: &RuntimeContext,
        tool_call: &ToolUseBlock,
    ) -> AgentResult<ToolResultBlock> {
        let acting_context = MiddlewareContext::new(
            MiddlewareStage::Acting,
            MiddlewarePhase::Before,
            runtime.clone(),
            Msg::assistant(
                &self.name,
                MsgContent::Blocks(vec![ContentBlock::ToolUse(tool_call.clone())]),
            ),
        );
        let _ = self.middleware.run(acting_context).await?;

        let result = self.execute_tool_via_bridge(runtime, tool_call).await?;
        Ok(super::tool_bridge::tool_execution_result_to_block(
            tool_call, result,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_response_events(
        &self,
        events: &mut Vec<AgentEvent>,
        accumulator: &mut AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
        runtime: &RuntimeContext,
        response: &ChatResponse,
    ) -> AgentResult<()> {
        self.push_event(
            events,
            accumulator,
            reply_id,
            sequence,
            AgentEventType::ModelCallEnd,
            AgentEventPayload::ModelCall {
                model_id: Some(self.model.name().to_string()),
                request_id: Some(response.id.clone()),
                usage: Some(serde_json::json!(response.usage)),
                reason: Some(if response.has_tool_calls() {
                    GenerateReason::ToolCalls.as_str().to_string()
                } else {
                    GenerateReason::EndTurn.as_str().to_string()
                }),
            },
            None,
            runtime,
        )?;

        for (index, block) in response.content.iter().enumerate() {
            let block_id = format!("block-{}-{index}", *sequence + 1);
            match block {
                ContentBlock::Text(text) => {
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::TextBlockStart,
                        AgentEventPayload::Empty,
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::TextBlockDelta,
                        AgentEventPayload::TextDelta {
                            delta: text.text.clone(),
                        },
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::TextBlockEnd,
                        AgentEventPayload::Empty,
                        Some(block_id),
                        runtime,
                    )?;
                }
                ContentBlock::Thinking(thinking) => {
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::ThinkingBlockStart,
                        AgentEventPayload::Empty,
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::ThinkingBlockDelta,
                        AgentEventPayload::ThinkingDelta {
                            delta: thinking.thinking.clone(),
                        },
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::ThinkingBlockEnd,
                        AgentEventPayload::Empty,
                        Some(block_id),
                        runtime,
                    )?;
                }
                ContentBlock::Data(data) => {
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::DataBlockStart,
                        AgentEventPayload::DataDelta {
                            data: data.data.clone(),
                        },
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::DataBlockEnd,
                        AgentEventPayload::Empty,
                        Some(block_id),
                        runtime,
                    )?;
                }
                ContentBlock::ToolUse(tool) => {
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::ToolCallStart,
                        AgentEventPayload::ToolCall {
                            tool_call_id: tool.id.clone(),
                            name: tool.name.clone(),
                            input: tool.input.clone(),
                            raw_delta: tool.raw_input.clone(),
                        },
                        Some(block_id.clone()),
                        runtime,
                    )?;
                    self.push_event(
                        events,
                        accumulator,
                        reply_id,
                        sequence,
                        AgentEventType::ToolCallEnd,
                        AgentEventPayload::Empty,
                        Some(block_id),
                        runtime,
                    )?;
                }
                ContentBlock::ToolResult(_)
                | ContentBlock::Hint(_)
                | ContentBlock::Image(_)
                | ContentBlock::Audio(_)
                | ContentBlock::Video(_) => {}
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_tool_result_events(
        &self,
        events: &mut Vec<AgentEvent>,
        accumulator: &mut AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
        runtime: &RuntimeContext,
        result: &ToolResultBlock,
    ) -> AgentResult<()> {
        let block_id = format!("tool-result-{}", result.tool_use_id);
        self.push_event(
            events,
            accumulator,
            reply_id,
            sequence,
            AgentEventType::ToolResultStart,
            AgentEventPayload::ToolResult {
                tool_call_id: result.tool_use_id.clone(),
                name: result.name.clone(),
                delta: String::new(),
                is_error: result.is_error,
            },
            Some(block_id.clone()),
            runtime,
        )?;
        self.push_event(
            events,
            accumulator,
            reply_id,
            sequence,
            AgentEventType::ToolResultTextDelta,
            AgentEventPayload::ToolResult {
                tool_call_id: result.tool_use_id.clone(),
                name: result.name.clone(),
                delta: result.output.clone(),
                is_error: result.is_error,
            },
            Some(block_id.clone()),
            runtime,
        )?;
        self.push_event(
            events,
            accumulator,
            reply_id,
            sequence,
            AgentEventType::ToolResultEnd,
            AgentEventPayload::Empty,
            Some(block_id),
            runtime,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_event(
        &self,
        events: &mut Vec<AgentEvent>,
        accumulator: &mut AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
        event_type: AgentEventType,
        payload: AgentEventPayload,
        block_id: Option<String>,
        runtime: &RuntimeContext,
    ) -> AgentResult<()> {
        *sequence += 1;
        let mut event = AgentEvent::new(reply_id, *sequence, event_type, payload)
            .with_trace_id(runtime.trace_id.clone());
        if let Some(block_id) = block_id {
            event = event.with_block_id(block_id);
        }
        accumulator
            .accept(event.clone())
            .map_err(|error| AgentError::Other(error.to_string()))?;
        events.push(event);
        Ok(())
    }

    pub(super) fn finish_interrupted(
        &self,
        command: AgentRunCommand,
        mut events: Vec<AgentEvent>,
        mut accumulator: AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
    ) -> AgentResult<AgentRunResult> {
        self.push_event(
            &mut events,
            &mut accumulator,
            reply_id,
            sequence,
            AgentEventType::Interrupted,
            AgentEventPayload::Lifecycle {
                agent_name: self.name.clone(),
                reason: Some(GenerateReason::Interrupted.as_str().to_string()),
            },
            None,
            &command.runtime,
        )?;
        let final_message = accumulator.finish();
        Ok(AgentRunResult {
            events,
            final_message,
            state: command.state,
            generate_reason: GenerateReason::Interrupted,
        })
    }

    pub(super) fn finish_with_reason(
        &self,
        command: AgentRunCommand,
        mut events: Vec<AgentEvent>,
        mut accumulator: AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
        generate_reason: GenerateReason,
    ) -> AgentResult<AgentRunResult> {
        self.push_event(
            &mut events,
            &mut accumulator,
            reply_id,
            sequence,
            AgentEventType::AgentEnd,
            AgentEventPayload::Lifecycle {
                agent_name: self.name.clone(),
                reason: Some(generate_reason.as_str().to_string()),
            },
            None,
            &command.runtime,
        )?;
        let final_message = accumulator.finish();
        Ok(AgentRunResult {
            events,
            final_message,
            state: command.state,
            generate_reason,
        })
    }
}

pub(super) fn response_to_content(response: &ChatResponse) -> MsgContent {
    if response.content.is_empty() {
        MsgContent::Text(String::new())
    } else if response.content.len() == 1 {
        match response.content.first() {
            Some(ContentBlock::Text(text)) => MsgContent::Text(text.text.clone()),
            _ => MsgContent::Blocks(response.content.clone()),
        }
    } else {
        MsgContent::Blocks(response.content.clone())
    }
}

pub(super) fn tool_response_text(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text.clone()),
            ContentBlock::Data(data) => Some(data.data.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
