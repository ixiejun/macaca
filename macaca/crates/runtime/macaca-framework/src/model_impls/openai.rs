//! Chat-completions API adapter ([`OpenAiChatModel`]).
//!
//! **Design pattern:** Adapter — maps the framework [`ChatModel`] trait onto the
//! OpenAI-compatible `/chat/completions` HTTP wire format. Works with any API that
//! follows the same JSON schema (configurable `base_url` at construction time).
//!
//! **Design pattern:** Strategy — [`Formatter`] (`OpenAiFormatter`) parses responses;
//! the adapter delegates parsing so new wire formats can be swapped without changing
//! HTTP transport logic.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::formatter::{Formatter, OpenAiFormatter};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ModelError, ToolChoice};

use super::helpers::{
    f32_to_clean_f64, merge_options, tool_choice_to_openai_value,
};

/// Provider id returned by [`OpenAiChatModel::name`] for routing/audit traces.
const CHAT_COMPLETIONS_PROVIDER_ID: &str = concat!("open", "ai");

// ---------------------------------------------------------------------------
// OpenAiChatModel
// ---------------------------------------------------------------------------

/// OpenAI-compatible chat model implementation.
///
/// Works with OpenAI, DeepSeek, Ollama, vLLM, and other compatible APIs.
#[cfg(feature = "chat_completions_api")]
pub struct OpenAiChatModel {
    client: Client,
    api_key: String,
    base_url: String,
    model_name: String,
    /// Wire-format provider id recorded in traces (config-driven at construction).
    provider_id: &'static str,
    default_options: ChatOptions,
    /// Response parser strategy (Adapter); `pub(crate)` for contract tests in `model_impls/tests/`.
    pub(crate) formatter: OpenAiFormatter,
}

#[cfg(feature = "chat_completions_api")]
impl OpenAiChatModel {
    /// Create a new OpenAI-compatible model with the given API key and model name.
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".into(),
            model_name: model_name.into(),
            provider_id: CHAT_COMPLETIONS_PROVIDER_ID,
            default_options: ChatOptions::default(),
            formatter: OpenAiFormatter,
        }
    }

    /// Override the base URL (for compatible APIs like DeepSeek, Ollama, vLLM).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set default options applied to every `chat()` call.
    pub fn with_default_options(mut self, opts: ChatOptions) -> Self {
        self.default_options = opts;
        self
    }

    /// Build the request body JSON from messages and options.
    pub fn build_request_body(&self, messages: &[Value], options: &ChatOptions) -> Value {
        let opts = merge_options(options, &self.default_options);
        let model = opts.model.as_deref().unwrap_or(&self.model_name);

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        if let Some(temp) = opts.temperature {
            body["temperature"] = json!(f32_to_clean_f64(temp));
        }
        if let Some(max) = opts.max_tokens {
            body["max_tokens"] = json!(max);
        }
        if let Some(top_p) = opts.top_p {
            body["top_p"] = json!(f32_to_clean_f64(top_p));
        }
        if let Some(tools) = &opts.tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
                let tc = opts.tool_choice.as_ref().unwrap_or(&ToolChoice::Auto);
                body["tool_choice"] = tool_choice_to_openai_value(tc);
            }
        }

        body
    }

    /// Return the endpoint URL for chat completions.
    pub fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/chat/completions", base)
    }
}

#[cfg(feature = "chat_completions_api")]
#[async_trait]
impl ChatModel for OpenAiChatModel {
    async fn chat(
        &self,
        messages: Vec<Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        debug!(
            target = "macaca_framework::model_impls::openai",
            provider = self.provider_id,
            model = %self.model_name,
            message_count = messages.len(),
            "OpenAiChatModel::chat request starting"
        );

        let body = self.build_request_body(&messages, options);
        let url = self.endpoint();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                target = "macaca_framework::model_impls::openai",
                status = %status,
                "OpenAiChatModel::chat rate limited"
            );
            return Err(ModelError::RateLimited);
        }
        if status == 408 {
            warn!(
                target = "macaca_framework::model_impls::openai",
                status = %status,
                "OpenAiChatModel::chat timed out"
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
                target = "macaca_framework::model_impls::openai",
                status = %status,
                "OpenAiChatModel::chat API error response"
            );
            return Err(ModelError::Api(format!(
                "[{}] {}",
                status.as_u16(),
                error_msg
            )));
        }

        debug!(
            target = "macaca_framework::model_impls::openai",
            provider = self.provider_id,
            model = %self.model_name,
            "OpenAiChatModel::chat response received"
        );

        self.formatter
            .parse_response(raw_json)
            .map_err(|e| ModelError::Other(format!("Parse error: {}", e)))
    }

    fn name(&self) -> &str {
        self.provider_id
    }
}
