// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It provides the canonical
// AgentScope 2.0-style ReAct loop whose output is a typed event stream.

//! AgentScope 2.0-style ReAct loop.
//!
//! `ReActAgent` is the canonical Macaca framework agent implementation. The
//! event-stream runtime owns the standard module, type, and file names so
//! consumers do not depend on version-suffixed framework symbols.
//!
//! Design patterns:
//! - **Template Method**: the loop follows input -> reasoning -> acting ->
//!   finish/suspend/max-iterations in a fixed order.
//! - **Observer/Event Sourcing**: every transition is emitted as `AgentEvent`.
//! - **Memento**: durable `AgentState` is updated with replayable conversation
//!   facts and returned to the caller.
//! - **Decorator**: `AgentMiddlewareChain` can wrap stages before side effects.

#[path = "agent_execution_config.rs"]
mod agent_execution_config;
#[path = "react_agent_steps.rs"]
mod react_agent_steps;
#[path = "react_agent_trait_adapter.rs"]
mod react_agent_trait_adapter;
#[path = "tool_bridge.rs"]
mod tool_bridge;

use std::sync::Arc;

use macaca_proto::AgentId;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::agent::{AgentError, AgentResult};
use crate::event_contract::{AgentEvent, AgentEventAccumulator, AgentEventPayload, AgentEventType};
use crate::formatter::Formatter;
use crate::message::{ContentBlock, Msg, MsgContent, ToolResultBlock, ToolUseBlock};
use crate::middleware::{
    AgentMiddlewareChain, MiddlewareContext, MiddlewarePhase, MiddlewareStage,
};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ToolChoice};
use crate::permission_contract::{
    AllowAllToolPermissionPolicy, ToolPermissionCommand, ToolPermissionEngineBridge,
    ToolPermissionPolicy,
};
use crate::runtime_context::{AgentState, RuntimeContext};
use crate::tool::Toolkit;
use crate::tool_contract::GuardedToolExecutor;

/// Generate reason recorded in final metadata and event payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateReason {
    EndTurn,
    ToolCalls,
    MaxIterations,
    Interrupted,
    UserConfirmRequired,
    ExternalExecutionRequired,
    Denied,
}

impl GenerateReason {
    fn as_str(self) -> &'static str {
        match self {
            GenerateReason::EndTurn => "end_turn",
            GenerateReason::ToolCalls => "tool_calls",
            GenerateReason::MaxIterations => "max_iterations",
            GenerateReason::Interrupted => "interrupted",
            GenerateReason::UserConfirmRequired => "user_confirm_required",
            GenerateReason::ExternalExecutionRequired => "external_execution_required",
            GenerateReason::Denied => "denied",
        }
    }
}

/// Command object for one ReActAgent run.
pub struct AgentRunCommand {
    pub input: Msg,
    pub runtime: RuntimeContext,
    pub state: AgentState,
}

/// Result of one ReActAgent run.
pub struct AgentRunResult {
    pub events: Vec<AgentEvent>,
    pub final_message: Msg,
    pub state: AgentState,
    pub generate_reason: GenerateReason,
}

/// Event-stream-first ReAct agent.
pub struct ReActAgent {
    name: String,
    id: AgentId,
    sys_prompt: String,
    model: Arc<dyn ChatModel>,
    formatter: Arc<dyn Formatter>,
    toolkit: Arc<Mutex<Toolkit>>,
    middleware: Arc<AgentMiddlewareChain>,
    permission_policy: Arc<dyn ToolPermissionPolicy>,
    tool_executor: Arc<GuardedToolExecutor>,
    max_iters: usize,
    cancel_token: CancellationToken,
    model_name: Option<String>,
    tool_choice: Option<ToolChoice>,
    execution_config: agent_execution_config::AgentExecutionConfig,
}

impl ReActAgent {
    /// Create a new event-stream-first ReAct agent.
    pub fn new(
        name: impl Into<String>,
        sys_prompt: impl Into<String>,
        model: Arc<dyn ChatModel>,
        formatter: Arc<dyn Formatter>,
    ) -> Self {
        Self {
            name: name.into(),
            id: AgentId::new(),
            sys_prompt: sys_prompt.into(),
            model,
            formatter,
            toolkit: Arc::new(Mutex::new(Toolkit::new())),
            middleware: Arc::new(AgentMiddlewareChain::new()),
            permission_policy: Arc::new(AllowAllToolPermissionPolicy),
            tool_executor: Arc::new(GuardedToolExecutor::new()),
            max_iters: 10,
            cancel_token: CancellationToken::new(),
            model_name: None,
            tool_choice: None,
            execution_config: agent_execution_config::AgentExecutionConfig::default(),
        }
    }

    pub fn with_toolkit(mut self, toolkit: Toolkit) -> Self {
        self.toolkit = Arc::new(Mutex::new(toolkit));
        self
    }

    pub fn with_middleware(mut self, middleware: AgentMiddlewareChain) -> Self {
        self.middleware = Arc::new(middleware);
        self
    }

    pub fn with_permission_policy(mut self, policy: Arc<dyn ToolPermissionPolicy>) -> Self {
        self.permission_policy = policy;
        self
    }

    pub fn with_tool_executor(mut self, executor: GuardedToolExecutor) -> Self {
        self.tool_executor = Arc::new(executor);
        self
    }

    pub fn with_max_iters(mut self, max_iters: usize) -> Self {
        self.max_iters = max_iters;
        self
    }

    pub fn with_model_name(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = Some(model_name.into());
        self
    }

    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Execute the agent and return the full typed event stream.
    pub async fn run(&self, mut command: AgentRunCommand) -> AgentResult<AgentRunResult> {
        command.input.validate_for_framework().map_err(|error| {
            AgentError::Other(format!("invalid framework input message: {error}"))
        })?;

        let reply_id = uuid::Uuid::new_v4().to_string();
        let mut sequence = 0_u64;
        let mut events = Vec::new();
        let mut accumulator = AgentEventAccumulator::new(reply_id.clone(), self.name.clone());
        command.state.push_message(command.input.clone());

        info!(
            agent = %self.name,
            trace_id = %command.runtime.trace_id,
            reply_id = %reply_id,
            "react_agent run started"
        );
        self.push_event(
            &mut events,
            &mut accumulator,
            &reply_id,
            &mut sequence,
            AgentEventType::AgentStart,
            AgentEventPayload::Lifecycle {
                agent_name: self.name.clone(),
                reason: None,
            },
            None,
            &command.runtime,
        )?;
        if self.handle_pending_external_execution(
            &mut command,
            &mut events,
            &mut accumulator,
            &reply_id,
            &mut sequence,
        )? {
            return self.finish_with_reason(
                command,
                events,
                accumulator,
                &reply_id,
                &mut sequence,
                GenerateReason::ExternalExecutionRequired,
            );
        }

        for iteration in 0..self.max_iters {
            if self.cancel_token.is_cancelled() {
                return self.finish_interrupted(
                    command,
                    events,
                    accumulator,
                    &reply_id,
                    &mut sequence,
                );
            }

            debug!(
                agent = %self.name,
                trace_id = %command.runtime.trace_id,
                iteration,
                "react_agent reasoning iteration started"
            );
            let response = self.reasoning(&command.runtime, &command.state).await?;
            self.emit_response_events(
                &mut events,
                &mut accumulator,
                &reply_id,
                &mut sequence,
                &command.runtime,
                &response,
            )?;

            let assistant_msg = Msg::assistant(
                &self.name,
                react_agent_steps::response_to_content(&response),
            );
            command.state.push_message(assistant_msg);

            if !response.has_tool_calls() {
                self.push_event(
                    &mut events,
                    &mut accumulator,
                    &reply_id,
                    &mut sequence,
                    AgentEventType::AgentEnd,
                    AgentEventPayload::Lifecycle {
                        agent_name: self.name.clone(),
                        reason: Some(GenerateReason::EndTurn.as_str().to_string()),
                    },
                    None,
                    &command.runtime,
                )?;
                let final_message = accumulator.finish();
                return Ok(AgentRunResult {
                    events,
                    final_message,
                    state: command.state,
                    generate_reason: GenerateReason::EndTurn,
                });
            }

            for tool_call in response.get_tool_calls() {
                if self.cancel_token.is_cancelled() {
                    return self.finish_interrupted(
                        command,
                        events,
                        accumulator,
                        &reply_id,
                        &mut sequence,
                    );
                }

                let decision = self
                    .permission_policy
                    .decide(ToolPermissionCommand {
                        runtime: &command.runtime,
                        state: &command.state,
                        tool_call,
                    })
                    .await?;
                match ToolPermissionEngineBridge::apply_decision(decision, tool_call) {
                    crate::permission_contract::PermissionGateOutcome::Allow => {}
                    crate::permission_contract::PermissionGateOutcome::Suspend {
                        permission,
                        result,
                    } => {
                        command.state.permission = permission;
                        if let crate::tool_contract::ToolExecutionResult::Suspended {
                            request_id,
                            reason,
                            external,
                        } = result
                        {
                            if external {
                                self.push_event(
                                    &mut events,
                                    &mut accumulator,
                                    &reply_id,
                                    &mut sequence,
                                    AgentEventType::RequireExternalExecution,
                                    AgentEventPayload::ExternalExecutionRequest {
                                        request_id,
                                        tool_call_id: Some(tool_call.id.clone()),
                                        metadata: serde_json::json!({
                                            "tool_name": tool_call.name.clone(),
                                        }),
                                    },
                                    None,
                                    &command.runtime,
                                )?;
                                return self.finish_with_reason(
                                    command,
                                    events,
                                    accumulator,
                                    &reply_id,
                                    &mut sequence,
                                    GenerateReason::ExternalExecutionRequired,
                                );
                            }
                            let prompt = match &command.state.permission {
                                crate::runtime_context::PermissionState::WaitingForUserConfirm {
                                    prompt,
                                    ..
                                } => prompt.clone(),
                                _ => reason,
                            };
                            self.push_event(
                                &mut events,
                                &mut accumulator,
                                &reply_id,
                                &mut sequence,
                                AgentEventType::RequireUserConfirm,
                                AgentEventPayload::UserConfirmRequest {
                                    request_id,
                                    prompt,
                                    metadata: serde_json::json!({
                                        "tool_call_id": tool_call.id.clone(),
                                        "tool_name": tool_call.name.clone(),
                                    }),
                                },
                                None,
                                &command.runtime,
                            )?;
                            return self.finish_with_reason(
                                command,
                                events,
                                accumulator,
                                &reply_id,
                                &mut sequence,
                                GenerateReason::UserConfirmRequired,
                            );
                        }
                    }

                    crate::permission_contract::PermissionGateOutcome::Deny {
                        permission,
                        result,
                    } => {
                        command.state.permission = permission;
                        let reason = match result {
                            crate::tool_contract::ToolExecutionResult::Denied { reason } => reason,
                            _ => "tool execution denied".to_string(),
                        };
                        self.push_event(
                            &mut events,
                            &mut accumulator,
                            &reply_id,
                            &mut sequence,
                            AgentEventType::RequestStop,
                            AgentEventPayload::Lifecycle {
                                agent_name: self.name.clone(),
                                reason: Some(reason),
                            },
                            None,
                            &command.runtime,
                        )?;
                        return self.finish_with_reason(
                            command,
                            events,
                            accumulator,
                            &reply_id,
                            &mut sequence,
                            GenerateReason::Denied,
                        );
                    }
                }
                let tool_result = self.acting(&command.runtime, tool_call).await?;
                self.emit_tool_result_events(
                    &mut events,
                    &mut accumulator,
                    &reply_id,
                    &mut sequence,
                    &command.runtime,
                    &tool_result,
                )?;
                command.state.push_message(Msg::tool_result(
                    &tool_result.tool_use_id,
                    tool_result.name.clone().unwrap_or_default(),
                    tool_result.output.clone(),
                    tool_result.is_error,
                ));
            }
        }

        warn!(
            agent = %self.name,
            trace_id = %command.runtime.trace_id,
            max_iters = self.max_iters,
            "react_agent exceeded max iterations"
        );
        self.push_event(
            &mut events,
            &mut accumulator,
            &reply_id,
            &mut sequence,
            AgentEventType::ExceedMaxIters,
            AgentEventPayload::Lifecycle {
                agent_name: self.name.clone(),
                reason: Some(GenerateReason::MaxIterations.as_str().to_string()),
            },
            None,
            &command.runtime,
        )?;
        self.push_event(
            &mut events,
            &mut accumulator,
            &reply_id,
            &mut sequence,
            AgentEventType::AgentEnd,
            AgentEventPayload::Lifecycle {
                agent_name: self.name.clone(),
                reason: Some(GenerateReason::MaxIterations.as_str().to_string()),
            },
            None,
            &command.runtime,
        )?;
        let final_message = accumulator.finish();
        Ok(AgentRunResult {
            events,
            final_message,
            state: command.state,
            generate_reason: GenerateReason::MaxIterations,
        })
    }

    // Step and event helper methods live in react_agent_steps.rs to keep this module small.
}

#[cfg(test)]
#[path = "react_agent_stream_tests.rs"]
mod react_agent_stream_tests;
