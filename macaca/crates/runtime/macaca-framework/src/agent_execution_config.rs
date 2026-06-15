// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! ReActAgent execution configuration.
//!
//! This module keeps retry, fallback, generation-option, structured-output,
//! and graceful-shutdown behavior behind a small Strategy object. The agent
//! loop stays provider-neutral: callers may provide any fallback model or
//! reminder text without hardcoded provider, application, workflow, or business
//! names entering the OS framework.

use std::sync::Arc;

use tracing::{debug, info, warn};

use super::*;
use crate::model::ModelError;
use crate::runtime_context::PermissionState;

/// RuntimeContext extras key used to carry a sanitized external execution
/// completion back into a suspended ReActAgent run.
pub const EXTERNAL_EXECUTION_RESULT_EXTRA: &str = "react_agent.external_execution_result";

/// Provider-neutral model execution settings for ReActAgent.
#[derive(Clone, Default)]
pub struct AgentExecutionConfig {
    pub model_retry_limit: usize,
    pub fallback_model: Option<Arc<dyn ChatModel>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub structured_output_reminder: Option<String>,
}

impl AgentExecutionConfig {
    /// Set bounded model retries. The first attempt is not counted as a retry.
    pub fn with_model_retry_limit(mut self, retry_limit: usize) -> Self {
        self.model_retry_limit = retry_limit;
        self
    }

    /// Set a fallback model strategy for degraded primary model behavior.
    pub fn with_fallback_model(mut self, fallback_model: Arc<dyn ChatModel>) -> Self {
        self.fallback_model = Some(fallback_model);
        self
    }

    /// Set optional provider-neutral sampling options.
    pub fn with_generation_options(
        mut self,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        top_p: Option<f32>,
    ) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self.top_p = top_p;
        self
    }

    /// Add a generic reminder that can steer structured output without
    /// hardcoding a schema or application-specific output contract.
    pub fn with_structured_output_reminder(mut self, reminder: impl Into<String>) -> Self {
        self.structured_output_reminder = Some(reminder.into());
        self
    }
}

impl ReActAgent {
    pub fn with_execution_config(mut self, config: AgentExecutionConfig) -> Self {
        self.execution_config = config;
        self
    }

    pub fn with_model_retry_limit(mut self, retry_limit: usize) -> Self {
        self.execution_config.model_retry_limit = retry_limit;
        self
    }

    pub fn with_fallback_model(mut self, fallback_model: Arc<dyn ChatModel>) -> Self {
        self.execution_config.fallback_model = Some(fallback_model);
        self
    }

    pub fn with_structured_output_reminder(mut self, reminder: impl Into<String>) -> Self {
        self.execution_config.structured_output_reminder = Some(reminder.into());
        self
    }

    pub fn request_graceful_shutdown(&self) {
        info!(agent = %self.name, "react_agent graceful shutdown requested");
        self.cancel_token.cancel();
    }

    pub(super) async fn chat_with_retry(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
        runtime: &RuntimeContext,
    ) -> Result<ChatResponse, ModelError> {
        let attempts = self.execution_config.model_retry_limit + 1;
        for attempt in 0..attempts {
            debug!(
                agent = %self.name,
                trace_id = %runtime.trace_id,
                attempt,
                "react_agent primary model attempt started"
            );
            match self.model.chat(messages.clone(), options).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt + 1 < attempts => {
                    warn!(
                        agent = %self.name,
                        trace_id = %runtime.trace_id,
                        attempt,
                        error = %error,
                        "react_agent primary model attempt failed; retrying"
                    );
                }
                Err(error) => {
                    warn!(
                        agent = %self.name,
                        trace_id = %runtime.trace_id,
                        error = %error,
                        has_fallback = self.execution_config.fallback_model.is_some(),
                        "react_agent primary model exhausted retry budget"
                    );
                    if let Some(fallback) = &self.execution_config.fallback_model {
                        info!(
                            agent = %self.name,
                            trace_id = %runtime.trace_id,
                            route_source = "fallback",
                            fallback_model_configured = true,
                            "react_agent fallback model attempt started"
                        );
                        return fallback.chat(messages, options).await;
                    }
                    return Err(error);
                }
            }
        }
        unreachable!("bounded model retry loop always returns from the final attempt")
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_pending_external_execution(
        &self,
        command: &mut AgentRunCommand,
        events: &mut Vec<AgentEvent>,
        accumulator: &mut AgentEventAccumulator,
        reply_id: &str,
        sequence: &mut u64,
    ) -> AgentResult<bool> {
        let PermissionState::WaitingForExternalExecution {
            request_id,
            tool_call_id,
        } = command.state.permission.clone()
        else {
            return Ok(false);
        };

        let Some(result) = command.runtime.extras.get(EXTERNAL_EXECUTION_RESULT_EXTRA) else {
            warn!(
                agent = %self.name,
                trace_id = %command.runtime.trace_id,
                request_id = %request_id,
                "react_agent remains suspended waiting for external execution result"
            );
            self.push_event(
                events,
                accumulator,
                reply_id,
                sequence,
                AgentEventType::RequireExternalExecution,
                AgentEventPayload::ExternalExecutionRequest {
                    request_id,
                    tool_call_id,
                    metadata: serde_json::json!({ "resume_required": true }),
                },
                None,
                &command.runtime,
            )?;
            return Ok(true);
        };

        let result_request_id = result
            .get("request_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if result_request_id != request_id {
            warn!(
                agent = %self.name,
                trace_id = %command.runtime.trace_id,
                expected_request_id = %request_id,
                actual_request_id = %result_request_id,
                "react_agent external execution result ignored because request id mismatched"
            );
            return Ok(true);
        }

        let success = result
            .get("success")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let output = result
            .get("output")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let reason = result
            .get("reason")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);

        info!(
            agent = %self.name,
            trace_id = %command.runtime.trace_id,
            request_id = %request_id,
            success,
            "react_agent external execution result ingested"
        );
        self.push_event(
            events,
            accumulator,
            reply_id,
            sequence,
            AgentEventType::ExternalExecutionResult,
            AgentEventPayload::ExternalExecutionResult {
                request_id: request_id.clone(),
                success,
                output: output.clone(),
                reason: reason.clone(),
            },
            None,
            &command.runtime,
        )?;

        if let Some(tool_call_id) = tool_call_id {
            command.state.push_message(Msg::tool_result(
                &tool_call_id,
                "external_execution",
                output.unwrap_or_else(|| reason.unwrap_or_default()),
                !success,
            ));
        }
        command.state.permission = PermissionState::Clear;
        Ok(false)
    }
}
