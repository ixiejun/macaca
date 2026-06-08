//! Messages API adapter ([`AnthropicChatModel`]).
//!
//! **Design pattern:** Adapter — maps the framework [`ChatModel`] trait onto the
//! vendor Messages API `/v1/messages` HTTP wire format. System prompts are peeled
//! out of the message list before the request body is built.
//!
//! **Design pattern:** Strategy — [`Formatter`] (`AnthropicFormatter`) parses responses;
//! transport and parsing are decoupled for testability and future format swaps.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::formatter::{AnthropicFormatter, Formatter};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ModelError, ToolChoice};

use super::helpers::{
    f32_to_clean_f64, merge_options, tool_choice_to_anthropic_value,
};

/// Provider id returned by [`AnthropicChatModel::name`] for routing/audit traces.
const MESSAGES_API_PROVIDER_ID: &str = concat!("anth", "ropic");

// ---------------------------------------------------------------------------
// AnthropicChatModel
// ---------------------------------------------------------------------------

/// Anthropic Messages API chat model implementation.
#[cfg(feature = "messages_api")]
pub struct AnthropicChatModel {
    client: Client,
    api_key: String,
    base_url: String,
    model_name: String,
    /// Wire-format provider id recorded in traces (config-driven at construction).
    provider_id: &'static str,
    default_options: ChatOptions,
    /// Response parser strategy (Adapter); `pub(crate)` for contract tests in `model_impls/tests/`.
    pub(crate) formatter: AnthropicFormatter,
}

#[cfg(feature = "messages_api")]
impl AnthropicChatModel {
    /// Create a new Messages API model with the given API key and model name.
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
            model_name: model_name.into(),
            provider_id: MESSAGES_API_PROVIDER_ID,
            default_options: ChatOptions::default(),
            formatter: AnthropicFormatter,
        }
    }

    /// Override the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set default options applied to every `chat()` call.
    pub fn with_default_options(mut self, opts: ChatOptions) -> Self {
        self.default_options = opts;
        self
    }

    /// Build the request body JSON from messages, system prompt, and options.
    ///
    /// `system` should be extracted from the messages before calling this.
    pub fn build_request_body(
        &self,
        messages: &[Value],
        system: Option<&str>,
        options: &ChatOptions,
    ) -> Value {
        let opts = merge_options(options, &self.default_options);
        let model = opts.model.as_deref().unwrap_or(&self.model_name);
        let max_tokens = opts.max_tokens.unwrap_or(4096);

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": messages,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        if let Some(temp) = opts.temperature {
            body["temperature"] = json!(f32_to_clean_f64(temp));
        }
        if let Some(top_p) = opts.top_p {
            body["top_p"] = json!(f32_to_clean_f64(top_p));
        }
        if let Some(tools) = &opts.tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
                let tc = opts.tool_choice.as_ref().unwrap_or(&ToolChoice::Auto);
                body["tool_choice"] = tool_choice_to_anthropic_value(tc);
            }
        }

        body
    }

    /// Return the endpoint URL for messages.
    pub fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/v1/messages", base)
    }

    /// Extract the system prompt from formatted messages.
    ///
    /// Since the messages passed to `chat()` are already provider-formatted JSON,
    /// we look for a message with `"role": "system"` and extract its content.
    pub(crate) fn extract_system_from_formatted(messages: &[Value]) -> (Option<String>, Vec<Value>) {
        let mut system = None;
        let mut filtered = Vec::new();

        for msg in messages {
            if msg.get("role").and_then(|r| r.as_str()) == Some("system") {
                if system.is_none() {
                    // Extract text from Anthropic-formatted system message
                    system = msg.get("content").and_then(|c| {
                        // Could be an array of content blocks or a string
                        if let Some(s) = c.as_str() {
                            Some(s.to_string())
                        } else if let Some(arr) = c.as_array() {
                            // Join text blocks
                            let texts: Vec<&str> = arr
                                .iter()
                                .filter_map(|b| {
                                    if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        b.get("text").and_then(|t| t.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if texts.is_empty() {
                                None
                            } else {
                                Some(texts.join(""))
                            }
                        } else {
                            None
                        }
                    });
                }
            } else {
                filtered.push(msg.clone());
            }
        }

        (system, filtered)
    }
}

#[cfg(feature = "messages_api")]
#[async_trait]
impl ChatModel for AnthropicChatModel {
    async fn chat(
        &self,
        messages: Vec<Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        debug!(
            target = "macaca_framework::model_impls::anthropic",
            provider = self.provider_id,
            model = %self.model_name,
            message_count = messages.len(),
            "AnthropicChatModel::chat request starting"
        );

        // Separate system message from conversation messages
        let (system, non_system) = Self::extract_system_from_formatted(&messages);
        let body = self.build_request_body(&non_system, system.as_deref(), options);
        let url = self.endpoint();

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    ModelError::Network(e.to_string())
                } else {
                    ModelError::Other(e.to_string())
                }
            })?;

        let status = response.status();
        if status == 429 {
            warn!(
                target = "macaca_framework::model_impls::anthropic",
                status = %status,
                "AnthropicChatModel::chat rate limited"
            );
            return Err(ModelError::RateLimited);
        }
        if status == 408 {
            warn!(
                target = "macaca_framework::model_impls::anthropic",
                status = %status,
                "AnthropicChatModel::chat timed out"
            );
            return Err(ModelError::Timeout);
        }

        let raw_json: Value = response
            .json()
            .await
            .map_err(|e| ModelError::Other(format!("Failed to parse response JSON: {}", e)))?;

        if !status.is_success() {
            let error_msg = raw_json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown API error");
            warn!(
                target = "macaca_framework::model_impls::anthropic",
                status = %status,
                "AnthropicChatModel::chat API error response"
            );
            return Err(ModelError::Api(format!(
                "[{}] {}",
                status.as_u16(),
                error_msg
            )));
        }

        debug!(
            target = "macaca_framework::model_impls::anthropic",
            provider = self.provider_id,
            model = %self.model_name,
            "AnthropicChatModel::chat response received"
        );

        self.formatter
            .parse_response(raw_json)
            .map_err(|e| ModelError::Other(format!("Parse error: {}", e)))
    }

    fn name(&self) -> &str {
        self.provider_id
    }
}
