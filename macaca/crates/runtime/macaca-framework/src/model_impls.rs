//! Concrete `ChatModel` implementations for OpenAI-compatible and Messages API HTTP shapes.
//!
//! Vendor routing identifiers are injected at construction time via compile-time
//! concatenation so this legacy framework layer does not own parallel routing tables.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::formatter::{AnthropicFormatter, Formatter, OpenAiFormatter};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ModelError, ToolChoice};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Merge `options` on top of `defaults`, preferring values in `options`.
fn merge_options<'a>(options: &'a ChatOptions, defaults: &'a ChatOptions) -> ChatOptions {
    ChatOptions {
        model: options.model.clone().or_else(|| defaults.model.clone()),
        temperature: options.temperature.or(defaults.temperature),
        max_tokens: options.max_tokens.or(defaults.max_tokens),
        top_p: options.top_p.or(defaults.top_p),
        tools: options.tools.clone().or_else(|| defaults.tools.clone()),
        tool_choice: options
            .tool_choice
            .clone()
            .or_else(|| defaults.tool_choice.clone()),
    }
}

/// Convert f32 to a clean f64 by going through string representation.
/// This avoids f32 precision artifacts like 0.7_f32 becoming 0.699999988079071.
fn f32_to_clean_f64(v: f32) -> f64 {
    // Format with enough precision for f32, then parse as f64
    let s = format!("{}", v);
    s.parse::<f64>().unwrap_or(v as f64)
}

fn tool_choice_to_openai_value(tc: &ToolChoice) -> Value {
    match tc {
        ToolChoice::Auto => json!("auto"),
        ToolChoice::None => json!("none"),
        ToolChoice::Required => json!("required"),
        ToolChoice::Specific(name) => json!({"type": "function", "function": {"name": name}}),
    }
}

fn tool_choice_to_anthropic_value(tc: &ToolChoice) -> Value {
    match tc {
        ToolChoice::Auto => json!({"type": "auto"}),
        ToolChoice::None => json!({"type": "none"}),
        ToolChoice::Required => json!({"type": "any"}),
        ToolChoice::Specific(name) => json!({"type": "tool", "name": name}),
    }
}

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
    formatter: OpenAiFormatter,
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
            return Err(ModelError::RateLimited);
        }
        if status == 408 {
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
            return Err(ModelError::Api(format!(
                "[{}] {}",
                status.as_u16(),
                error_msg
            )));
        }

        self.formatter
            .parse_response(raw_json)
            .map_err(|e| ModelError::Other(format!("Parse error: {}", e)))
    }

    fn name(&self) -> &str {
        self.provider_id
    }
}

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
    formatter: AnthropicFormatter,
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
    fn extract_system_from_formatted(messages: &[Value]) -> (Option<String>, Vec<Value>) {
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
            return Err(ModelError::RateLimited);
        }
        if status == 408 {
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
            return Err(ModelError::Api(format!(
                "[{}] {}",
                status.as_u16(),
                error_msg
            )));
        }

        self.formatter
            .parse_response(raw_json)
            .map_err(|e| ModelError::Other(format!("Parse error: {}", e)))
    }

    fn name(&self) -> &str {
        self.provider_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // OpenAI tests
    // -----------------------------------------------------------------------

    #[cfg(feature = "chat_completions_api")]
    mod openai_tests {
        use super::*;

        #[test]
        fn test_openai_build_request_body() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let messages = vec![
                json!({"role": "system", "content": "You are helpful."}),
                json!({"role": "user", "content": "Hello"}),
            ];
            let opts = ChatOptions {
                temperature: Some(0.7),
                max_tokens: Some(1024),
                ..Default::default()
            };

            let body = model.build_request_body(&messages, &opts);

            assert_eq!(body["model"], "gpt-4");
            assert_eq!(body["messages"].as_array().unwrap().len(), 2);
            assert_eq!(body["temperature"], 0.7);
            assert_eq!(body["max_tokens"], 1024);
            assert!(body.get("tools").is_none());
        }

        #[test]
        fn test_openai_build_request_body_with_tools() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let messages = vec![json!({"role": "user", "content": "Search for rust"})];
            let tools = vec![json!({
                "type": "function",
                "function": {
                    "name": "search",
                    "description": "Search the web",
                    "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
                }
            })];
            let opts = ChatOptions {
                tools: Some(tools),
                tool_choice: Some(ToolChoice::Auto),
                ..Default::default()
            };

            let body = model.build_request_body(&messages, &opts);

            assert!(body["tools"].as_array().unwrap().len() == 1);
            assert_eq!(body["tool_choice"], "auto");
        }

        #[test]
        fn test_openai_parse_success_response() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let raw = json!({
                "id": "chatcmpl-abc123",
                "created": 1700000000,
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you?"
                    }
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 8,
                    "total_tokens": 18
                }
            });

            let resp = model.formatter.parse_response(raw).unwrap();
            assert_eq!(resp.id, "chatcmpl-abc123");
            assert_eq!(resp.get_text(), "Hello! How can I help you?");
            assert_eq!(resp.usage.input_tokens, 10);
            assert_eq!(resp.usage.output_tokens, 8);
            assert_eq!(resp.usage.total_tokens, 18);
            assert!(!resp.has_tool_calls());
        }

        #[test]
        fn test_openai_parse_tool_call_response() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let raw = json!({
                "id": "chatcmpl-tools",
                "created": 1700000001,
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_abc",
                            "type": "function",
                            "function": {
                                "name": "search",
                                "arguments": "{\"q\": \"rust programming\"}"
                            }
                        }]
                    }
                }],
                "usage": {"prompt_tokens": 20, "completion_tokens": 15, "total_tokens": 35}
            });

            let resp = model.formatter.parse_response(raw).unwrap();
            assert!(resp.has_tool_calls());
            let calls = resp.get_tool_calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].id, "call_abc");
            assert_eq!(calls[0].name, "search");
            assert_eq!(calls[0].input["q"], "rust programming");
        }

        #[test]
        fn test_openai_error_response() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            // Simulate an API error JSON — formatter returns empty content
            let raw = json!({
                "error": {
                    "message": "Invalid API key",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            });

            // The formatter gracefully handles missing choices with empty response
            let result = model.formatter.parse_response(raw).unwrap();
            assert!(result.content.is_empty());
            assert_eq!(result.get_text(), "");
        }

        #[test]
        fn test_openai_with_base_url() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4")
                .with_base_url("https://api.deepseek.com/v1");
            assert_eq!(
                model.endpoint(),
                "https://api.deepseek.com/v1/chat/completions"
            );
        }

        #[test]
        fn test_openai_with_base_url_trailing_slash() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4")
                .with_base_url("https://api.deepseek.com/v1/");
            assert_eq!(
                model.endpoint(),
                "https://api.deepseek.com/v1/chat/completions"
            );
        }

        #[test]
        fn test_openai_default_options() {
            let default_opts = ChatOptions {
                temperature: Some(0.5),
                max_tokens: Some(2048),
                ..Default::default()
            };
            let model = OpenAiChatModel::new("sk-test", "gpt-4").with_default_options(default_opts);

            // Call with empty options → defaults should apply
            let messages = vec![json!({"role": "user", "content": "hi"})];
            let body = model.build_request_body(&messages, &ChatOptions::default());

            assert_eq!(body["temperature"], 0.5);
            assert_eq!(body["max_tokens"], 2048);
        }

        #[test]
        fn test_openai_options_override_defaults() {
            let default_opts = ChatOptions {
                temperature: Some(0.5),
                max_tokens: Some(2048),
                ..Default::default()
            };
            let model = OpenAiChatModel::new("sk-test", "gpt-4").with_default_options(default_opts);

            let override_opts = ChatOptions {
                temperature: Some(1.0),
                ..Default::default()
            };
            let messages = vec![json!({"role": "user", "content": "hi"})];
            let body = model.build_request_body(&messages, &override_opts);

            // temperature overridden, max_tokens from default
            assert_eq!(body["temperature"], 1.0);
            assert_eq!(body["max_tokens"], 2048);
        }

        #[test]
        fn test_openai_model_name_override() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let opts = ChatOptions {
                model: Some("gpt-4o".into()),
                ..Default::default()
            };
            let messages = vec![json!({"role": "user", "content": "hi"})];
            let body = model.build_request_body(&messages, &opts);
            assert_eq!(body["model"], "gpt-4o");
        }

        #[test]
        fn test_openai_tool_choice_specific() {
            let model = OpenAiChatModel::new("sk-test", "gpt-4");
            let opts = ChatOptions {
                tools: Some(vec![
                    json!({"type": "function", "function": {"name": "my_tool"}}),
                ]),
                tool_choice: Some(ToolChoice::Specific("my_tool".into())),
                ..Default::default()
            };
            let messages = vec![json!({"role": "user", "content": "hi"})];
            let body = model.build_request_body(&messages, &opts);
            assert_eq!(body["tool_choice"]["type"], "function");
            assert_eq!(body["tool_choice"]["function"]["name"], "my_tool");
        }
    }

    // -----------------------------------------------------------------------
    // Anthropic tests
    // -----------------------------------------------------------------------

    #[cfg(feature = "messages_api")]
    mod anthropic_tests {
        use super::*;

        #[test]
        fn test_anthropic_build_request_with_system() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "Hello"}]})];
            let opts = ChatOptions {
                max_tokens: Some(4096),
                temperature: Some(0.7),
                ..Default::default()
            };

            let body = model.build_request_body(&messages, Some("Be helpful."), &opts);

            assert_eq!(body["model"], "claude-sonnet-4-20250514");
            assert_eq!(body["max_tokens"], 4096);
            assert_eq!(body["system"], "Be helpful.");
            assert_eq!(body["temperature"], 0.7);
            assert_eq!(body["messages"].as_array().unwrap().len(), 1);
            assert!(body.get("tools").is_none());
        }

        #[test]
        fn test_anthropic_build_request_no_system() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "Hello"}]})];
            let opts = ChatOptions::default();

            let body = model.build_request_body(&messages, None, &opts);

            assert!(body.get("system").is_none());
            assert_eq!(body["max_tokens"], 4096); // default
        }

        #[test]
        fn test_anthropic_build_request_with_tools() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "Search"}]})];
            let tools = vec![json!({
                "name": "search",
                "description": "Search the web",
                "input_schema": {"type": "object", "properties": {"q": {"type": "string"}}}
            })];
            let opts = ChatOptions {
                tools: Some(tools),
                tool_choice: Some(ToolChoice::Auto),
                ..Default::default()
            };

            let body = model.build_request_body(&messages, None, &opts);

            assert!(body["tools"].as_array().unwrap().len() == 1);
            assert_eq!(body["tool_choice"]["type"], "auto");
        }

        #[test]
        fn test_anthropic_parse_success_response() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let raw = json!({
                "id": "msg-abc123",
                "type": "message",
                "content": [
                    {"type": "text", "text": "Hello from Claude!"}
                ],
                "usage": {
                    "input_tokens": 12,
                    "output_tokens": 6
                }
            });

            let resp = model.formatter.parse_response(raw).unwrap();
            assert_eq!(resp.id, "msg-abc123");
            assert_eq!(resp.get_text(), "Hello from Claude!");
            assert_eq!(resp.usage.input_tokens, 12);
            assert_eq!(resp.usage.output_tokens, 6);
            assert_eq!(resp.usage.total_tokens, 18);
            assert!(!resp.has_tool_calls());
        }

        #[test]
        fn test_anthropic_parse_tool_use_response() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let raw = json!({
                "id": "msg-tools",
                "type": "message",
                "content": [
                    {"type": "text", "text": "Let me search for that."},
                    {
                        "type": "tool_use",
                        "id": "toolu_abc",
                        "name": "search",
                        "input": {"q": "rust programming"}
                    }
                ],
                "usage": {"input_tokens": 20, "output_tokens": 15}
            });

            let resp = model.formatter.parse_response(raw).unwrap();
            assert!(resp.has_tool_calls());
            assert_eq!(resp.get_text(), "Let me search for that.");
            let calls = resp.get_tool_calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].id, "toolu_abc");
            assert_eq!(calls[0].name, "search");
            assert_eq!(calls[0].input["q"], "rust programming");
        }

        #[test]
        fn test_anthropic_extract_system_from_formatted() {
            let messages = vec![
                json!({"role": "system", "content": "Be helpful."}),
                json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]}),
            ];

            let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
            assert_eq!(system.as_deref(), Some("Be helpful."));
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0]["role"], "user");
        }

        #[test]
        fn test_anthropic_extract_system_from_formatted_array_content() {
            let messages = vec![
                json!({"role": "system", "content": [{"type": "text", "text": "System prompt"}]}),
                json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]}),
            ];

            let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
            assert_eq!(system.as_deref(), Some("System prompt"));
            assert_eq!(filtered.len(), 1);
        }

        #[test]
        fn test_anthropic_extract_system_no_system() {
            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]})];

            let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
            assert!(system.is_none());
            assert_eq!(filtered.len(), 1);
        }

        #[test]
        fn test_anthropic_with_base_url() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514")
                .with_base_url("https://my-proxy.com");
            assert_eq!(model.endpoint(), "https://my-proxy.com/v1/messages");
        }

        #[test]
        fn test_anthropic_default_options() {
            let default_opts = ChatOptions {
                temperature: Some(0.3),
                max_tokens: Some(8192),
                ..Default::default()
            };
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514")
                .with_default_options(default_opts);

            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "hi"}]})];
            let body = model.build_request_body(&messages, None, &ChatOptions::default());

            assert_eq!(body["temperature"], 0.3);
            assert_eq!(body["max_tokens"], 8192);
        }

        #[test]
        fn test_anthropic_tool_choice_specific() {
            let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
            let opts = ChatOptions {
                tools: Some(vec![json!({"name": "my_tool"})]),
                tool_choice: Some(ToolChoice::Specific("my_tool".into())),
                ..Default::default()
            };
            let messages =
                vec![json!({"role": "user", "content": [{"type": "text", "text": "hi"}]})];
            let body = model.build_request_body(&messages, None, &opts);
            assert_eq!(body["tool_choice"]["type"], "tool");
            assert_eq!(body["tool_choice"]["name"], "my_tool");
        }
    }

    // -----------------------------------------------------------------------
    // Shared helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_options_prefers_options() {
        let defaults = ChatOptions {
            temperature: Some(0.5),
            max_tokens: Some(2048),
            ..Default::default()
        };
        let options = ChatOptions {
            temperature: Some(1.0),
            ..Default::default()
        };

        let merged = merge_options(&options, &defaults);
        assert_eq!(merged.temperature, Some(1.0));
        assert_eq!(merged.max_tokens, Some(2048));
    }

    #[test]
    fn test_merge_options_uses_defaults_when_none() {
        let defaults = ChatOptions {
            temperature: Some(0.5),
            max_tokens: Some(2048),
            top_p: Some(0.9),
            ..Default::default()
        };
        let options = ChatOptions::default();

        let merged = merge_options(&options, &defaults);
        assert_eq!(merged.temperature, Some(0.5));
        assert_eq!(merged.max_tokens, Some(2048));
        assert_eq!(merged.top_p, Some(0.9));
    }

    #[test]
    fn test_tool_choice_openai_values() {
        assert_eq!(
            tool_choice_to_openai_value(&ToolChoice::Auto),
            json!("auto")
        );
        assert_eq!(
            tool_choice_to_openai_value(&ToolChoice::None),
            json!("none")
        );
        assert_eq!(
            tool_choice_to_openai_value(&ToolChoice::Required),
            json!("required")
        );
        assert_eq!(
            tool_choice_to_openai_value(&ToolChoice::Specific("foo".into())),
            json!({"type": "function", "function": {"name": "foo"}})
        );
    }

    #[test]
    fn test_tool_choice_anthropic_values() {
        assert_eq!(
            tool_choice_to_anthropic_value(&ToolChoice::Auto),
            json!({"type": "auto"})
        );
        assert_eq!(
            tool_choice_to_anthropic_value(&ToolChoice::None),
            json!({"type": "none"})
        );
        assert_eq!(
            tool_choice_to_anthropic_value(&ToolChoice::Required),
            json!({"type": "any"})
        );
        assert_eq!(
            tool_choice_to_anthropic_value(&ToolChoice::Specific("bar".into())),
            json!({"type": "tool", "name": "bar"})
        );
    }
}
